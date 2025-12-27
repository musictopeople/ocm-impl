# OCM API Documentation

## Overview

The Our Collective Memory (OCM) implementation provides a distributed memory protocol for decentralized identity and data sovereignty. This API documentation covers the core modules and their public interfaces.

## Core Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Identity      │    │   Persistence   │    │   Networking    │
│   (PLC/DID)     │◄──►│   (SQLite)      │◄──►│   (P2P/TCP)     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         ▲                        ▲                        ▲
         │                        │                        │
         ▼                        ▼                        ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Claims        │    │   Models        │    │   Sync/CRDT     │
│   (Proxy)       │    │   (Data)        │    │   (Conflict)    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Core Modules

### 1. Identity Module (`src/identity/`)

#### `OcmProtocol`
Main protocol coordinator for identity and cryptographic operations.

```rust
pub struct OcmProtocol {
    plc_directory: PlcDirectory,
    current_identity: Option<PlcIdentity>,
}

impl OcmProtocol {
    pub fn new() -> Self
    pub async fn create_identity(&mut self, handle: Option<String>) -> Result<&PlcIdentity, Box<dyn Error>>
    pub async fn attest_memory(&self, memory: &mut SignedMemory) -> Result<(), Box<dyn Error>>
    pub async fn verify_federated_memory(&mut self, memory: &SignedMemory) -> Result<bool, Box<dyn Error>>
    pub async fn get_identity_info(&self) -> Option<IdentityInfo>
}
```

**Usage Example:**
```rust
let mut ocm = OcmProtocol::new();
let identity = ocm.create_identity(Some("alice".to_string())).await?;
println!("Created identity: {}", identity.did);

// Sign a memory
let mut memory = SignedMemory::new(&identity.did, "individual", &data);
ocm.attest_memory(&mut memory).await?;
```

#### `PlcDirectory`
Manages Bluesky PLC (Public Ledger of Credentials) directory operations.

```rust
pub struct PlcDirectory {
    pub client: Client,
    pub base_url: String,
    pub local_cache: std::collections::HashMap<String, PlcDocument>,
}

impl PlcDirectory {
    pub fn new() -> Self
    pub async fn create_identity(&self, handle: Option<String>) -> Result<PlcIdentity, Box<dyn Error>>
    pub async fn publish_identity(&mut self, identity: &PlcIdentity) -> Result<(), Box<dyn Error>>
    pub async fn resolve_did(&mut self, did: &str) -> Result<Option<PlcDocument>, Box<dyn Error>>
    pub async fn verify_signature(&mut self, memory: &SignedMemory, public_key_b64: &str) -> Result<bool, Box<dyn Error>>
}
```

#### `ClaimSystem`
Handles proxy records and claim tokens for organizational use cases.

```rust
pub struct ClaimSystem {
    db: Arc<Database>,
}

impl ClaimSystem {
    pub fn new(db: Arc<Database>) -> Self
    pub async fn create_proxy_record(&self, ocm: &mut OcmProtocol, organization_did: &str, proxy_for_name: &str, proxy_for_info: Option<String>, individual_data: &Individual) -> Result<(ProxyMemory, ClaimToken), OcmError>
    pub async fn claim_proxy_record(&self, ocm: &mut OcmProtocol, token_code: &str, claimer_did: &str) -> Result<SignedMemory, OcmError>
    pub fn get_claim_statistics(&self, organization_did: &str) -> Result<ClaimStatistics, OcmError>
}
```

### 2. Persistence Module (`src/persistence/`)

#### `Database`
SQLite database operations with CRUD support for all models.

