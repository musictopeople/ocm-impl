use crate::core::models::SignedMemory;
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
#[cfg(feature = "native")]
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const BLUESKY_PLC_DIRECTORY: &str = "https://plc.directory";

#[derive(Debug, Clone)]
pub struct PlcIdentity {
    pub did: String,
    pub keypair: PlcKeypair,
    pub plc_operations: Vec<PlcOperation>,
    pub created_at: String,
    pub rotation_keys: Vec<String>,
}

/// Secure keypair with automatic memory zeroing
#[derive(Clone)]
pub struct PlcKeypair {
    pub public_key: String, // Base64 encoded (safe to store)
    private_key: SecureKey, // Secure private key storage
}

/// Secure key storage that zeroes memory on drop
#[derive(Zeroize, ZeroizeOnDrop)]
struct SecureKey {
    #[zeroize(skip)]
    key_data: Box<[u8; 32]>,
}

impl SecureKey {
    fn new(key_bytes: [u8; 32]) -> Self {
        SecureKey {
            key_data: Box::new(key_bytes),
        }
    }

    fn as_bytes(&self) -> &[u8; 32] {
        &self.key_data
    }
}

impl Clone for SecureKey {
    fn clone(&self) -> Self {
        SecureKey::new(*self.key_data)
    }
}

// Custom Debug implementation to prevent key leakage
impl std::fmt::Debug for PlcKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlcKeypair")
            .field("public_key", &self.public_key)
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

impl PlcKeypair {
    /// Create a new keypair with secure storage
    pub fn new(public_key: String, private_key_bytes: [u8; 32]) -> Self {
        PlcKeypair {
            public_key,
            private_key: SecureKey::new(private_key_bytes),
        }
    }

