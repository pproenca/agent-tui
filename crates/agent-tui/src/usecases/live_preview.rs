use std::net::SocketAddr;
use std::sync::Arc;

use crate::domain::{
    LivePreviewStartInput, LivePreviewStartOutput, LivePreviewStatusOutput, LivePreviewStopOutput,
};
use crate::usecases::ports::{
    LivePreviewError, LivePreviewOptions, LivePreviewService, SessionRepository,
};

pub trait LivePreviewStartUseCase: Send + Sync {
    fn execute(
        &self,
        input: LivePreviewStartInput,
    ) -> Result<LivePreviewStartOutput, LivePreviewError>;
}

pub trait LivePreviewStopUseCase: Send + Sync {
    fn execute(&self) -> Result<LivePreviewStopOutput, LivePreviewError>;
}

pub trait LivePreviewStatusUseCase: Send + Sync {
    fn execute(&self) -> LivePreviewStatusOutput;
}

pub struct LivePreviewStartUseCaseImpl<R: SessionRepository> {
    repository: Arc<R>,
    service: Arc<dyn LivePreviewService>,
}

impl<R: SessionRepository> LivePreviewStartUseCaseImpl<R> {
    pub fn new(repository: Arc<R>, service: Arc<dyn LivePreviewService>) -> Self {
        Self {
            repository,
            service,
        }
    }
}

impl<R: SessionRepository> LivePreviewStartUseCase for LivePreviewStartUseCaseImpl<R> {
    fn execute(
        &self,
        input: LivePreviewStartInput,
    ) -> Result<LivePreviewStartOutput, LivePreviewError> {
        let session = self.repository.resolve(input.session_id.as_deref())?;
        if !session.is_running() {
            let session_id = session.session_id().to_string();
            return Err(LivePreviewError::Session(
                crate::usecases::ports::SessionError::NotFound(format!(
                    "{} (session not running)",
                    session_id
                )),
            ));
        }

        let listen_addr = match input.listen_addr {
            Some(addr) => parse_listen_addr(&addr)?,
            None => default_listen_addr(),
        };
        if !input.allow_remote && !listen_addr.ip().is_loopback() {
            return Err(LivePreviewError::InvalidListenAddress(
                listen_addr.to_string(),
            ));
        }

        self.service
            .start(session, LivePreviewOptions { listen_addr })
    }
}

pub struct LivePreviewStopUseCaseImpl {
    service: Arc<dyn LivePreviewService>,
}

impl LivePreviewStopUseCaseImpl {
    pub fn new(service: Arc<dyn LivePreviewService>) -> Self {
        Self { service }
    }
}

impl LivePreviewStopUseCase for LivePreviewStopUseCaseImpl {
    fn execute(&self) -> Result<LivePreviewStopOutput, LivePreviewError> {
        self.service.stop()
    }
}

pub struct LivePreviewStatusUseCaseImpl {
    service: Arc<dyn LivePreviewService>,
}

impl LivePreviewStatusUseCaseImpl {
    pub fn new(service: Arc<dyn LivePreviewService>) -> Self {
        Self { service }
    }
}

impl LivePreviewStatusUseCase for LivePreviewStatusUseCaseImpl {
    fn execute(&self) -> LivePreviewStatusOutput {
        self.service.status()
    }
}

fn default_listen_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 0))
}

fn parse_listen_addr(addr: &str) -> Result<SocketAddr, LivePreviewError> {
    addr.parse::<SocketAddr>()
        .map_err(|_| LivePreviewError::InvalidListenAddress(addr.to_string()))
}
