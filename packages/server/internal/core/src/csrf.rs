//! CSRF (Cross-Site Request Forgery) Protection Middleware
//!
//! Provides token-based CSRF protection for state-changing requests.
//!
//! ## How it works
//! 1. Generates a random CSRF token per session
//! 2. Stores token in a secure, HTTP-only, SameSite cookie
//! 3. Validates token on POST, PUT, DELETE, PATCH requests
//! 4. Tokens must be sent in X-CSRF-Token header or _csrf form field
//!
//! ## Security Features
//! - Cryptographically secure random token generation (32 bytes)
//! - Constant-time token comparison (prevents timing attacks)
//! - SameSite=Strict cookie (prevents CSRF from cross-site requests)
//! - HTTP-only cookie (prevents XSS token theft)
//! - Secure flag for HTTPS (prevents MITM token theft)
//! - Configurable token lifetime

use crate::middleware::{Context, Middleware, MiddlewareFuture, MiddlewareError, MiddlewareResult};
use crate::method::Method;
use rand::Rng;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::time::{SystemTime, UNIX_EPOCH};

/// CSRF protection configuration
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    /// Cookie name for CSRF token (default: "csrf_token")
    pub cookie_name: String,
    /// Header name for CSRF token (default: "X-CSRF-Token")
    pub header_name: String,
    /// Form field name for CSRF token (default: "_csrf")
    pub form_field_name: String,
    /// Token lifetime in seconds (default: 86400 = 24 hours)
    pub token_lifetime: u64,
    /// Cookie path (default: "/")
    pub cookie_path: String,
    /// Cookie domain (default: None = current domain)
    pub cookie_domain: Option<String>,
    /// Use Secure flag on cookie (default: true for production)
    pub secure: bool,
    /// SameSite policy (default: Strict)
    pub same_site: SameSitePolicy,
    /// Skip CSRF validation for specific paths (e.g., webhooks)
    pub skip_paths: Vec<String>,
}

/// SameSite cookie policy
#[derive(Debug, Clone, Copy)]
pub enum SameSitePolicy {
    /// Strict: Cookie only sent for same-site requests
    Strict,
    /// Lax: Cookie sent for top-level navigation
    Lax,
    /// None: Cookie sent for all requests (requires Secure flag)
    None,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            cookie_name: "csrf_token".to_string(),
            header_name: "X-CSRF-Token".to_string(),
            form_field_name: "_csrf".to_string(),
            token_lifetime: 86400, // 24 hours
            cookie_path: "/".to_string(),
            cookie_domain: None,
            secure: true,
            same_site: SameSitePolicy::Strict,
            skip_paths: Vec::new(),
        }
    }
}

impl CsrfConfig {
    /// Create config for development (Secure=false, SameSite=Lax)
    pub fn development() -> Self {
        Self {
            secure: false,
            same_site: SameSitePolicy::Lax,
            ..Default::default()
        }
    }

    /// Create config for production (Secure=true, SameSite=Strict)
    pub fn production() -> Self {
        Self::default()
    }

    /// Builder: Set cookie name
    pub fn cookie_name(mut self, name: impl Into<String>) -> Self {
        self.cookie_name = name.into();
        self
    }

    /// Builder: Set header name
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }

    /// Builder: Set token lifetime
    pub fn token_lifetime(mut self, seconds: u64) -> Self {
        self.token_lifetime = seconds;
        self
    }

    /// Builder: Set cookie domain
    pub fn cookie_domain(mut self, domain: impl Into<String>) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }

    /// Builder: Skip CSRF validation for specific paths
    pub fn skip_paths(mut self, paths: Vec<String>) -> Self {
        self.skip_paths = paths;
        self
    }
}

/// CSRF protection middleware
pub struct CsrfMiddleware {
    config: CsrfConfig,
}

impl CsrfMiddleware {
    /// Create CSRF middleware with default production config
    pub fn new() -> Self {
        Self {
            config: CsrfConfig::production(),
        }
    }

    /// Create CSRF middleware with custom config
    pub fn with_config(config: CsrfConfig) -> Self {
        Self { config }
    }

    /// Create CSRF middleware for development
    pub fn development() -> Self {
        Self {
            config: CsrfConfig::development(),
        }
    }

    /// Generate a cryptographically secure CSRF token
    fn generate_token() -> String {
        let mut rng = rand::thread_rng();
        let token_bytes: [u8; 32] = rng.gen();
        URL_SAFE_NO_PAD.encode(token_bytes)
    }