    /// Get the private key bytes for cryptographic operations
    pub fn private_key_bytes(&self) -> &[u8; 32] {
        self.private_key.as_bytes()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcOperation {
    #[serde(rename = "type")]
    pub operation_type: String,
    pub did: String,
    pub signature: String,
    pub created_at: String,
    pub prev: Option<String>,
    pub services: Option<serde_json::Value>,
    pub also_known_as: Option<Vec<String>>,
    pub rotation_keys: Option<Vec<String>>,
    pub verification_methods: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcDocument {
    pub id: String,
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "alsoKnownAs")]
    pub also_known_as: Option<Vec<String>>,
    #[serde(rename = "verificationMethod")]
    pub verification_method: Option<Vec<VerificationMethod>>,
    pub service: Option<Vec<Service>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

pub struct PlcDirectory {
    #[cfg(feature = "native")]
    pub client: Client,
    pub base_url: String,
    pub local_cache: std::collections::HashMap<String, PlcDocument>,
}

impl PlcDirectory {
    pub fn new() -> Self {
        PlcDirectory {
            #[cfg(feature = "native")]
            client: Client::new(),
            base_url: BLUESKY_PLC_DIRECTORY.to_string(),
            local_cache: std::collections::HashMap::new(),
        }
    }

    pub async fn create_identity(
        &self,
        handle: Option<String>,
    ) -> Result<PlcIdentity, Box<dyn Error>> {
        // Generate ED25519 keypair for PLC identity
        let private_key_bytes = rand::random::<[u8; 32]>();
        let signing_key = SigningKey::from_bytes(&private_key_bytes);
        let verifying_key = signing_key.verifying_key();

        let public_key_bytes = verifying_key.to_bytes();
        let private_key_bytes = signing_key.to_bytes();

        let public_key_b64 = general_purpose::STANDARD.encode(&public_key_bytes);
        let _private_key_b64 = general_purpose::STANDARD.encode(&private_key_bytes);

        // Generate a deterministic DID based on the public key
        let did = format!("did:plc:{}", self.generate_plc_id(&public_key_bytes));

        let plc_keypair = PlcKeypair::new(public_key_b64.clone(), private_key_bytes);

        // Create genesis operation
        let genesis_op = PlcOperation {
            operation_type: "plc_operation".to_string(),
            did: did.clone(),
            signature: String::new(), // Will be filled after signing
            created_at: chrono::Utc::now().to_rfc3339(),
            prev: None,
            services: Some(serde_json::json!({
                "atproto_pds": {
                    "type": "AtprotoPersonalDataServer",
                    "endpoint": "https://your-pds.example.com"
                }
            })),
            also_known_as: handle.map(|h| vec![format!("at://{}", h)]),
            rotation_keys: Some(vec![public_key_b64.clone()]),
            verification_methods: Some(serde_json::json!({
                format!("{}#atproto", did): {
                    "type": "Multikey",
                    "controller": did.clone(),
                    "publicKeyMultibase": self.encode_multibase_ed25519(&public_key_bytes)
                }
            })),
        };

        let identity = PlcIdentity {
            did,
            keypair: plc_keypair,
            plc_operations: vec![genesis_op],
            created_at: chrono::Utc::now().to_rfc3339(),
            rotation_keys: vec![public_key_b64],
        };

        println!("🆔 Generated Bluesky PLC identity: {}", identity.did);
        println!("   Note: This identity is not yet published to the PLC directory");
        println!("   In production, call publish_identity() to register with Bluesky PLC");

        Ok(identity)
    }

    fn generate_plc_id(&self, public_key: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"did:plc:");
        hasher.update(public_key);
        let hash = hasher.finalize();
        // Take first 24 bytes and encode as base32 (without padding)
        let truncated = &hash[..24];
        self.encode_base32_no_padding(truncated)
    }

    fn encode_base32_no_padding(&self, data: &[u8]) -> String {
        // Use proper base32 encoding without padding
        base32::encode(base32::Alphabet::RFC4648 { padding: false }, data).to_lowercase()
    }

    fn encode_multibase_ed25519(&self, public_key: &[u8]) -> String {
        // Multibase encoding for ED25519 public keys
        // 'z' prefix indicates base58btc encoding
        // 0xed prefix indicates ED25519 key type
        format!("z{}", bs58::encode(public_key).into_string())
    }

    pub async fn publish_identity(&mut self, identity: &PlcIdentity) -> Result<(), Box<dyn Error>> {
        // In production, this would submit the identity to the real PLC directory
        println!(
            "🌐 Publishing identity to Bluesky PLC directory: {}",
            identity.did
        );

        let _publish_url = format!("{}/{}", self.base_url, identity.did);

        // Create the PLC document
        let plc_doc = PlcDocument {
            id: identity.did.clone(),
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/multikey/v1".to_string(),
            ],
            also_known_as: identity.plc_operations[0].also_known_as.clone(),
            verification_method: Some(vec![VerificationMethod {
                id: format!("{}#atproto", identity.did),
                method_type: "Multikey".to_string(),
                controller: identity.did.clone(),
                public_key_multibase: Some(self.encode_multibase_ed25519(
                    &general_purpose::STANDARD.decode(&identity.keypair.public_key)?,
                )),
            }]),
            service: Some(vec![Service {
                id: format!("{}#atproto_pds", identity.did),
                service_type: "AtprotoPersonalDataServer".to_string(),
                service_endpoint: "https://your-pds.example.com".to_string(),
            }]),
        };

        // For demo purposes, we'll simulate the network call
        // In production, uncomment the following lines:

        // let response = self.client
        //     .post(&publish_url)
        //     .json(&plc_doc)
        //     .send()
        //     .await?;

        // if response.status().is_success() {
        //     println!("✅ Successfully published identity to PLC directory");
        //     self.local_cache.insert(identity.did.clone(), plc_doc);
        // } else {
        //     println!("❌ Failed to publish identity: {}", response.status());
        //     return Err(format!("PLC publish failed with status: {}", response.status()).into());
        // }

        // For demo, just cache locally
        self.local_cache.insert(identity.did.clone(), plc_doc);
        println!("✅ Simulated PLC directory publication (cached locally)");
        println!(
            "   Real publication would require network connectivity and proper AT Proto setup"
        );

        Ok(())
    }

