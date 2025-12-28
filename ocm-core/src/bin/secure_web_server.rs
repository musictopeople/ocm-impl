use axum::{middleware, routing::get, Router};
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::{info, warn};

#[cfg(feature = "native")]
use axum_server::tls_rustls::RustlsConfig;
#[cfg(feature = "native")]
use rustls;

// Import our security modules
#[cfg(feature = "native")]
use ocm_core::security::{
    auth::*,
    middleware::*,
    rate_limiting::{
        create_api_read_rate_limiter, create_health_rate_limiter, create_rate_limiter_store,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize crypto provider for rustls
    #[cfg(feature = "native")]
    {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    let app = create_app().await;

    // Try to set up HTTPS if certificates are available
    if let Ok(_) = setup_https_server(app.clone()).await {
        info!("🔒 HTTPS server started successfully");
    } else {
        warn!("⚠️ HTTPS setup failed, falling back to HTTP");
        setup_http_server(app).await?;
    }

    Ok(())
}

#[cfg(feature = "native")]
async fn create_app() -> Router {
    // Create rate limiter store
    let rate_limiter_store = create_rate_limiter_store();

    // Build API routes with appropriate rate limiting and security
    let api_routes = Router::new()
        .route("/status", get(api_status))
        .route("/security", get(security_status))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(create_api_read_rate_limiter(
                    rate_limiter_store.clone(),
                )))
                .layer(middleware::from_fn(optional_auth_middleware))
                .layer(middleware::from_fn(request_validation_middleware)),
        );

    // Health check route with higher rate limits
    let health_routes =
        Router::new()
            .route("/health", get(health_check))
            .layer(middleware::from_fn(create_health_rate_limiter(
                rate_limiter_store.clone(),
            )));

    // Static file serving (no rate limiting for now to avoid complexity)
    let static_routes = Router::new().nest_service("/", ServeDir::new("ocm-wasm"));

    // Combine all routes with global security middleware
    Router::new()
        .nest("/api/v1", api_routes)
        .merge(health_routes)
        .merge(static_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::from_fn(security_headers_middleware))
                .layer(middleware::from_fn(security_logging_middleware))
                .layer(middleware::from_fn(request_size_limit_middleware))
                .layer(CorsLayer::permissive()), // Will be replaced by secure_cors_middleware in production
        )
}

#[cfg(not(feature = "native"))]
async fn create_app() -> Router {
    // Simplified version for non-native builds
    let cors = CorsLayer::permissive();

    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/status", get(api_status))
        .route("/api/v1/security", get(security_status))
        .nest_service("/", ServeDir::new("ocm-wasm"))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}

#[cfg(feature = "native")]
async fn setup_https_server(app: Router) -> Result<(), Box<dyn std::error::Error>> {
    // Check if certificates exist
    if !Path::new("certs/cert.pem").exists() || !Path::new("certs/key.pem").exists() {
        return Err("TLS certificates not found".into());
    }

    info!("🔒 TLS certificates found, setting up HTTPS server...");

    // Configure TLS using axum-server
    let config = RustlsConfig::from_pem_file("certs/cert.pem", "certs/key.pem").await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8443));

    info!("🔒 HTTPS server listening on {}", addr);
    info!("🔗 Visit: https://127.0.0.1:8443");
    info!("📜 TLS certificates loaded from certs/");

    // Start HTTPS server using axum-server with TLS
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[cfg(not(feature = "native"))]
async fn setup_https_server(_app: Router) -> Result<(), Box<dyn std::error::Error>> {
    Err("TLS not available in WASM build".into())
}

async fn setup_http_server(app: Router) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = TcpListener::bind(&addr).await?;

    warn!("🌐 HTTP server listening on {} (DEVELOPMENT ONLY)", addr);
    warn!("⚠️  Use HTTPS in production!");
    info!("🔗 Visit: http://127.0.0.1:8000");
    info!("🔒 Run with 'cargo run --bin secure-web-server' for enhanced security");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn api_status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "OCM Enhanced Web Server",
        "version": "0.1.0",
        "security": {
            "protocol": "HTTP (development)",
            "cors": "configured",
            "headers": "basic",
            "note": "Use HTTPS in production"
        },
        "features": [
            "WASM deployment",
            "OPFS persistence",
            "WebCrypto integration",
            "Zero-install browser OCM",
            "Enhanced security headers",
            "Production-ready foundation"
        ],
        "endpoints": {
            "health": "/health",
            "status": "/api/v1/status",
            "security": "/api/v1/security"
        },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn security_status() -> axum::Json<serde_json::Value> {
    let has_certs = Path::new("certs/cert.pem").exists() && Path::new("certs/key.pem").exists();
    let using_https = has_certs; // If certs exist, we're likely running HTTPS

    axum::Json(serde_json::json!({
        "security_assessment": {
            "current_protocol": if using_https { "HTTPS" } else { "HTTP" },
            "recommended_protocol": "HTTPS",
            "certificates_available": has_certs,
            "browser_security": {
                "webcrypto_api": "available",
                "opfs_persistence": "secure",
                "same_origin_policy": "enforced",
                "cors": "configured"
            },
            "recommendations": [
                "Deploy with HTTPS in production",
                "Use Let's Encrypt for certificates",
                "Configure Content Security Policy",
                "Enable security headers (HSTS, etc.)",
                "Regular security audits"
            ],
            "deployment_readiness": {
                "development": if using_https { "ready_with_https" } else { "ready" },
                "staging": if using_https { "https_ready" } else { "needs_https" },
                "production": if using_https { "needs_prod_certs_and_audit" } else { "needs_https_and_audit" }
            }
        },
        "security_features": {
            "cryptographic_identity": "Ed25519 + PLC DID",
            "memory_signing": "SHA256 + Ed25519 signatures",
            "browser_storage": "OPFS (Origin Private File System)",
            "data_sovereignty": "Local SQLite ownership",
            "zero_trust": "Cryptographic verification"
        },
        "next_steps": [
            "Implement TLS 1.3 for production",
            "Add input validation middleware",
            "Configure rate limiting",
            "Set up monitoring and alerting"
        ]
    }))
}