```rust
pub struct Database {
    connection_pool: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(database_path: &str) -> Result<Self, OcmError>
    
    // Generic CRUD operations
    pub fn create<T: DatabaseModel>(&self, model: &T) -> Result<(), OcmError>
    pub fn get<T: DatabaseModel>(&self, id: &str) -> Result<Option<T>, OcmError>
    pub fn update<T: DatabaseModel>(&self, model: &T) -> Result<(), OcmError>
    pub fn delete<T: DatabaseModel>(&self, id: &str) -> Result<(), OcmError>
    pub fn list<T: DatabaseModel>(&self) -> Result<Vec<T>, OcmError>
    
    // Memory-specific operations
    pub fn create_signed_memory(&self, memory: &SignedMemory) -> Result<(), OcmError>
    pub fn list_memories_by_did(&self, did: &str) -> Result<Vec<SignedMemory>, OcmError>
    pub fn list_signed_memories(&self) -> Result<Vec<SignedMemory>, OcmError>
    
    // Individual operations
    pub fn create_individual(&self, individual: &Individual) -> Result<(), OcmError>
    pub fn search_individuals(&self, first_name: Option<&str>, last_name: Option<&str>, email: Option<&str>) -> Result<Vec<Individual>, OcmError>
    
    // Claim token operations
    pub fn get_claim_token_by_token(&self, token: &str) -> Result<Option<ClaimToken>, OcmError>
    pub fn list_claim_tokens_by_organization(&self, organization_did: &str) -> Result<Vec<ClaimToken>, OcmError>
}
```

### 3. Models Module (`src/core/models.rs`)

#### Core Data Structures

**Individual**
```rust
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
```

**SignedMemory**
```rust
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
    pub fn new(did: &str, memory_type: &str, memory_data: &str) -> Self
    pub fn compute_hash(data: &str) -> String
    pub fn get_signing_payload(&self) -> String
    pub fn verify_hash(&self) -> bool
}
```

**ClaimToken**
```rust
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
    pub fn new(memory_id: &str, organization_did: &str, expires_in_hours: i64) -> Self
    pub fn is_expired(&self) -> bool
    pub fn is_claimed(&self) -> bool
    pub fn claim(&mut self, claimer_did: &str) -> Result<(), String>
}
```

### 4. Networking Module (`src/networking/`)

#### `OcmNetworking`
P2P networking layer for federation.

```rust
pub struct OcmNetworking {
    pub local_peer_id: String,
    port: u16,
    ocm_protocol: Arc<Mutex<OcmProtocol>>,
    database: Arc<Database>,
    connections: Arc<Mutex<HashMap<String, TcpStream>>>,
}

impl OcmNetworking {
    pub fn new(port: u16, ocm_protocol: OcmProtocol, database: Arc<Database>) -> Self
    pub async fn start_server(&self) -> Result<(), OcmError>
    pub async fn connect_to_peer(&self, peer_address: &str) -> Result<(), OcmError>
    pub async fn broadcast_memory(&self, memory: &SignedMemory) -> Result<(), OcmError>
    pub async fn sync_with_peer(&self, peer_id: &str) -> Result<(), OcmError>
    pub async fn start_heartbeat(&self) -> Result<(), OcmError>
}
```

#### `PeerDiscovery`
UDP-based peer discovery service.

```rust
pub struct PeerDiscovery {
    local_peer_id: String,
    discovery_port: u16,
    ocm_port: u16,
    local_did: Option<String>,
}

impl PeerDiscovery {
    pub fn new(local_peer_id: String, discovery_port: u16, ocm_port: u16, local_did: Option<String>) -> Self
    pub async fn start_discovery_service(&self) -> Result<(), OcmError>
    pub async fn start_periodic_discovery(&self) -> Result<(), OcmError>
    pub async fn add_seed_peers(&self, peer_addresses: Vec<&str>) -> Result<(), OcmError>
    pub async fn connect_discovered_peers(&self, networking: &Arc<OcmNetworking>) -> Result<(), OcmError>
}
```

### 5. Sync Module (`src/sync/`)

#### `SyncManager`
Manages memory synchronization and conflict resolution.

