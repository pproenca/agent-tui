use std::net::SocketAddr;

use crate::domain::{LivePreviewStartOutput, LivePreviewStatusOutput, LivePreviewStopOutput};

use super::errors::LivePreviewError;
use super::session_repository::SessionHandle;

#[derive(Debug, Clone, Copy)]
pub struct LivePreviewOptions {
    pub listen_addr: SocketAddr,
}

pub trait LivePreviewService: Send + Sync {
    fn start(
        &self,
        session: SessionHandle,
        options: LivePreviewOptions,
    ) -> Result<LivePreviewStartOutput, LivePreviewError>;
    fn stop(&self) -> Result<LivePreviewStopOutput, LivePreviewError>;
    fn status(&self) -> LivePreviewStatusOutput;
}
