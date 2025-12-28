use js_sys::*;
use wasm_bindgen::prelude::*;
use web_sys::*;

/// Secure WebCrypto-based utilities for browser deployment
/// This provides basic security enhancements for browser OCM deployment
#[wasm_bindgen]
pub struct SecureKeyStore {
    crypto: Crypto,
}

#[wasm_bindgen]
impl SecureKeyStore {
    /// Initialize WebCrypto API
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<SecureKeyStore, String> {
        let window = web_sys::window().ok_or("No window available")?;
        let crypto_obj = window.crypto().map_err(|_| "WebCrypto API not available")?;

        Ok(SecureKeyStore { crypto: crypto_obj })
    }

    /// Generate a cryptographically secure random key ID
    #[wasm_bindgen]
    pub fn generate_key_id(&self) -> Result<String, String> {
        // Generate a secure random ID using WebCrypto
        let mut random_bytes = [0u8; 16];
        self.crypto
            .get_random_values_with_u8_array(&mut random_bytes)
            .map_err(|_| "Failed to generate random bytes")?;

        // Convert to hex string
        Ok(hex::encode(random_bytes))
    }

    /// Generate cryptographically secure random bytes
    #[wasm_bindgen]
    pub fn get_random_bytes(&self, length: usize) -> Result<Vec<u8>, String> {
        let mut bytes = vec![0u8; length];
        self.crypto
            .get_random_values_with_u8_array(&mut bytes)
            .map_err(|_| "Failed to generate random bytes")?;
        Ok(bytes)
    }
}

/// Browser-specific secure random number generation
#[wasm_bindgen]
pub fn get_secure_random_bytes(length: usize) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("No window available")?;
    let crypto = window.crypto().map_err(|_| "WebCrypto API not available")?;
    let mut bytes = vec![0u8; length];
    crypto
        .get_random_values_with_u8_array(&mut bytes)
        .map_err(|_| "Failed to generate random bytes")?;
    Ok(bytes)
}

/// Clear sensitive data from browser storage (localStorage, sessionStorage)
#[wasm_bindgen]
pub async fn clear_browser_storage() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window available")?;

    // Clear localStorage
    if let Ok(Some(local_storage)) = window.local_storage() {
        local_storage
            .clear()
            .map_err(|_| "Failed to clear localStorage")?;
    }

    // Clear sessionStorage
    if let Ok(Some(session_storage)) = window.session_storage() {
        session_storage
            .clear()
            .map_err(|_| "Failed to clear sessionStorage")?;
    }

    crate::log!("Browser storage cleared for security");
    Ok(())
}

/// Enhanced security configuration for browser deployment
#[wasm_bindgen]
pub struct SecurityConfig {
    enable_secure_context_check: bool,
    require_https: bool,
}

#[wasm_bindgen]
impl SecurityConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SecurityConfig {
        SecurityConfig {
            enable_secure_context_check: true,
            require_https: true,
        }
    }

    /// Verify browser security context
    #[wasm_bindgen]
    pub fn check_security_context(&self) -> Result<(), String> {
        let window = web_sys::window().ok_or("No window available")?;

        if self.enable_secure_context_check {
            // Check if running in secure context (HTTPS)
            if !window.is_secure_context() {
                return Err("Application must run in secure context (HTTPS)".to_string());
            }
        }

        if self.require_https {
            let location = window.location();
            let protocol = location.protocol().map_err(|_| "Cannot read protocol")?;
            if protocol != "https:" && protocol != "file:" {
                return Err("Application must use HTTPS in production".to_string());
            }
        }

        // Check WebCrypto availability
        if window.crypto().is_err() {
            return Err("WebCrypto API not available - browser too old".to_string());
        }

        Ok(())
    }

    /// Get security status report
    #[wasm_bindgen]
    pub fn get_security_status(&self) -> String {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return "❌ No window context available".to_string(),
        };

        let mut status = Vec::new();

        // Check secure context
        if window.is_secure_context() {
            status.push("✅ Secure context (HTTPS)".to_string());
        } else {
            status.push("⚠️ Not in secure context - use HTTPS in production".to_string());
        }

        // Check WebCrypto
        if window.crypto().is_ok() {
            status.push("✅ WebCrypto API available".to_string());
        } else {
            status.push("❌ WebCrypto API not available".to_string());
        }

        // Check OPFS support
        if js_sys::Reflect::has(&window.navigator(), &"storage".into()).unwrap_or(false) {
            status.push("✅ OPFS (Origin Private File System) supported".to_string());
        } else {
            status.push("⚠️ OPFS not supported - limited offline storage".to_string());
        }

        status.join("\n")
    }

    /// Get security recommendations for deployment
    #[wasm_bindgen]
    pub fn get_security_recommendations(&self) -> String {
        let recommendations = vec![
            "🔒 Security Recommendations:",
            "",
            "✓ Always use HTTPS in production",
            "✓ Implement Content Security Policy (CSP)",
            "✓ Use SubResource Integrity (SRI) for external scripts",
            "✓ Regularly rotate cryptographic keys",
            "✓ Implement proper session management",
            "✓ Use secure cookie settings (HttpOnly, Secure, SameSite)",
            "✓ Monitor for security updates",
            "✓ Implement proper error handling (don't leak sensitive info)",
            "✓ Use strong password policies",
            "✓ Implement rate limiting for sensitive operations",
            "",
            "🔧 Technical Recommendations:",
            "✓ Enable OPFS for persistent storage",
            "✓ Use WebCrypto API for cryptographic operations",
            "✓ Implement proper key management",
            "✓ Regular security audits",
        ];

        recommendations.join("\n")
    }

    /// Disable security checks for development
    #[wasm_bindgen]
    pub fn disable_for_development(&mut self) {
        self.enable_secure_context_check = false;
        self.require_https = false;
        crate::log!("⚠️ Security checks disabled for development");
    }
}