    /// Constant-time token comparison (prevents timing attacks)
    fn tokens_equal(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
            result |= byte_a ^ byte_b;
        }
        result == 0
    }

    /// Extract CSRF token from cookie
    fn extract_cookie_token<'a>(&self, ctx: &Context<'a>) -> Option<&'a str> {
        ctx.headers().get("Cookie").and_then(|cookie_header| {
            cookie_header
                .split(';')
                .map(|s| s.trim())
                .find_map(|cookie| {
                    let mut parts = cookie.splitn(2, '=');
                    let name = parts.next()?;
                    let value = parts.next()?;
                    if name == self.config.cookie_name {
                        Some(value)
                    } else {
                        None
                    }
                })
        })
    }

    /// Extract CSRF token from request (header or form field)
    fn extract_request_token<'a>(&self, ctx: &Context<'a>) -> Option<String> {
        // First try header
        if let Some(token) = ctx.headers().get(&self.config.header_name) {
            return Some(token.to_string());
        }

        // Then try form field (URL-encoded body)
        if let Ok(body_str) = ctx.body_string() {
            let field_prefix = format!("{}=", self.config.form_field_name);
            if let Some(token) = body_str
                .split('&')
                .find(|field| field.starts_with(&field_prefix))
            {
                return Some(token[field_prefix.len()..].to_string());
            }
        }

        None
    }

    /// Check if path should skip CSRF validation
    fn should_skip_path(&self, path: &str) -> bool {
        self.config.skip_paths.iter().any(|skip_path| {
            if skip_path.ends_with('*') {
                path.starts_with(&skip_path[..skip_path.len() - 1])
            } else {
                path == skip_path
            }
        })
    }

    /// Build Set-Cookie header value
    fn build_cookie_header(&self, token: &str) -> String {
        let mut cookie = format!("{}={}; Path={}", self.config.cookie_name, token, self.config.cookie_path);

        if let Some(ref domain) = self.config.cookie_domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }

        // Calculate expiration
        let max_age = self.config.token_lifetime;
        cookie.push_str(&format!("; Max-Age={}", max_age));

        // Security flags
        cookie.push_str("; HttpOnly");

        if self.config.secure {
            cookie.push_str("; Secure");
        }

        match self.config.same_site {
            SameSitePolicy::Strict => cookie.push_str("; SameSite=Strict"),
            SameSitePolicy::Lax => cookie.push_str("; SameSite=Lax"),
            SameSitePolicy::None => {
                cookie.push_str("; SameSite=None");
                if !self.config.secure {
                    eprintln!("WARNING: SameSite=None requires Secure flag. Setting Secure=true.");
                    cookie.push_str("; Secure");
                }
            }
        }

        cookie
    }

    /// Validate CSRF token for state-changing requests
    fn validate_token<'a>(&self, ctx: &Context<'a>) -> Result<(), MiddlewareError> {
        let cookie_token = self.extract_cookie_token(ctx).ok_or_else(|| {
            MiddlewareError::Unauthorized("CSRF token missing from cookie".to_string())
        })?;

        let request_token = self.extract_request_token(ctx).ok_or_else(|| {
            MiddlewareError::Unauthorized(format!(
                "CSRF token missing. Include token in {} header or {} form field",
                self.config.header_name, self.config.form_field_name
            ))
        })?;

        if !Self::tokens_equal(cookie_token, &request_token) {
            return Err(MiddlewareError::Unauthorized(
                "CSRF token mismatch".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CsrfMiddleware {
    fn call<'a>(&'a self, ctx: Context<'a>) -> MiddlewareFuture<'a> {
        Box::pin(async move {
            let method = ctx.method();
            let path = ctx.path();

            // Skip CSRF validation for safe methods (GET, HEAD, OPTIONS)
            if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
                // Generate token if not present (for initial page load)
                if self.extract_cookie_token(&ctx).is_none() {
                    let token = Self::generate_token();
                    let cookie_header = self.build_cookie_header(&token);

                    let mut new_ctx = ctx;
                    new_ctx.response = new_ctx.response.header("Set-Cookie", cookie_header);

                    return Ok((new_ctx, MiddlewareResult::Continue));
                }

                return Ok((ctx, MiddlewareResult::Continue));
            }

            // Skip paths explicitly configured
            if self.should_skip_path(path) {
                return Ok((ctx, MiddlewareResult::Continue));
            }

            // Validate CSRF token for state-changing methods
            if matches!(
                method,
                Method::POST | Method::PUT | Method::DELETE | Method::PATCH
            ) {
                self.validate_token(&ctx)?;
            }

            Ok((ctx, MiddlewareResult::Continue))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpParser;

    #[tokio::test]
    async fn test_generate_token() {
        let token1 = CsrfMiddleware::generate_token();
        let token2 = CsrfMiddleware::generate_token();

        // Tokens should be different
        assert_ne!(token1, token2);
        // Tokens should be non-empty
        assert!(!token1.is_empty());
        // Tokens should be URL-safe base64 (43 chars for 32 bytes)
        assert_eq!(token1.len(), 43);
    }

    #[test]
    fn test_constant_time_comparison() {
        assert!(CsrfMiddleware::tokens_equal("abc123", "abc123"));
        assert!(!CsrfMiddleware::tokens_equal("abc123", "abc124"));
        assert!(!CsrfMiddleware::tokens_equal("abc123", "abc12"));
        assert!(!CsrfMiddleware::tokens_equal("abc123", "xyz789"));
    }

    #[tokio::test]
    async fn test_safe_method_generates_token() {
        let request_bytes = b"GET /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let csrf = CsrfMiddleware::development();

        let (new_ctx, result) = csrf.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));

        // Check Set-Cookie header was added
        let has_cookie = new_ctx
            .response
            .headers
            .iter()
            .any(|(k, v)| k == "Set-Cookie" && v.contains("csrf_token="));
        assert!(has_cookie);
    }

    #[tokio::test]
    async fn test_post_without_token_fails() {
        let request_bytes = b"POST /test HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let csrf = CsrfMiddleware::development();

        let result = csrf.call(ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_post_with_valid_token_succeeds() {
        let token = CsrfMiddleware::generate_token();
        let request = format!(
            "POST /test HTTP/1.1\r\nHost: example.com\r\nCookie: csrf_token={}\r\nX-CSRF-Token: {}\r\n\r\n",
            token, token
        );
        let request_bytes = request.as_bytes();

        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let csrf = CsrfMiddleware::development();

        let (_, result) = csrf.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));
    }

    #[tokio::test]
    async fn test_post_with_mismatched_token_fails() {
        let token1 = CsrfMiddleware::generate_token();
        let token2 = CsrfMiddleware::generate_token();
        let request = format!(
            "POST /test HTTP/1.1\r\nHost: example.com\r\nCookie: csrf_token={}\r\nX-CSRF-Token: {}\r\n\r\n",
            token1, token2
        );
        let request_bytes = request.as_bytes();

        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let csrf = CsrfMiddleware::development();

        let result = csrf.call(ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_form_field_token() {
        let token = CsrfMiddleware::generate_token();
        let body = format!("username=test&_csrf={}&password=secret", token);
        let request = format!(
            "POST /login HTTP/1.1\r\nHost: example.com\r\nCookie: csrf_token={}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{}",
            token,
            body.len(),
            body
        );
        let request_bytes = request.as_bytes();

        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body_bytes = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body_bytes);
        let csrf = CsrfMiddleware::development();

        let (_, result) = csrf.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));
    }

    #[tokio::test]
    async fn test_skip_paths() {
        let request_bytes = b"POST /webhook/stripe HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let parser = HttpParser::new();
        let parsed = parser.parse_request(request_bytes).unwrap();
        let body = &request_bytes[parsed.body_offset..];

        let ctx = Context::new(&parsed, body);
        let config = CsrfConfig::development().skip_paths(vec!["/webhook/*".to_string()]);
        let csrf = CsrfMiddleware::with_config(config);

        let (_, result) = csrf.call(ctx).await.unwrap();
        assert!(matches!(result, MiddlewareResult::Continue));
    }

    #[test]
    fn test_cookie_header_generation() {
        let csrf = CsrfMiddleware::new();
        let token = "test_token_12345";
        let cookie = csrf.build_cookie_header(token);

        assert!(cookie.contains("csrf_token=test_token_12345"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age="));
    }

    #[test]
    fn test_development_config() {
        let csrf = CsrfMiddleware::development();
        let token = "test_token";
        let cookie = csrf.build_cookie_header(token);

        // Development should not have Secure flag
        assert!(!cookie.contains("Secure") || cookie.contains("SameSite=None"));
        // But should still have HttpOnly
        assert!(cookie.contains("HttpOnly"));
    }
}
