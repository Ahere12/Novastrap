//! Application-wide error type. Serialized to the UI as `{ kind, message, retryable }`.

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("network request failed: {0}")]
    Network(String),
    #[error("Roblox returned HTTP {0}")]
    HttpStatus(u16),
    #[error("Roblox's response did not match the expected format: {0}")]
    UpstreamChanged(String),
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Unsupported(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Network(_) => "network",
            AppError::HttpStatus(_) => "http_status",
            AppError::UpstreamChanged(_) => "upstream_changed",
            AppError::Io(_) => "io",
            AppError::Unsupported(_) => "unsupported",
        }
    }

    /// Whether retrying the same operation later might succeed.
    pub fn retryable(&self) -> bool {
        match self {
            AppError::Network(_) => true,
            AppError::HttpStatus(code) => *code == 429 || *code >= 500,
            _ => false,
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            AppError::UpstreamChanged(e.to_string())
        } else if let Some(status) = e.status() {
            AppError::HttpStatus(status.as_u16())
        } else {
            // `without_url` keeps query strings (which could carry identifiers) out of logs/UI.
            AppError::Network(e.without_url().to_string())
        }
    }
}

/// The UI-facing, cloneable snapshot of an [`AppError`].
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ErrorInfo {
    pub kind: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl From<&AppError> for ErrorInfo {
    fn from(e: &AppError) -> Self {
        ErrorInfo { kind: e.kind(), message: e.to_string(), retryable: e.retryable() }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        ErrorInfo::from(self).serialize(s)
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_classification() {
        assert!(AppError::Network("x".into()).retryable());
        assert!(AppError::HttpStatus(429).retryable());
        assert!(AppError::HttpStatus(503).retryable());
        assert!(!AppError::HttpStatus(404).retryable());
        assert!(!AppError::UpstreamChanged("x".into()).retryable());
    }

    #[test]
    fn serializes_for_ui() {
        let v = serde_json::to_value(AppError::HttpStatus(429)).unwrap();
        assert_eq!(v["kind"], "http_status");
        assert_eq!(v["retryable"], true);
        assert_eq!(v["message"], "Roblox returned HTTP 429");
    }
}
