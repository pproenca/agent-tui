//! Shared daemon RPC core used by Unix and WebSocket transports.

use crate::adapters::attach_output_to_response;
use crate::adapters::daemon::Router;
use crate::adapters::daemon::UseCaseContainer;
use crate::adapters::parse_attach_input;
use crate::adapters::parse_session_selector;
use crate::adapters::rpc::RpcRequest;
use crate::adapters::rpc::RpcResponse;
use crate::adapters::session_error_response;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use crate::infra::daemon::DaemonConfig;
use crate::infra::daemon::SessionManager;
use crate::infra::daemon::SystemClock;
use crate::usecases::AttachUseCase;
use crate::usecases::ports::SessionRepository;
use crate::usecases::ports::ShutdownNotifierHandle;
use crate::usecases::ports::StreamCursor;

const ATTACH_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const ATTACH_STREAM_MAX_TICK_BYTES: usize = 512 * 1024;
const ATTACH_STREAM_HEARTBEAT: Duration = Duration::from_secs(30);
const LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES: usize = 64 * 1024;
const LIVE_PREVIEW_STREAM_MAX_TICK_BYTES: usize = 256 * 1024;
const LIVE_PREVIEW_STREAM_HEARTBEAT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamKind {
    Attach,
    LivePreview,
}

#[derive(Debug)]
pub(crate) enum RpcCoreError {
    ConnectionClosed,
    Other(String),
}

