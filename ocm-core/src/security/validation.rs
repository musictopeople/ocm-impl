use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Simple validation functions without complex regex to avoid syntax issues
pub fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("Email is required".to_string());
    }
    if email.len() > 254 {
        return Err("Email too long".to_string());
    }
    if !email.contains('@') || !email.contains('.') {
        return Err("Invalid email format".to_string());
    }
    Ok(())
}

pub fn validate_uuid(uuid: &str) -> Result<(), String> {
    if uuid.is_empty() {
        return Err("UUID is required".to_string());
    }
    if uuid.len() != 36 {
        return Err("Invalid UUID format".to_string());
    }
    let parts: Vec<&str> = uuid.split('-').collect();
    if parts.len() != 5
        || parts[0].len() != 8
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 4
        || parts[4].len() != 12
    {
        return Err("Invalid UUID format".to_string());
    }
    Ok(())
}

pub fn validate_safe_text(text: &str, max_length: usize) -> Result<(), String> {
    if text.len() > max_length {
        return Err("Text too long".to_string());
    }
    if text.contains('<') || text.contains('>') || text.contains('\0') {
        return Err("Text contains unsafe characters".to_string());
    }
    Ok(())
}

pub fn validate_timestamp(timestamp: &str) -> Result<(), String> {
    if timestamp.is_empty() {
        return Err("Timestamp is required".to_string());
    }
    if chrono::DateTime::parse_from_rfc3339(timestamp).is_err() {
        return Err("Invalid timestamp format".to_string());
    }
    Ok(())
}

pub fn validate_content_hash(hash: &str) -> Result<(), String> {
    if hash.is_empty() {
        return Err("Hash is required".to_string());
    }
    if hash.len() != 64 {
        return Err("Invalid hash length".to_string());
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid hash format".to_string());
    }
    Ok(())
}

// Input sanitization functions
pub fn sanitize_text(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        .replace('\0', "")
}

pub fn sanitize_html(input: &str) -> String {
    input
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// Validation result structure
#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: HashMap::new(),
        }
    }

    pub fn add_error(&mut self, field: String, message: String) {
        self.is_valid = false;
        self.errors
            .entry(field)
            .or_insert_with(Vec::new)
            .push(message);
    }

    pub fn has_errors(&self) -> bool {
        !self.is_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("").is_err());
    }

    #[test]
    fn test_uuid_validation() {
        assert!(validate_uuid("123e4567-e89b-12d3-a456-426614174000").is_ok());
        assert!(validate_uuid("invalid").is_err());
        assert!(validate_uuid("").is_err());
    }

    #[test]
    fn test_safe_text_validation() {
        assert!(validate_safe_text("Hello World!", 20).is_ok());
        assert!(validate_safe_text("Hello <script>", 50).is_err());
        assert!(validate_safe_text("Very long text".repeat(10).as_str(), 10).is_err());
    }
}
