use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde_json::json;
use std::convert::Infallible;
// Removed unused imports

// Security headers middleware
pub async fn security_headers_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, Infallible> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
             connect-src 'self' wss: https:; img-src 'self' data:; \
             style-src 'self' 'unsafe-inline'; font-src 'self' data:; \
             object-src 'none'; base-uri 'self'; \
             frame-ancestors 'none'; upgrade-insecure-requests;",
        ),
    );

    // Strict Transport Security (HSTS)
    headers.insert(
        "Strict-Transport-Security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    // X-Frame-Options
    headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));

    // X-Content-Type-Options
    headers.insert(
        "X-Content-Type-Options",
        HeaderValue::from_static("nosniff"),
    );

    // X-XSS-Protection
    headers.insert(
        "X-XSS-Protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Referrer Policy
    headers.insert(
        "Referrer-Policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions Policy
    headers.insert(
        "Permissions-Policy",
        HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), \
             payment=(), usb=(), magnetometer=(), gyroscope=(), \
             accelerometer=(), ambient-light-sensor=()",
        ),
    );

    // Server header (minimal information disclosure)
    headers.insert("Server", HeaderValue::from_static("OCM-Server"));

    Ok(response)
}

// Request validation middleware
pub async fn request_validation_middleware(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();

    // Validate Content-Type for POST/PUT requests
    if let Some(method) = request.method().as_str().get(..4) {
        if method == "POST" || method.starts_with("PUT") {
            if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
                if let Ok(ct_str) = content_type.to_str() {
                    if !ct_str.starts_with("application/json")
                        && !ct_str.starts_with("application/x-www-form-urlencoded")
                    {
                        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
                    }
                } else {
                    return Err(StatusCode::BAD_REQUEST);
                }
            } else {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    // Validate Content-Length (prevent large payloads)
    if let Some(content_length) = headers.get(header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                // Limit request body to 10MB
                if length > 10 * 1024 * 1024 {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    // Check for suspicious headers
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            // Basic XSS detection in headers
            if value_str.contains("<script")
                || value_str.contains("javascript:")
                || value_str.contains("vbscript:")
            {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    Ok(next.run(request).await)
}

// Request size limiting middleware
pub async fn request_size_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check Content-Length header
    if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                // Different limits for different endpoints
                let path = request.uri().path();
                let limit = match path {
                    p if p.starts_with("/api/v1/memories") => 1024 * 1024, // 1MB for memories
                    p if p.starts_with("/api/v1/individuals") => 64 * 1024, // 64KB for individuals
                    p if p.starts_with("/api/v1/") => 256 * 1024, // 256KB for other API endpoints
                    _ => 10 * 1024 * 1024,                        // 10MB for everything else
                };

                if length > limit {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    Ok(next.run(request).await)
}

// JSON validation middleware
// Removed unused validation import

pub async fn json_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Only validate JSON endpoints
    let path = request.uri().path();
    if !path.starts_with("/api/") {
        return Ok(next.run(request).await);
    }

    // Check if request has JSON content type
    if let Some(content_type) = request.headers().get(header::CONTENT_TYPE) {
        if let Ok(ct_str) = content_type.to_str() {
            if ct_str.starts_with("application/json") {
                // Basic JSON structure validation will be done by axum's Json extractor
                // Custom validation happens in individual handlers using our validation structs
                return Ok(next.run(request).await);
            }
        }
    }

    Ok(next.run(request).await)
}

// CORS security middleware (more restrictive than basic CORS)
pub async fn secure_cors_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Only allow specific origins in production
    headers.insert(
        "Access-Control-Allow-Origin",
        HeaderValue::from_static("https://localhost:8443"), // Update for production
    );

    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );

    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("Content-Type, Authorization, X-Requested-With"),
    );

    headers.insert(
        "Access-Control-Max-Age",
        HeaderValue::from_static("86400"), // 24 hours
    );

    headers.insert(
        "Access-Control-Allow-Credentials",
        HeaderValue::from_static("true"),
    );

    Ok(response)
}

// Request logging middleware for security monitoring
use tracing::{info, warn};

pub async fn security_logging_middleware(
    request: Request,
    next: Next,
) -> Result<Response, Infallible> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();

    // Extract client IP (considering proxies)
    let client_ip = request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split(',').next())
        .or_else(|| {
            request
                .headers()
                .get("X-Real-IP")
                .and_then(|h| h.to_str().ok())
        })
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Log request
    info!(
        method = %method,
        path = %path,
        query = %query,
        client_ip = %client_ip,
        user_agent = %user_agent,
        "HTTP request received"
    );

    // Check for suspicious patterns
    if path.contains("..")
        || path.contains("<script")
        || query.contains("<script")
        || user_agent.contains("<script")
    {
        warn!(
            client_ip = %client_ip,
            path = %path,
            query = %query,
            user_agent = %user_agent,
            "Suspicious request detected"
        );
    }

    let response = next.run(request).await;
    let status = response.status();

    // Log response
    info!(
        method = %method,
        path = %path,
        status = %status,
        client_ip = %client_ip,
        "HTTP response sent"
    );

    Ok(response)
}

// Error response helper
pub fn create_error_response(
    status: StatusCode,
    error_code: &str,
    message: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(json!({
            "error": error_code,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": status.as_u16()
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, Response};

    // Note: Full middleware testing would require integration tests
    // These are basic unit tests for helper functions

    #[test]
    fn test_error_response_creation() {
        let (status, json_response) = create_error_response(
            StatusCode::BAD_REQUEST,
            "validation_error",
            "Invalid input provided",
        );

        assert_eq!(status, StatusCode::BAD_REQUEST);
        // Additional assertions would test the JSON structure
    }
}
