use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use dashmap::DashMap;
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};

// Rate limiting configurations for different endpoint types
#[derive(Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
        }
    }
}

// Predefined rate limits for different endpoint categories
pub mod limits {
    use super::RateLimitConfig;

    pub const HEALTH_CHECK: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 300,
        burst_size: 50,
    };

    pub const API_READ: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 100,
        burst_size: 20,
    };

    pub const API_WRITE: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 30,
        burst_size: 5,
    };

    pub const API_SENSITIVE: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 10,
        burst_size: 2,
    };

    pub const WEBSOCKET: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 200,
        burst_size: 30,
    };

    pub const STATIC_FILES: RateLimitConfig = RateLimitConfig {
        requests_per_minute: 500,
        burst_size: 100,
    };
}

// Rate limiter state
#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub requests: Vec<Instant>,
}

impl RateLimitState {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    pub fn is_allowed(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(60); // 1 minute window

        // Remove old requests outside the window
        self.requests
            .retain(|&request_time| request_time > window_start);

        // Check if we're within limits
        if self.requests.len() >= config.requests_per_minute as usize {
            return false;
        }

        // Check burst limit (requests in last 10 seconds)
        let burst_window_start = now - Duration::from_secs(10);
        let burst_count = self
            .requests
            .iter()
            .filter(|&&request_time| request_time > burst_window_start)
            .count();

        if burst_count >= config.burst_size as usize {
            return false;
        }

        // Add this request to the history
        self.requests.push(now);
        true
    }
}

// Global rate limiter store
pub type RateLimiterStore = Arc<DashMap<String, RateLimitState>>;

// Create a new rate limiter store
pub fn create_rate_limiter_store() -> RateLimiterStore {
    Arc::new(DashMap::new())
}

// Extract client IP from request
fn get_client_ip(headers: &HeaderMap) -> String {
    // Try X-Forwarded-For first (for proxies)
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback to unknown
    "unknown".to_string()
}

// Create rate limiting middleware
pub fn rate_limit_middleware(
    store: RateLimiterStore,
    config: RateLimitConfig,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Response, (StatusCode, Json<serde_json::Value>)>>
            + Send,
    >,
> + Clone {
    move |request: Request, next: Next| {
        let store = store.clone();
        let config = config.clone();

        Box::pin(async move {
            let client_ip = get_client_ip(request.headers());
            let mut state = store
                .entry(client_ip.clone())
                .or_insert_with(RateLimitState::new);

            if !state.is_allowed(&config) {
                return Err(rate_limit_exceeded_response());
            }

            Ok(next.run(request).await)
        })
    }
}

// Convenience middleware creators
pub fn create_health_rate_limiter(
    store: RateLimiterStore,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Response, (StatusCode, Json<serde_json::Value>)>>
            + Send,
    >,
> + Clone {
    rate_limit_middleware(store, limits::HEALTH_CHECK)
}

pub fn create_api_read_rate_limiter(
    store: RateLimiterStore,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Response, (StatusCode, Json<serde_json::Value>)>>
            + Send,
    >,
> + Clone {
    rate_limit_middleware(store, limits::API_READ)
}

pub fn create_api_write_rate_limiter(
    store: RateLimiterStore,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<Response, (StatusCode, Json<serde_json::Value>)>>
            + Send,
    >,
> + Clone {
    rate_limit_middleware(store, limits::API_WRITE)
}

// Rate limit error response (imports already at top)

pub fn rate_limit_exceeded_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "rate_limit_exceeded",
            "message": "Too many requests. Please slow down.",
            "retry_after": 60,
            "documentation": "https://docs.ocm.example.com/rate-limits"
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_creation() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 60);
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_predefined_limits() {
        assert_eq!(limits::HEALTH_CHECK.requests_per_minute, 300);
        assert_eq!(limits::API_WRITE.requests_per_minute, 30);
        assert_eq!(limits::API_SENSITIVE.requests_per_minute, 10);
    }
}