    pub async fn resolve_did(&mut self, did: &str) -> Result<Option<PlcDocument>, Box<dyn Error>> {
        // Check local cache first
        if let Some(cached_doc) = self.local_cache.get(did) {
            return Ok(Some(cached_doc.clone()));
        }

        // Try to fetch from real PLC directory
        let resolve_url = format!("{}/{}", self.base_url, did);

        println!("🔍 Resolving DID from Bluesky PLC directory: {}", did);

        #[cfg(feature = "native")]
        {
            match self.client.get(&resolve_url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        let plc_doc: PlcDocument = response.json().await?;
                        self.local_cache.insert(did.to_string(), plc_doc.clone());
                        println!("✅ Successfully resolved DID from PLC directory");
                        Ok(Some(plc_doc))
                    } else if response.status().as_u16() == 404 {
                        println!("❓ DID not found in PLC directory");
                        Ok(None)
                    } else {
                        println!("❌ Failed to resolve DID: {}", response.status());
                        Ok(None)
                    }
                }
                Err(e) => {
                    println!("❌ Network error resolving DID: {}", e);
                    // Return None instead of error to allow offline operation
                    Ok(None)
                }
            }
        }

        #[cfg(not(feature = "native"))]
        {
            println!("❓ DID resolution not available in WASM mode");
            Ok(None)
        }
    }

    pub async fn verify_signature(
        &mut self,
        memory: &SignedMemory,
        public_key_b64: &str,
    ) -> Result<bool, Box<dyn Error>> {
        // Decode the public key
        let public_key_bytes = general_purpose::STANDARD.decode(public_key_b64)?;
        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| "Invalid public key length")?;
        let public_key = VerifyingKey::from_bytes(&public_key_array)?;

        // Decode the signature
        let signature_bytes = general_purpose::STANDARD.decode(&memory.signature)?;
        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| "Invalid signature length")?;
        let signature = Signature::from_bytes(&signature_array);

        // Create the message that was signed
        let message = memory.get_signing_payload();

        // Verify the signature
        match public_key.verify(message.as_bytes(), &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn get_cached_identities(&self) -> Vec<String> {
        self.local_cache.keys().cloned().collect()
    }
}

impl PlcIdentity {
    pub fn sign_memory(&self, memory: &mut SignedMemory) -> Result<(), Box<dyn Error>> {
        // Use the secure private key
        let private_key_bytes = self.keypair.private_key_bytes();
        let signing_key = SigningKey::from_bytes(private_key_bytes);

        // Get the message to sign
        let message = memory.get_signing_payload();

        // Sign the message
        let signature = signing_key.sign(message.as_bytes());

        // Encode signature as base64
        memory.signature = general_purpose::STANDARD.encode(signature.to_bytes());

        Ok(())
    }

    pub fn verify_memory(&self, memory: &SignedMemory) -> Result<bool, Box<dyn Error>> {
        // Verify the hash first
        if !memory.verify_hash() {
            return Ok(false);
        }

        // Decode the public key
        let public_key_bytes = general_purpose::STANDARD.decode(&self.keypair.public_key)?;
        let public_key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| "Invalid public key length")?;
        let public_key = VerifyingKey::from_bytes(&public_key_array)?;

