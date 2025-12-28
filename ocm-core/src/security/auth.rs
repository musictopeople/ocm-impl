use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// API Key structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_id: String,
    pub key_hash: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub rate_limit_tier: RateLimitTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitTier {
    Basic,   // Standard rate limits
    Premium, // Higher rate limits
    Admin,   // Elevated rate limits
}

// Session structure for stateful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub user_did: String,
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub is_active: bool,
}

// Authentication context passed to handlers
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_did: Option<String>,
    pub permissions: Vec<String>,
    pub rate_limit_tier: RateLimitTier,
    pub session_id: Option<String>,
    pub api_key_id: Option<String>,
}

impl Default for AuthContext {
    fn default() -> Self {
        Self {
            user_did: None,
            permissions: vec!["public".to_string()],
            rate_limit_tier: RateLimitTier::Basic,
            session_id: None,
            api_key_id: None,
        }
    }
}

// In-memory storage for demonstration (replace with database in production)
use std::sync::{Arc, RwLock};

#[derive(Debug)]
pub struct AuthStore {
    api_keys: Arc<RwLock<HashMap<String, ApiKey>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            api_keys: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // API Key management
    pub fn create_api_key(
        &self,
        permissions: Vec<String>,
        expires_in_days: Option<i64>,
        rate_limit_tier: RateLimitTier,
    ) -> Result<(String, String), String> {
        // Generate secure API key
        let key_bytes: [u8; 32] = rand::random();
        let api_key = hex::encode(key_bytes);
        let key_id = uuid::Uuid::new_v4().to_string();

        // Hash the key for storage
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let expires_at = expires_in_days.map(|days| Utc::now() + Duration::days(days));

        let api_key_record = ApiKey {
            key_id: key_id.clone(),
            key_hash,
            permissions,
            expires_at,
            created_at: Utc::now(),
            last_used: None,
            is_active: true,
            rate_limit_tier,
        };

        self.api_keys
            .write()
            .map_err(|_| "Failed to acquire write lock")?
            .insert(key_id.clone(), api_key_record);

        Ok((key_id, api_key))
    }

    pub fn validate_api_key(&self, api_key: &str) -> Option<ApiKey> {
        // Hash the provided key
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        let key_hash = hex::encode(hasher.finalize());

        let api_keys = self.api_keys.read().ok()?;

        // Find matching key by hash
        for key_record in api_keys.values() {
            if key_record.key_hash == key_hash && key_record.is_active {
                // Check expiration
                if let Some(expires_at) = key_record.expires_at {
                    if Utc::now() > expires_at {
                        return None; // Expired
                    }
                }
                return Some(key_record.clone());
            }
        }

        None
    }

    pub fn update_api_key_usage(&self, key_id: &str) {
        if let Ok(mut api_keys) = self.api_keys.write() {
            if let Some(key_record) = api_keys.get_mut(key_id) {
                key_record.last_used = Some(Utc::now());
            }
        }
    }

    // Session management
    pub fn create_session(
        &self,
        user_did: String,
        permissions: Vec<String>,
        expires_in_hours: i64,
    ) -> Result<String, String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let session = Session {
            session_id: session_id.clone(),
            user_did,
            permissions,
            created_at: now,
            expires_at: now + Duration::hours(expires_in_hours),
            last_activity: now,
            is_active: true,
        };

        self.sessions
            .write()
            .map_err(|_| "Failed to acquire write lock")?
            .insert(session_id.clone(), session);

        Ok(session_id)
    }

    pub fn validate_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().ok()?;

        if let Some(session) = sessions.get(session_id) {
            if session.is_active && Utc::now() < session.expires_at {
                return Some(session.clone());
            }
        }

        None
    }

    pub fn update_session_activity(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.write() {
            if let Some(session) = sessions.get_mut(session_id) {
                session.last_activity = Utc::now();
            }
        }
    }

    pub fn invalidate_session(&self, session_id: &str) {
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(session_id);
        }
    }
}

// Simplified API key handling without custom headers crate

