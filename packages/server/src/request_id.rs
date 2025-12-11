//! Request ID generation and extraction for request correlation
//!
//! Provides unique identifiers for each request that flow through
//! the entire system (Rust server -> IPC -> TypeScript handlers).
//! This enables end-to-end request tracing in logs and metrics.

use uuid::Uuid;

/// Standard header name for request ID
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Generate a new UUID v4 request ID
#[inline]
pub fn generate() -> String {
    Uuid::new_v4().to_string()
}

/// Extract request ID from headers or generate a new one
///
/// Checks for both lowercase and mixed-case header names.
pub fn get_or_generate(headers: &std::collections::HashMap<String, String>) -> String {
    // Check lowercase first (normalized headers)
    if let Some(id) = headers.get(REQUEST_ID_HEADER) {
        if !id.is_empty() {
            return id.clone();
        }
    }

    // Check mixed-case variant
    if let Some(id) = headers.get("X-Request-ID") {
        if !id.is_empty() {
            return id.clone();
        }
    }

    // Check another common variant
    if let Some(id) = headers.get("X-Request-Id") {
        if !id.is_empty() {
            return id.clone();
        }
    }

    generate()
}

/// Validate that a request ID looks reasonable
/// (not empty, not too long, alphanumeric + hyphens)
pub fn is_valid(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }

    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique() {
        let id1 = generate();
        let id2 = generate();
        assert_ne!(id1, id2);
        assert!(is_valid(&id1));
        assert!(is_valid(&id2));
    }

    #[test]
    fn test_get_or_generate_extracts() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("x-request-id".to_string(), "test-123".to_string());

        let id = get_or_generate(&headers);
        assert_eq!(id, "test-123");
    }

    #[test]
    fn test_get_or_generate_creates() {
        let headers = std::collections::HashMap::new();
        let id = get_or_generate(&headers);
        assert!(is_valid(&id));
        // Should be UUID format
        assert!(id.contains('-'));
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid("abc-123"));
        assert!(is_valid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid("request_123"));
        assert!(!is_valid(""));
        assert!(!is_valid(&"a".repeat(200)));
        assert!(!is_valid("test@123")); // @ not allowed
    }
}
