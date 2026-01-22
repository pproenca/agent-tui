use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(#[from] std::io::Error),

    #[error("Failed to serialize request: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("RPC error ({code}): {message}")]
    RpcError { code: i32, message: String },

    #[error("Daemon not running")]
    DaemonNotRunning,

    #[error("Invalid response from daemon")]
    InvalidResponse,
}