```rust
pub struct SyncManager {
    peer_id: String,
    database: Arc<Database>,
    networking: Arc<OcmNetworking>,
    crdt_state: Arc<Mutex<CrdtState>>,
}

impl SyncManager {
    pub fn new(peer_id: String, database: Arc<Database>, networking: Arc<OcmNetworking>) -> Self
    pub async fn start_sync_service(&self) -> Result<(), OcmError>
    pub async fn initialize_crdt_from_database(&self) -> Result<(), OcmError>
    pub async fn update_memory_field(&self, memory_id: &str, field_name: &str, new_value: serde_json::Value) -> Result<(), OcmError>
    pub async fn get_sync_statistics(&self) -> SyncStatistics
    pub async fn get_conflict_summary(&self) -> ConflictSummary
}
```

## Configuration Module (`src/config/`)

#### `OcmConfig`
Application configuration management.

```rust
#[derive(Debug, Clone)]
pub struct OcmConfig {
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub identity: IdentityConfig,
    pub logging: LoggingConfig,
}

impl OcmConfig {
    pub fn from_env() -> Result<Self, OcmError>
    pub fn validate(&self) -> Result<(), OcmError>
}
```

## Error Handling

All operations return `Result<T, OcmError>` where `OcmError` provides detailed error information:

```rust
#[derive(Debug, thiserror::Error)]
pub enum OcmError {
    #[error("Database operation failed: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    
    #[error("Network operation failed: {0}")]
    NetworkError(String),
    
    #[error("Cryptographic operation failed: {0}")]
    CryptographicError(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}
```

## Usage Examples

### Basic OCM Flow

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize configuration
    let config = OcmConfig::from_env()?;
    
    // Initialize database
    let db = Arc::new(Database::new(&config.database.path)?);
    
    // Create identity
    let mut ocm = OcmProtocol::new();
    let identity = ocm.create_identity(Some("alice".to_string())).await?;
    
    // Create and sign memory
    let individual = Individual {
        id: uuid::Uuid::new_v4().to_string(),
        first_name: "Alice".to_string(),
        last_name: "Smith".to_string(),
        email: Some("alice@example.com".to_string()),
        // ... other fields
    };
    
    let memory_data = serde_json::to_string(&individual)?;
    let mut memory = SignedMemory::new(&identity.did, "individual", &memory_data);
    ocm.attest_memory(&mut memory).await?;
    
    // Store in database
    db.create_signed_memory(&memory)?;
    
    // Start networking
    let networking = Arc::new(OcmNetworking::new(8080, ocm, db.clone()));
    networking.start_server().await?;
    
    Ok(())
}
```

### Claim Token Workflow

```rust
// Organization creates proxy record
let claim_system = ClaimSystem::new(db.clone());
let (proxy, token) = claim_system.create_proxy_record(
    &mut camp_ocm,
    &camp_did,
    "Jamie Smith",
    Some("Summer camp participant".to_string()),
    &jamie_data,
).await?;

println!("Claim token: {}", token.token);

// Parent claims the record later
let parent_identity = parent_ocm.create_identity(Some("parent".to_string())).await?;
let claimed_memory = claim_system.claim_proxy_record(
    &mut parent_ocm,
    &token.token,
    &parent_identity.did
).await?;
```

## Security Considerations

⚠️ **Important Security Notes:**

1. **Private Key Storage**: Currently stored as base64 in memory - implement secure storage for production
2. **Network Security**: No TLS encryption implemented - add for production networks
3. **Token Entropy**: Current tokens use only 4-character randomness - increase for production
4. **Input Validation**: Limited validation on network inputs - enhance before production use
5. **Rate Limiting**: No rate limiting on discovery service - implement for DoS protection

See the separate Security Audit document for detailed vulnerability analysis and remediation recommendations.

## Development and Testing

### Running the Application

```bash
# Initialize database
cargo run --bin migrate

# Start OCM node
cargo run --bin ocm-impl
```

### Environment Variables

```bash
export OCM_DATABASE_PATH="./data/ocm.db"
export OCM_NETWORK_PORT="8080"
export OCM_DISCOVERY_PORT="8081"
export OCM_LOG_LEVEL="info"
```

## Future Enhancements

- WebAssembly compilation for browser deployment
- Real PLC network integration
- Enhanced relay infrastructure for NAT traversal
- Web UI for non-technical users
- Mobile application support
- Enhanced security hardening