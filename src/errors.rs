use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Command '{cmd}' failed with status {status:?}: {stderr}")]
    CommandFailure {
        cmd: String,
        status: Option<i32>,
        stderr: String,
    },
}