impl fmt::Display for RpcCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionClosed => write!(f, "connection closed"),
            Self::Other(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RpcCoreError {}

pub(crate) trait RpcResponseWriter {
    fn write_response(&mut self, response: &RpcResponse) -> Result<(), RpcCoreError>;
}

pub(crate) struct RpcCore {
    session_manager: Arc<SessionManager>,
    usecases: UseCaseContainer<SessionManager>,
    shutdown_flag: Arc<AtomicBool>,
}

impl RpcCore {
    pub fn with_config(
        config: DaemonConfig,
        shutdown_flag: Arc<AtomicBool>,
        shutdown_notifier: ShutdownNotifierHandle,
    ) -> Self {
        let session_manager = Arc::new(SessionManager::with_max_sessions(config.max_sessions()));
        let clock = Arc::new(SystemClock::new());
        let usecases = UseCaseContainer::new(
            Arc::clone(&session_manager),
            clock,
            Arc::clone(&shutdown_flag),
            shutdown_notifier,
        );
        Self {
            session_manager,
            usecases,
            shutdown_flag,
        }
    }

    pub fn session_repository_handle(&self) -> Arc<dyn SessionRepository> {
        let repository: Arc<dyn SessionRepository> = self.session_manager.clone();
        repository
    }

    pub fn shutdown_all_sessions(&self) {
        let sessions = self.session_manager.list();
        for info in sessions {
            if let Err(err) = self.session_manager.kill(info.id.as_str()) {
                tracing::warn!(session_id = %info.id, error = %err, "Failed to kill session during shutdown");
            }
        }
    }

    pub fn route(&self, request: RpcRequest) -> RpcResponse {
        let router = Router::new(&self.usecases);
        router.route(request)
    }

    pub fn stream_kind_for_method(method: &str) -> Option<StreamKind> {
        match method {
            "attach_stream" => Some(StreamKind::Attach),
            "live_preview_stream" => Some(StreamKind::LivePreview),
            _ => None,
        }
    }

    pub fn handle_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
        kind: StreamKind,
    ) -> Result<(), RpcCoreError> {
        match kind {
            StreamKind::Attach => self.handle_attach_stream(writer, request),
            StreamKind::LivePreview => self.handle_live_preview_stream(writer, request),
        }
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::Relaxed)
    }

    fn handle_attach_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
    ) -> Result<(), RpcCoreError> {
        let req_id = request.id;
        let input = match parse_attach_input(&request) {
            Ok(input) => input,
            Err(response) => {
                let _ = writer.write_response(&response);
                return Ok(());
            }
        };

        let session_id = input.session_id.clone();
        match self.usecases.session.attach.execute(input) {
            Ok(output) => {
                let response = attach_output_to_response(req_id, output);
                writer.write_response(&response)?;
            }
            Err(err) => {
                let response = session_error_response(req_id, err);
                let _ = writer.write_response(&response);
                return Ok(());
            }
        }

        let session =
            match SessionRepository::resolve(self.session_manager.as_ref(), Some(&session_id)) {
                Ok(session) => session,
                Err(err) => {
                    let response = session_error_response(req_id, err);
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = writer.write_response(&response);
            return Ok(());
        }

        #[derive(Serialize)]
        struct AttachReady<'a> {
            event: &'static str,
            session_id: &'a str,
        }

        #[derive(Serialize)]
        struct AttachDropped {
            event: &'static str,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct AttachOutput<'a> {
            event: &'static str,
            data: &'a str,
            bytes: usize,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct AttachEvent {
            event: &'static str,
        }

        let ready = RpcResponse::success_json(
            req_id,
            &AttachReady {
                event: "ready",
                session_id: session_id.as_str(),
            },
        );
        writer.write_response(&ready)?;

        let stream_seq = session.live_preview_snapshot().stream_seq;
        let subscription = session.stream_subscribe();
        let mut cursor = StreamCursor { seq: stream_seq };

        loop {
            if self.should_shutdown() {
                let response = RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                let _ = writer.write_response(&response);
                return Ok(());
            }
            let mut budget = ATTACH_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(ATTACH_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                };

                if read.dropped_bytes > 0 && read.data.is_empty() {
                    let response = RpcResponse::success_json(
                        req_id,
                        &AttachDropped {
                            event: "dropped",
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    writer.write_response(&response)?;
                    sent_any = true;
                }

                if !read.data.is_empty() {
                    let data_b64 = STANDARD.encode(&read.data);
                    let response = RpcResponse::success_json(
                        req_id,
                        &AttachOutput {
                            event: "output",
                            data: &data_b64,
                            bytes: read.data.len(),
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    writer.write_response(&response)?;
                    sent_any = true;
                    budget = budget.saturating_sub(read.data.len());
                    if read.closed {
                        let response =
                            RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                    continue;
                }

                if read.closed {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = writer.write_response(&response);
                    return Ok(());
                }

                break;
            }

            if sent_any && budget == 0 {
                continue;
            }

            if !subscription.wait(Some(ATTACH_STREAM_HEARTBEAT)) {
                if self.should_shutdown() {
                    let response =
                        RpcResponse::success_json(req_id, &AttachEvent { event: "closed" });
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
                let response =
                    RpcResponse::success_json(req_id, &AttachEvent { event: "heartbeat" });
                writer.write_response(&response)?;
            }
        }
    }

    fn handle_live_preview_stream(
        &self,
        writer: &mut impl RpcResponseWriter,
        request: RpcRequest,
    ) -> Result<(), RpcCoreError> {
        let req_id = request.id;
        let session_param = parse_live_preview_session_selector(&request);

        let session =
            match SessionRepository::resolve(self.session_manager.as_ref(), session_param.as_ref())
            {
                Ok(session) => session,
                Err(err) => {
                    let response = session_error_response(req_id, err);
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
            };

        if let Err(err) = session.update() {
            let response = session_error_response(req_id, err);
            let _ = writer.write_response(&response);
            return Ok(());
        }

        let snapshot = session.live_preview_snapshot();
        let session_id = session.session_id().to_string();
        #[derive(Serialize)]
        struct LivePreviewReady<'a> {
            event: &'static str,
            session_id: &'a str,
            cols: u16,
            rows: u16,
        }

        #[derive(Serialize)]
        struct LivePreviewInit<'a> {
            event: &'static str,
            time: f64,
            cols: u16,
            rows: u16,
            init: &'a str,
        }

        #[derive(Serialize)]
        struct LivePreviewDropped {
            event: &'static str,
            time: f64,
            dropped_bytes: u64,
        }

        #[derive(Serialize)]
        struct LivePreviewOutput<'a> {
            event: &'static str,
            time: f64,
            data_b64: &'a str,
        }

        #[derive(Serialize)]
        struct LivePreviewClosed {
            event: &'static str,
            time: f64,
        }

        #[derive(Serialize)]
        struct LivePreviewResize {
            event: &'static str,
            time: f64,
            cols: u16,
            rows: u16,
        }

        #[derive(Serialize)]
        struct LivePreviewHeartbeat {
            event: &'static str,
            time: f64,
        }

        let ready = RpcResponse::success_json(
            req_id,
            &LivePreviewReady {
                event: "ready",
                session_id: &session_id,
                cols: snapshot.cols,
                rows: snapshot.rows,
            },
        );
        writer.write_response(&ready)?;

        let start_time = Instant::now();
        let init = RpcResponse::success_json(
            req_id,
            &LivePreviewInit {
                event: "init",
                time: start_time.elapsed().as_secs_f64(),
                cols: snapshot.cols,
                rows: snapshot.rows,
                init: &snapshot.seq,
            },
        );
        writer.write_response(&init)?;

        let subscription = session.stream_subscribe();
        let mut cursor = StreamCursor::default();
        let mut last_size = (snapshot.cols, snapshot.rows);

        loop {
            if self.should_shutdown() {
                let response = RpcResponse::success_json(
                    req_id,
                    &LivePreviewClosed {
                        event: "closed",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                );
                let _ = writer.write_response(&response);
                return Ok(());
            }
            let mut budget = LIVE_PREVIEW_STREAM_MAX_TICK_BYTES;
            let mut sent_any = false;

            loop {
                if budget == 0 {
                    break;
                }

                let max_chunk = budget.min(LIVE_PREVIEW_STREAM_MAX_CHUNK_BYTES);
                let read = match session.stream_read(&mut cursor, max_chunk, 0) {
                    Ok(read) => read,
                    Err(err) => {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                };

                if read.dropped_bytes > 0 {
                    let dropped = RpcResponse::success_json(
                        req_id,
                        &LivePreviewDropped {
                            event: "dropped",
                            time: start_time.elapsed().as_secs_f64(),
                            dropped_bytes: read.dropped_bytes,
                        },
                    );
                    writer.write_response(&dropped)?;
                    if let Err(err) = session.update() {
                        let response = session_error_response(req_id, err);
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                    let snapshot = session.live_preview_snapshot();
                    let init = RpcResponse::success_json(
                        req_id,
                        &LivePreviewInit {
                            event: "init",
                            time: start_time.elapsed().as_secs_f64(),
                            cols: snapshot.cols,
                            rows: snapshot.rows,
                            init: &snapshot.seq,
                        },
                    );
                    writer.write_response(&init)?;
                    last_size = (snapshot.cols, snapshot.rows);
                    cursor.seq = read.latest_cursor.seq;
                    sent_any = true;
                    break;
                }

                if !read.data.is_empty() {
                    let data_b64 = STANDARD.encode(&read.data);
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewOutput {
                            event: "output",
                            time: start_time.elapsed().as_secs_f64(),
                            data_b64: &data_b64,
                        },
                    );
                    writer.write_response(&response)?;
                    sent_any = true;
                    budget = budget.saturating_sub(read.data.len());
                    if read.closed {
                        let response = RpcResponse::success_json(
                            req_id,
                            &LivePreviewClosed {
                                event: "closed",
                                time: start_time.elapsed().as_secs_f64(),
                            },
                        );
                        let _ = writer.write_response(&response);
                        return Ok(());
                    }
                    continue;
                }

                if read.closed {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = writer.write_response(&response);
                    return Ok(());
                }

                break;
            }

            let size = session.size();
            if size != last_size {
                let resize = RpcResponse::success_json(
                    req_id,
                    &LivePreviewResize {
                        event: "resize",
                        time: start_time.elapsed().as_secs_f64(),
                        cols: size.0,
                        rows: size.1,
                    },
                );
                writer.write_response(&resize)?;
                last_size = size;
                sent_any = true;
            }

            if sent_any && budget == 0 {
                continue;
            }

            if !subscription.wait(Some(LIVE_PREVIEW_STREAM_HEARTBEAT)) {
                if self.should_shutdown() {
                    let response = RpcResponse::success_json(
                        req_id,
                        &LivePreviewClosed {
                            event: "closed",
                            time: start_time.elapsed().as_secs_f64(),
                        },
                    );
                    let _ = writer.write_response(&response);
                    return Ok(());
                }
                let response = RpcResponse::success_json(
                    req_id,
                    &LivePreviewHeartbeat {
                        event: "heartbeat",
                        time: start_time.elapsed().as_secs_f64(),
                    },
                );
                writer.write_response(&response)?;
            }
        }
    }
}

fn parse_live_preview_session_selector(request: &RpcRequest) -> Option<crate::domain::SessionId> {
    parse_session_selector(request.param_str("session").map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_request(params: Option<serde_json::Value>) -> RpcRequest {
        RpcRequest::new(1, "live_preview_stream".to_string(), params)
    }

    #[test]
    fn live_preview_selector_maps_active_to_none() {
        let request = make_request(Some(json!({ "session": "active" })));
        let parsed = parse_live_preview_session_selector(&request);
        assert!(parsed.is_none());
    }

    #[test]
    fn live_preview_selector_defaults_to_none_when_omitted() {
        let request = make_request(None);
        let parsed = parse_live_preview_session_selector(&request);
        assert!(parsed.is_none());
    }

    #[test]
    fn live_preview_selector_keeps_explicit_session_id() {
        let request = make_request(Some(json!({ "session": "sess-1" })));
        let parsed = parse_live_preview_session_selector(&request).expect("session id");
        assert_eq!(parsed.as_str(), "sess-1");
    }
}
