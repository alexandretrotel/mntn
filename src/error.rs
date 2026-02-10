use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Inquire(#[from] inquire::error::InquireError),
    #[error("Command '{cmd}' failed with status {status:?}: {stderr}")]
    CommandFailure {
        cmd: String,
        status: Option<i32>,
        stderr: String,
    },
    #[error("Registry entry '{id}' not found")]
    RegistryEntryNotFound { id: String },
}
