#[cfg(feature = "native")]
use rusqlite::{Result, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Individual {
    pub id: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub dob: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub employer: Option<String>,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
    pub coordinates_lat: Option<f64>,
    pub coordinates_lon: Option<f64>,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Affiliation {
    pub id: String,
    pub name: String,
    pub affiliation_type: AffiliationType,
    pub value: Option<String>,
    pub range_min: Option<i32>,
    pub range_max: Option<i32>,
    pub cohort: Option<String>,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffiliationType {
    Range,
    Value,
    Cohort,
}

impl AffiliationType {
    pub fn to_string(&self) -> String {
        match self {
            AffiliationType::Range => "RANGE".to_string(),
            AffiliationType::Value => "VALUE".to_string(),
            AffiliationType::Cohort => "COHORT".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Result<Self, String> {
        match s {
            "RANGE" => Ok(AffiliationType::Range),
            "VALUE" => Ok(AffiliationType::Value),
            "COHORT" => Ok(AffiliationType::Cohort),
            _ => Err(format!("Invalid affiliation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub id: String,
    pub name: String,
    pub condition_type: ConditionType,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub calculated_age_from: Option<String>,
    pub calculated_age_to: Option<String>,
    pub coordinates_lat: Option<f64>,
    pub coordinates_lon: Option<f64>,
    pub distance: Option<f64>,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Age,
    Coordinates,
}

impl ConditionType {
    pub fn to_string(&self) -> String {
        match self {
            ConditionType::Age => "AGE".to_string(),
            ConditionType::Coordinates => "COORDINATES".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Result<Self, String> {
        match s {
            "AGE" => Ok(ConditionType::Age),
            "COORDINATES" => Ok(ConditionType::Coordinates),
            _ => Err(format!("Invalid condition type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cohort {
    pub id: String,
    pub name: String,
    pub capacity: Option<f64>,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub name: String,
    pub updated_on: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub days_of_week_min: Option<i32>,
    pub days_of_week_max: Option<i32>,
}

#[cfg(feature = "native")]
pub trait DatabaseModel: Sized {
    fn table_name() -> &'static str;
    fn id(&self) -> &str;
    fn from_row(row: &Row) -> Result<Self>;
    fn insert_sql() -> &'static str;
    fn update_sql() -> &'static str;
    fn select_fields() -> &'static str;
}

#[cfg(feature = "native")]
impl DatabaseModel for Individual {
    fn table_name() -> &'static str {
        "individual"
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn from_row(row: &Row) -> Result<Self> {
        Ok(Individual {
            id: row.get(0)?,
            first_name: row.get(1)?,
            middle_name: row.get(2)?,
            last_name: row.get(3)?,
            dob: row.get(4)?,
            phone: row.get(5)?,
            email: row.get(6)?,
            employer: row.get(7)?,
            updated_on: row.get(8)?,
        })
    }

    fn insert_sql() -> &'static str {
        "INSERT INTO individual (id, first_name, middle_name, last_name, dob, phone, email, employer, updated_on) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    }

    fn update_sql() -> &'static str {
        "UPDATE individual SET first_name = ?2, middle_name = ?3, last_name = ?4, dob = ?5, phone = ?6, email = ?7, employer = ?8, updated_on = ?9 WHERE id = ?1"
    }

    fn select_fields() -> &'static str {
        "id, first_name, middle_name, last_name, dob, phone, email, employer, updated_on"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMemory {
    pub id: String,
    pub did: String,          // DID:PLC identifier of the author
    pub memory_type: String,  // Type of memory (individual, location, etc.)
    pub memory_data: String,  // JSON serialized memory content
    pub content_hash: String, // SHA256 hash of memory_data
    pub signature: String,    // Cryptographic signature
    pub timestamp: String,    // ISO 8601 timestamp
    pub updated_on: String,
}

impl SignedMemory {
    pub fn new(did: &str, memory_type: &str, memory_data: &str) -> Self {
        let content_hash = Self::compute_hash(memory_data);
        let timestamp = chrono::Utc::now().to_rfc3339();
        let updated_on = timestamp.clone();

        SignedMemory {
            id: uuid::Uuid::new_v4().to_string(),
            did: did.to_string(),
            memory_type: memory_type.to_string(),
            memory_data: memory_data.to_string(),
            content_hash,
            signature: String::new(), // Will be set during signing
            timestamp,
            updated_on,
        }
    }

    pub fn compute_hash(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn get_signing_payload(&self) -> String {
        // Create deterministic payload for signing
        serde_json::json!({
            "did": self.did,
            "memory_type": self.memory_type,
            "content_hash": self.content_hash,
            "timestamp": self.timestamp
        })
        .to_string()
    }

    pub fn verify_hash(&self) -> bool {
        let computed_hash = Self::compute_hash(&self.memory_data);
        computed_hash == self.content_hash
    }
}

#[cfg(feature = "native")]
impl DatabaseModel for SignedMemory {
    fn table_name() -> &'static str {
        "signed_memory"
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn from_row(row: &Row) -> Result<Self> {
        Ok(SignedMemory {
            id: row.get(0)?,
            did: row.get(1)?,
            memory_type: row.get(2)?,
            memory_data: row.get(3)?,
            content_hash: row.get(4)?,
            signature: row.get(5)?,
            timestamp: row.get(6)?,
            updated_on: row.get(7)?,
        })
    }

    fn insert_sql() -> &'static str {
        "INSERT INTO signed_memory (id, did, memory_type, memory_data, content_hash, signature, timestamp, updated_on) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    }

    fn update_sql() -> &'static str {
        "UPDATE signed_memory SET did = ?2, memory_type = ?3, memory_data = ?4, content_hash = ?5, signature = ?6, timestamp = ?7, updated_on = ?8 WHERE id = ?1"
    }

    fn select_fields() -> &'static str {
        "id, did, memory_type, memory_data, content_hash, signature, timestamp, updated_on"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimToken {
    pub id: String,
    pub token: String,
    pub memory_id: String,
    pub organization_did: String,
    pub expiry_timestamp: String,
    pub claimed_by_did: Option<String>,
    pub claimed_timestamp: Option<String>,
    pub created_timestamp: String,
    pub updated_on: String,
}

impl ClaimToken {
    pub fn new(memory_id: &str, organization_did: &str, expires_in_hours: i64) -> Self {
        let now = chrono::Utc::now();
        let expiry = now + chrono::Duration::hours(expires_in_hours);

        // Generate cryptographically secure token with 128 bits of entropy
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        let mut random_bytes = [0u8; 16]; // 128 bits
        rng.fill_bytes(&mut random_bytes);

        // Encode as base32 for human readability while maintaining security
        let random_part =
            base32::encode(base32::Alphabet::RFC4648 { padding: false }, &random_bytes);
        let token = format!("OCM-{}", &random_part[..16]); // Take first 16 chars for readability

        ClaimToken {
            id: uuid::Uuid::new_v4().to_string(),
            token,
            memory_id: memory_id.to_string(),
            organization_did: organization_did.to_string(),
            expiry_timestamp: expiry.to_rfc3339(),
            claimed_by_did: None,
            claimed_timestamp: None,
            created_timestamp: now.to_rfc3339(),
            updated_on: now.to_rfc3339(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(&self.expiry_timestamp) {
            chrono::Utc::now() > expiry.with_timezone(&chrono::Utc)
        } else {
            true
        }
    }

    pub fn is_claimed(&self) -> bool {
        self.claimed_by_did.is_some()
    }

    pub fn claim(&mut self, claimer_did: &str) -> Result<(), String> {
        if self.is_expired() {
            return Err("Token has expired".to_string());
        }
        if self.is_claimed() {
            return Err("Token has already been claimed".to_string());
        }

        self.claimed_by_did = Some(claimer_did.to_string());
        self.claimed_timestamp = Some(chrono::Utc::now().to_rfc3339());
        self.updated_on = chrono::Utc::now().to_rfc3339();
        Ok(())
    }
}

#[cfg(feature = "native")]
impl DatabaseModel for ClaimToken {
    fn table_name() -> &'static str {
        "claim_token"
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn from_row(row: &Row) -> Result<Self> {
        Ok(ClaimToken {
            id: row.get(0)?,
            token: row.get(1)?,
            memory_id: row.get(2)?,
            organization_did: row.get(3)?,
            expiry_timestamp: row.get(4)?,
            claimed_by_did: row.get(5)?,
            claimed_timestamp: row.get(6)?,
            created_timestamp: row.get(7)?,
            updated_on: row.get(8)?,
        })
    }

    fn insert_sql() -> &'static str {
        "INSERT INTO claim_token (id, token, memory_id, organization_did, expiry_timestamp, claimed_by_did, claimed_timestamp, created_timestamp, updated_on) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    }

    fn update_sql() -> &'static str {
        "UPDATE claim_token SET token = ?2, memory_id = ?3, organization_did = ?4, expiry_timestamp = ?5, claimed_by_did = ?6, claimed_timestamp = ?7, created_timestamp = ?8, updated_on = ?9 WHERE id = ?1"
    }

    fn select_fields() -> &'static str {
        "id, token, memory_id, organization_did, expiry_timestamp, claimed_by_did, claimed_timestamp, created_timestamp, updated_on"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyMemory {
    pub id: String,
    pub proxy_for_name: String,
    pub proxy_for_info: Option<String>,
    pub organization_did: String,
    pub memory_data: String,
    pub created_timestamp: String,
    pub claim_token_id: Option<String>,
}

impl ProxyMemory {
    pub fn new(
        proxy_for_name: &str,
        proxy_for_info: Option<String>,
        organization_did: &str,
        memory_data: &str,
    ) -> Self {
        let now = chrono::Utc::now();

        ProxyMemory {
            id: uuid::Uuid::new_v4().to_string(),
            proxy_for_name: proxy_for_name.to_string(),
            proxy_for_info,
            organization_did: organization_did.to_string(),
            memory_data: memory_data.to_string(),
            created_timestamp: now.to_rfc3339(),
            claim_token_id: None,
        }
    }
}

#[cfg(feature = "native")]
impl DatabaseModel for ProxyMemory {
    fn table_name() -> &'static str {
        "proxy_memory"
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn from_row(row: &Row) -> Result<Self> {
        Ok(ProxyMemory {
            id: row.get(0)?,
            proxy_for_name: row.get(1)?,
            proxy_for_info: row.get(2)?,
            organization_did: row.get(3)?,
            memory_data: row.get(4)?,
            created_timestamp: row.get(5)?,
            claim_token_id: row.get(6)?,
        })
    }

    fn insert_sql() -> &'static str {
        "INSERT INTO proxy_memory (id, proxy_for_name, proxy_for_info, organization_did, memory_data, created_timestamp, claim_token_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    }

    fn update_sql() -> &'static str {
        "UPDATE proxy_memory SET proxy_for_name = ?2, proxy_for_info = ?3, organization_did = ?4, memory_data = ?5, created_timestamp = ?6, claim_token_id = ?7 WHERE id = ?1"
    }

    fn select_fields() -> &'static str {
        "id, proxy_for_name, proxy_for_info, organization_did, memory_data, created_timestamp, claim_token_id"
    }
}
