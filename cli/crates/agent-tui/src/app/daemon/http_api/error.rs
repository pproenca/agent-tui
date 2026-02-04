use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ApiServerError {
    #[error("API disabled")]
    Disabled,
    #[error("Invalid listen address: {message}")]
    InvalidListen { message: String },
    #[error("API server I/O error ({operation}): {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
}
