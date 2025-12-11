//! Comprehensive error handling with proper context and recovery
//!
//! Type-safe error handling throughout the application with proper
//! error propagation and context preservation.
//!
//! ## Error Structure
//! Each error has:
//! - A machine-readable **code** (e.g., "HANDLER_ERROR")
//! - A human-readable **message**
//! - An HTTP **status code**
//! - A unique **digest** for log correlation
//! - Optional **details** for additional context

use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;
use uuid::Uuid;

/// Zap error type covering all possible failure modes
#[derive(Debug, Error)]
pub enum ZapError {
    /// HTTP server errors
    #[error("HTTP error: {message}")]
    Http {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Route not found (404)
    #[error("Route not found: {path}")]
    RouteNotFound { path: String },

    /// Handler execution errors
    #[error("Handler error: {message}")]
    Handler {
        message: String,
        handler_id: Option<String>,
    },

    /// IPC/Socket errors
    #[error("IPC error: {message}")]
    Ipc { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Validation errors (400)
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
    },

    /// Authentication required (401)
    #[error("Authentication required: {message}")]
    Unauthorized { message: String },

    /// Access forbidden (403)
    #[error("Access forbidden: {message}")]
    Forbidden { message: String },

    /// Timeout errors (408/504)
    #[error("Timeout: {message}")]
    Timeout { message: String, timeout_ms: u64 },

    /// Rate limit exceeded (429)
    #[error("Rate limit exceeded")]
    RateLimited { retry_after_secs: u64 },

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Internal error (500)
    #[error("Internal error: {0}")]
    Internal(String),

    /// WebSocket errors
    #[error("WebSocket error: {message}")]
    WebSocket { message: String },
}

impl ZapError {
    /// Get the machine-readable error code
    pub fn code(&self) -> &'static str {
        match self {
            ZapError::Http { .. } => "HTTP_ERROR",
            ZapError::RouteNotFound { .. } => "ROUTE_NOT_FOUND",
            ZapError::Handler { .. } => "HANDLER_ERROR",
            ZapError::Ipc { .. } => "IPC_ERROR",
            ZapError::Config { .. } => "CONFIG_ERROR",
            ZapError::Io(_) => "IO_ERROR",
            ZapError::Serialization(_) => "SERIALIZATION_ERROR",
            ZapError::Validation { .. } => "VALIDATION_ERROR",
            ZapError::Unauthorized { .. } => "UNAUTHORIZED",
            ZapError::Forbidden { .. } => "FORBIDDEN",
            ZapError::Timeout { .. } => "TIMEOUT",
            ZapError::RateLimited { .. } => "RATE_LIMITED",
            ZapError::InvalidState(_) => "INVALID_STATE",
            ZapError::Internal(_) => "INTERNAL_ERROR",
            ZapError::WebSocket { .. } => "WEBSOCKET_ERROR",
        }
    }