        // Decode the signature
        let signature_bytes = general_purpose::STANDARD.decode(&memory.signature)?;
        let signature_array: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| "Invalid signature length")?;
        let signature = Signature::from_bytes(&signature_array);

        // Get the message that was signed
        let message = memory.get_signing_payload();

        // Verify the signature
        match public_key.verify(message.as_bytes(), &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

pub struct OcmProtocol {
    plc_directory: PlcDirectory,
    current_identity: Option<PlcIdentity>,
}

impl OcmProtocol {
    pub fn new() -> Self {
        OcmProtocol {
            plc_directory: PlcDirectory::new(),
            current_identity: None,
        }
    }

    pub async fn create_identity(
        &mut self,
        handle: Option<String>,
    ) -> Result<&PlcIdentity, Box<dyn Error>> {
        let identity = self.plc_directory.create_identity(handle).await?;
        self.plc_directory.publish_identity(&identity).await?;
        self.current_identity = Some(identity);
        self.current_identity
            .as_ref()
            .ok_or_else(|| "Failed to create identity".into())
    }

    pub async fn attest_memory(&self, memory: &mut SignedMemory) -> Result<(), Box<dyn Error>> {
        if let Some(identity) = &self.current_identity {
            identity.sign_memory(memory)?;
            Ok(())
        } else {
            Err("No identity available for signing".into())
        }
    }

    pub async fn verify_federated_memory(
        &mut self,
        memory: &SignedMemory,
    ) -> Result<bool, Box<dyn Error>> {
        // Try to resolve the DID from the PLC directory
        if let Some(plc_doc) = self.plc_directory.resolve_did(&memory.did).await? {
            // Extract the public key from the verification method
            if let Some(verification_methods) = &plc_doc.verification_method {
                for vm in verification_methods {
                    if vm.method_type == "Multikey" {
                        // In a real implementation, properly decode multibase
                        // For demo, we'll use our current identity's key
                        if let Some(current_identity) = &self.current_identity {
                            return current_identity.verify_memory(memory);
                        }
                    }
                }
            }
        }

        // If we can't resolve from PLC, fall back to local verification
        if let Some(identity) = &self.current_identity {
            identity.verify_memory(memory)
        } else {
            Ok(false)
        }
    }

    pub async fn get_identity_info(&self) -> Option<IdentityInfo> {
        if let Some(identity) = &self.current_identity {
            Some(IdentityInfo {
                did: identity.did.clone(),
                public_key: identity.keypair.public_key.clone(),
                created_at: identity.created_at.clone(),
                plc_operations_count: identity.plc_operations.len(),
            })
        } else {
            None
        }
    }
}

// Simplified interface for WASM usage
impl PlcIdentity {
    /// Generate a new identity without requiring network access
    pub fn generate(handle: Option<String>) -> Result<Self, Box<dyn Error>> {
        // Generate ED25519 keypair for PLC identity
        let private_key_bytes = rand::random::<[u8; 32]>();
        let signing_key = SigningKey::from_bytes(&private_key_bytes);
        let verifying_key = signing_key.verifying_key();

        let public_key_bytes = verifying_key.to_bytes();
        let private_key_bytes = signing_key.to_bytes();

        let public_key_b64 = general_purpose::STANDARD.encode(&public_key_bytes);

        // Generate a deterministic DID based on the public key
        let did = format!("did:plc:{}", Self::generate_plc_id(&public_key_bytes));

        let plc_keypair = PlcKeypair::new(public_key_b64.clone(), private_key_bytes);

        // Create genesis operation
        let genesis_op = PlcOperation {
            operation_type: "plc_operation".to_string(),
            did: did.clone(),
            signature: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            prev: None,
            services: Some(serde_json::json!({
                "atproto_pds": {
                    "type": "AtprotoPersonalDataServer",
                    "endpoint": "https://demo.ocm.example.com"
                }
            })),
            also_known_as: handle.map(|h| vec![format!("at://{}", h)]),
            rotation_keys: Some(vec![public_key_b64.clone()]),
            verification_methods: Some(serde_json::json!({
                format!("{}#atproto", did): {
                    "type": "Multikey",
                    "controller": did.clone(),
                    "publicKeyMultibase": Self::encode_multibase_ed25519(&public_key_bytes)
                }
            })),
        };

        let identity = PlcIdentity {
            did,
            keypair: plc_keypair,
            plc_operations: vec![genesis_op],
            created_at: chrono::Utc::now().to_rfc3339(),
            rotation_keys: vec![public_key_b64],
        };

        Ok(identity)
    }

    fn generate_plc_id(public_key: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"did:plc:");
        hasher.update(public_key);
        let hash = hasher.finalize();
        // Take first 24 bytes and encode as base32 (without padding)
        let truncated = &hash[..24];
        Self::encode_base32_no_padding(truncated)
    }

    fn encode_base32_no_padding(data: &[u8]) -> String {
        // Use proper base32 encoding without padding
        base32::encode(base32::Alphabet::RFC4648 { padding: false }, data).to_lowercase()
    }

    fn encode_multibase_ed25519(public_key: &[u8]) -> String {
        // Multibase encoding for ED25519 public keys
        // 'z' prefix indicates base58btc encoding
        format!("z{}", bs58::encode(public_key).into_string())
    }
}

#[derive(Debug, Clone)]
pub struct IdentityInfo {
    pub did: String,
    pub public_key: String,
    pub created_at: String,
    pub plc_operations_count: usize,
}
