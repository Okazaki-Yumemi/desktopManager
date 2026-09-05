use serde::Serialize;

/// Structured application error.
///
/// Every error crossing the IPC boundary is serialized as a human-readable
/// string so the frontend can show a toast / inline error. Production code
/// must not panic on recoverable failures — return this instead.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("internal error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("{0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