    /// Get the appropriate HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            ZapError::Http { .. } => 500,
            ZapError::RouteNotFound { .. } => 404,
            ZapError::Handler { .. } => 500,
            ZapError::Ipc { .. } => 502,
            ZapError::Config { .. } => 500,
            ZapError::Io(_) => 500,
            ZapError::Serialization(_) => 400,
            ZapError::Validation { .. } => 400,
            ZapError::Unauthorized { .. } => 401,
            ZapError::Forbidden { .. } => 403,
            ZapError::Timeout { .. } => 504,
            ZapError::RateLimited { .. } => 429,
            ZapError::InvalidState(_) => 500,
            ZapError::Internal(_) => 500,
            ZapError::WebSocket { .. } => 500,
        }
    }

    /// Convert to a structured error response
    pub fn to_error_response(&self) -> ErrorResponse {
        let digest = Uuid::new_v4().to_string();

        ErrorResponse {
            error: true,
            code: self.code().to_string(),
            message: self.to_string(),
            status: self.status_code(),
            digest,
            details: self.details(),
        }
    }

    /// Get additional error-specific details
    fn details(&self) -> Option<serde_json::Value> {
        match self {
            ZapError::Validation { field, .. } => {
                field.as_ref().map(|f| serde_json::json!({ "field": f }))
            }
            ZapError::RateLimited { retry_after_secs } => {
                Some(serde_json::json!({ "retryAfter": retry_after_secs }))
            }
            ZapError::Timeout { timeout_ms, .. } => {
                Some(serde_json::json!({ "timeoutMs": timeout_ms }))
            }
            ZapError::RouteNotFound { path } => Some(serde_json::json!({ "path": path })),
            ZapError::Handler { handler_id, .. } => {
                handler_id.as_ref().map(|id| serde_json::json!({ "handlerId": id }))
            }
            _ => None,
        }
    }

    // Convenience constructors

    /// Create an HTTP error
    pub fn http(message: impl Into<String>) -> Self {
        ZapError::Http {
            message: message.into(),
            source: None,
        }
    }

    /// Create a route not found error
    pub fn route_not_found(path: impl Into<String>) -> Self {
        ZapError::RouteNotFound { path: path.into() }
    }

    /// Create a handler error
    pub fn handler(message: impl Into<String>) -> Self {
        ZapError::Handler {
            message: message.into(),
            handler_id: None,
        }
    }

    /// Create a handler error with handler ID
    pub fn handler_with_id(message: impl Into<String>, handler_id: impl Into<String>) -> Self {
        ZapError::Handler {
            message: message.into(),
            handler_id: Some(handler_id.into()),
        }
    }

    /// Create an IPC error
    pub fn ipc(message: impl Into<String>) -> Self {
        ZapError::Ipc {
            message: message.into(),
        }
    }

    /// Create a config error
    pub fn config(message: impl Into<String>) -> Self {
        ZapError::Config {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        ZapError::Validation {
            message: message.into(),
            field: None,
        }
    }

    /// Create a validation error with field name
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        ZapError::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Create an unauthorized error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        ZapError::Unauthorized {
            message: message.into(),
        }
    }

    /// Create a forbidden error
    pub fn forbidden(message: impl Into<String>) -> Self {
        ZapError::Forbidden {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>, timeout_ms: u64) -> Self {
        ZapError::Timeout {
            message: message.into(),
            timeout_ms,
        }
    }

    /// Create a rate limited error
    pub fn rate_limited(retry_after_secs: u64) -> Self {
        ZapError::RateLimited { retry_after_secs }
    }

    /// Create a WebSocket error
    pub fn websocket(message: impl Into<String>) -> Self {
        ZapError::WebSocket {
            message: message.into(),
        }
    }
}

impl From<String> for ZapError {
    fn from(msg: String) -> Self {
        Self::Internal(msg)
    }
}

impl From<&str> for ZapError {
    fn from(msg: &str) -> Self {
        Self::Internal(msg.to_string())
    }
}

/// Structured error response for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Always true for error responses
    pub error: bool,

    /// Machine-readable error code (e.g., "HANDLER_ERROR")
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// HTTP status code
    pub status: u16,

    /// Unique error identifier for log correlation
    pub digest: String,

    /// Additional error-specific details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(code: impl Into<String>, message: impl Into<String>, status: u16) -> Self {
        Self {
            error: true,
            code: code.into(),
            message: message.into(),
            status,
            digest: Uuid::new_v4().to_string(),
            details: None,
        }
    }

    /// Add details to the error response
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"error":true,"code":"{}","message":"{}","status":{},"digest":"{}"}}"#,
                self.code, self.message, self.status, self.digest
            )
        })
    }
}

/// Convenient Result type for Zap operations
pub type ZapResult<T> = Result<T, ZapError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ZapError::route_not_found("/test").code(), "ROUTE_NOT_FOUND");
        assert_eq!(ZapError::handler("test").code(), "HANDLER_ERROR");
        assert_eq!(ZapError::validation("test").code(), "VALIDATION_ERROR");
        assert_eq!(ZapError::rate_limited(60).code(), "RATE_LIMITED");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(ZapError::route_not_found("/test").status_code(), 404);
        assert_eq!(ZapError::validation("test").status_code(), 400);
        assert_eq!(ZapError::unauthorized("test").status_code(), 401);
        assert_eq!(ZapError::forbidden("test").status_code(), 403);
        assert_eq!(ZapError::rate_limited(60).status_code(), 429);
        assert_eq!(ZapError::timeout("test", 5000).status_code(), 504);
    }

    #[test]
    fn test_error_response() {
        let error = ZapError::validation_field("Invalid email", "email");
        let response = error.to_error_response();

        assert!(response.error);
        assert_eq!(response.code, "VALIDATION_ERROR");
        assert_eq!(response.status, 400);
        assert!(!response.digest.is_empty());
        assert!(response.details.is_some());

        let details = response.details.unwrap();
        assert_eq!(details["field"], "email");
    }

    #[test]
    fn test_error_response_json() {
        let response = ErrorResponse::new("TEST_ERROR", "Test message", 500);
        let json = response.to_json();

        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
        assert!(json.contains("500"));
    }
} 