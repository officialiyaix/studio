use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Errors returned to the frontend as `{ code, message }`. Messages never contain credentials.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{message}")]
    Coded { code: &'static str, message: String },
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Network error: {0}")]
    Http(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // Strip URLs (which could carry query parameters) from network errors before they reach logs or the UI.
        AppError::Http(e.without_url().to_string())
    }
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        AppError::Coded { code, message: message.into() }
    }

    pub fn code(&self) -> &'static str {
        match self {
            AppError::Coded { code, .. } => code,
            AppError::Io(_) => "IO",
            AppError::Json(_) => "DATA",
            AppError::Http(_) => "NETWORK",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[macro_export]
macro_rules! bail {
    ($code:expr, $($arg:tt)*) => {
        return Err($crate::error::AppError::new($code, format!($($arg)*)))
    };
}