// Authentication middleware
pub async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let mut auth_context = AuthContext::default();

    // Try API key authentication first
    if let Some(api_key_header) = headers.get("x-api-key") {
        if let Ok(api_key) = api_key_header.to_str() {
            // In a real application, this would be dependency injected
            let auth_store = AuthStore::new();

            if let Some(key_record) = auth_store.validate_api_key(api_key) {
                auth_context.api_key_id = Some(key_record.key_id.clone());
                auth_context.permissions = key_record.permissions.clone();
                auth_context.rate_limit_tier = key_record.rate_limit_tier.clone();

                // Update usage
                auth_store.update_api_key_usage(&key_record.key_id);
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "invalid_api_key",
                        "message": "Invalid or expired API key"
                    })),
                ));
            }
        }
    }
    // Try session authentication
    else if let Some(session_header) = headers.get("x-session-id") {
        if let Ok(session_id) = session_header.to_str() {
            let auth_store = AuthStore::new();

            if let Some(session) = auth_store.validate_session(session_id) {
                auth_context.session_id = Some(session.session_id.clone());
                auth_context.user_did = Some(session.user_did.clone());
                auth_context.permissions = session.permissions.clone();

                // Update activity
                auth_store.update_session_activity(&session.session_id);
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "error": "invalid_session",
                        "message": "Invalid or expired session"
                    })),
                ));
            }
        }
    }

    // Add auth context to request extensions
    request.extensions_mut().insert(auth_context);

    Ok(next.run(request).await)
}

// Optional authentication middleware (allows unauthenticated access)
pub async fn optional_auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, std::convert::Infallible> {
    let mut auth_context = AuthContext::default();

    // Try authentication but don't fail if not present
    if let Some(api_key_header) = headers.get("x-api-key") {
        if let Ok(api_key) = api_key_header.to_str() {
            let auth_store = AuthStore::new();
            if let Some(key_record) = auth_store.validate_api_key(api_key) {
                auth_context.api_key_id = Some(key_record.key_id.clone());
                auth_context.permissions = key_record.permissions.clone();
                auth_context.rate_limit_tier = key_record.rate_limit_tier.clone();
                auth_store.update_api_key_usage(&key_record.key_id);
            }
        }
    }

    request.extensions_mut().insert(auth_context);
    Ok(next.run(request).await)
}

// Permission checking helper
impl AuthContext {
    pub fn has_permission(&self, required_permission: &str) -> bool {
        self.permissions.contains(&required_permission.to_string())
            || self.permissions.contains(&"admin".to_string())
    }

    pub fn require_permission(
        &self,
        required_permission: &str,
    ) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
        if !self.has_permission(required_permission) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "insufficient_permissions",
                    "message": format!("Required permission: {}", required_permission),
                    "required": required_permission
                })),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_store_creation() {
        let store = AuthStore::new();
        assert!(store.api_keys.read().is_ok());
        assert!(store.sessions.read().is_ok());
    }

    #[test]
    fn test_api_key_creation() {
        let store = AuthStore::new();
        let result = store.create_api_key(vec!["read".to_string()], Some(30), RateLimitTier::Basic);
        assert!(result.is_ok());

        let (key_id, api_key) = result.unwrap();
        assert!(!key_id.is_empty());
        assert!(!api_key.is_empty());
        assert_eq!(api_key.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_session_creation() {
        let store = AuthStore::new();
        let result = store.create_session(
            "did:plc:test123".to_string(),
            vec!["read".to_string(), "write".to_string()],
            24,
        );
        assert!(result.is_ok());

        let session_id = result.unwrap();
        assert!(!session_id.is_empty());

        // Validate the session
        let session = store.validate_session(&session_id);
        assert!(session.is_some());
        assert_eq!(session.unwrap().user_did, "did:plc:test123");
    }

    #[test]
    fn test_auth_context_permissions() {
        let mut context = AuthContext::default();
        context.permissions = vec!["read".to_string(), "write".to_string()];

        assert!(context.has_permission("read"));
        assert!(context.has_permission("write"));
        assert!(!context.has_permission("admin"));

        // Test admin override
        context.permissions.push("admin".to_string());
        assert!(context.has_permission("delete")); // Admin has all permissions
    }
}
