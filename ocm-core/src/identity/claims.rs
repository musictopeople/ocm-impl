use crate::persistence::database::Database;
use crate::core::error::{OcmError, Result};
use crate::core::models::{ClaimToken, Individual, ProxyMemory, SignedMemory};
use crate::identity::plc::OcmProtocol;
use std::sync::Arc;

pub struct ClaimSystem {
    db: Arc<Database>,
}

impl ClaimSystem {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Organization creates a proxy record for someone (like a summer camp creating a record for Jamie)
    /// Returns a claim token that can be shared with the individual/parent
    pub async fn create_proxy_record(
        &self,
        ocm_protocol: &mut OcmProtocol,
        organization_did: &str,
        proxy_for_name: &str,
        proxy_for_info: Option<String>,
        individual_data: &Individual,
    ) -> Result<(ProxyMemory, ClaimToken)> {
        // Serialize the individual data
        let memory_data = serde_json::to_string(individual_data)
            .map_err(|e| OcmError::OperationFailed(format!("Failed to serialize data: {}", e)))?;

        // Create proxy memory entry
        let mut proxy = ProxyMemory::new(proxy_for_name, proxy_for_info, organization_did, &memory_data);
        
        // Create a signed memory for this proxy data (signed by organization)
        let mut signed_memory = SignedMemory::new(organization_did, "proxy_individual", &memory_data);
        
        // Sign the memory with organization's credentials
        ocm_protocol.attest_memory(&mut signed_memory).await?;
        
        // Store the signed memory
        self.db.create_signed_memory(&signed_memory)?;
        
        // Create claim token that expires in 30 days (reasonable for camp scenarios)
        let claim_token = ClaimToken::new(&signed_memory.id, organization_did, 30 * 24); // 30 days
        
        // Link the proxy to the claim token
        proxy.claim_token_id = Some(claim_token.id.clone());
        
        // Store both records
        self.db.create_proxy_memory(&proxy)?;
        self.db.create_claim_token(&claim_token)?;

        println!("🎫 Generated claim token: {} for {}", claim_token.token, proxy_for_name);
        println!("   Organization: {}", organization_did);
        println!("   Expires: {}", claim_token.expiry_timestamp);

        Ok((proxy, claim_token))
    }

    /// Individual/parent claims ownership of a proxy record using the token
    /// This transfers the data from organization's control to individual's control
    pub async fn claim_proxy_record(
        &self,
        ocm_protocol: &mut OcmProtocol,
        token_code: &str,
        claimer_did: &str,
    ) -> Result<SignedMemory> {
        // Find the claim token
        let mut token = self.db.get_claim_token_by_token(token_code)?
            .ok_or_else(|| OcmError::OperationFailed(format!("Claim token '{}' not found", token_code)))?;

        // Attempt to claim the token (this validates expiry and claimed status)
        token.claim(claimer_did)
            .map_err(|e| OcmError::OperationFailed(e))?;

        // Get the original signed memory
        let original_memory = self.db.get_signed_memory(&token.memory_id)?
            .ok_or_else(|| OcmError::OperationFailed("Original memory not found".to_string()))?;

        // Create a new signed memory owned by the claimer (not the organization)
        let mut claimed_memory = SignedMemory::new(claimer_did, "individual", &original_memory.memory_data);
        
        // Sign with claimer's identity
        ocm_protocol.attest_memory(&mut claimed_memory).await?;

        // Store the newly claimed memory
        self.db.create_signed_memory(&claimed_memory)?;

        // Update the token to mark it as claimed
        self.db.update_claim_token(&token)?;

        println!("✅ Successfully claimed record!");
        println!("   Token: {}", token_code);
        println!("   New owner: {}", claimer_did);
        println!("   Memory ID: {}", claimed_memory.id);

        Ok(claimed_memory)
    }

    /// List all proxy records created by an organization
    pub fn list_organization_proxies(&self, organization_did: &str) -> Result<Vec<ProxyMemory>> {
        self.db.list_proxy_memories_by_organization(organization_did)
    }

    /// List all claim tokens created by an organization
    pub fn list_organization_tokens(&self, organization_did: &str) -> Result<Vec<ClaimToken>> {
        self.db.list_claim_tokens_by_organization(organization_did)
    }

    /// Search for proxy records by name (useful for parents looking for their child's record)
    pub fn search_proxy_records(&self, name_pattern: &str) -> Result<Vec<ProxyMemory>> {
        self.db.search_proxy_memories_by_name(name_pattern)
    }

    /// Get statistics about the claim system usage
    pub fn get_claim_statistics(&self, organization_did: &str) -> Result<ClaimStatistics> {
        let tokens = self.list_organization_tokens(organization_did)?;
        let proxies = self.list_organization_proxies(organization_did)?;
        
        let total_tokens = tokens.len();
        let claimed_tokens = tokens.iter().filter(|t| t.is_claimed()).count();
        let expired_tokens = tokens.iter().filter(|t| t.is_expired()).count();
        let active_tokens = total_tokens - claimed_tokens - expired_tokens;

        Ok(ClaimStatistics {
            total_proxy_records: proxies.len(),
            total_tokens_created: total_tokens,
            tokens_claimed: claimed_tokens,
            tokens_expired: expired_tokens,
            tokens_active: active_tokens,
        })
    }
}

#[derive(Debug)]
pub struct ClaimStatistics {
    pub total_proxy_records: usize,
    pub total_tokens_created: usize,
    pub tokens_claimed: usize,
    pub tokens_expired: usize,
    pub tokens_active: usize,
}

impl ClaimStatistics {
    pub fn claim_rate(&self) -> f32 {
        if self.total_tokens_created == 0 {
            0.0
        } else {
            self.tokens_claimed as f32 / self.total_tokens_created as f32 * 100.0
        }
    }
}