mod config;
mod core;
mod identity;
mod networking;
mod persistence;
mod sync;

use config::{init_logging, OcmConfig};
use core::{Individual, OcmError, Result, SignedMemory};
use tracing::{error, info};

use identity::{plc::OcmProtocol, ClaimSystem};
use networking::{OcmNetworking, PeerDiscovery};
use persistence::Database;
use std::sync::Arc;
use sync::SyncManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize configuration
    let config = OcmConfig::from_env().map_err(|e| {
        eprintln!("Failed to load configuration: {}", e);
        e
    })?;

    // Initialize logging
    init_logging(&config)?;

    // Validate configuration
    config.validate()?;

    info!("OCM (Our Collective Memory) Protocol Implementation");
    info!("Starting OCM node with configuration: {:#?}", config);

    // Run the main application with proper error handling
    if let Err(e) = run_ocm_node(config).await {
        error!("OCM node failed: {}", e);
        return Err(e);
    }

    Ok(())
}

async fn run_ocm_node(config: OcmConfig) -> Result<()> {
    info!("Connecting to database: {:?}", config.database.path);
    let db = Database::new(
        config
            .database
            .path
            .as_path()
            .to_str()
            .ok_or_else(|| OcmError::Config("Invalid database path".to_string()))?,
    )?;
    let db_arc = Arc::new(db);
    info!("Database connection established");

    // Test individual CRUD with proper logging
    let test_individual = Individual {
        id: uuid::Uuid::new_v4().to_string(),
        first_name: "Test".to_string(),
        middle_name: None,
        last_name: "User".to_string(),
        dob: None,
        phone: None,
        email: Some("test@example.com".to_string()),
        employer: None,
        updated_on: chrono::Utc::now().to_rfc3339(),
    };

    db_arc.create_individual(&test_individual).map_err(|e| {
        error!("Failed to create test individual: {}", e);
        e
    })?;
    info!(
        "Created individual: {} {}",
        test_individual.first_name, test_individual.last_name
    );

    // Initialize OCM Protocol with Bluesky PLC identity management
    let mut ocm = OcmProtocol::new();
    let identity = ocm.create_identity(Some("ocm-demo".to_string())).await?;
    let identity_did = identity.did.clone();
    println!("Created PLC identity: {}", identity_did);

    // Demonstrate the OCM flow: Capture -> Attestation -> Federation

    // Step 1: Capture - Create a memory from the individual
    let memory_data = serde_json::to_string(&test_individual)?;
    let mut memory = SignedMemory::new(&identity_did, "individual", &memory_data);
    println!("CAPTURE: Created memory with hash: {}", memory.content_hash);

    // Step 2: Attestation - Sign the memory with PLC identity
    ocm.attest_memory(&mut memory).await?;
    println!(
        "ATTESTATION: Signed memory with signature: {:.20}...",
        memory.signature
    );

    // Step 3: Store the signed memory locally (part of federation)
    db_arc.create_signed_memory(&memory)?;
    println!("STORAGE: Stored signed memory in local database");

    // Step 4: Federation - Verify the memory (as if received from a peer)
    let is_valid = ocm.verify_federated_memory(&memory).await?;
    println!("FEDERATION: Memory verification result: {}", is_valid);

    // List all memories from this DID
    let memories = db_arc.list_memories_by_did(&identity_did)?;
    println!(
        "Found {} memories from DID: {}",
        memories.len(),
        identity_did
    );

    // Demonstrate the Claim Token System
    println!("\n🎫 === OCM CLAIM TOKEN SYSTEM DEMO ===");

    // Create a second identity to act as the organization (summer camp)
    let mut camp_identity = ocm
        .create_identity(Some("summer-camp-2024".to_string()))
        .await?;
    let camp_did = camp_identity.did.clone();
    println!("Created camp organization: {}", camp_did);

    // Camp creates a proxy record for a child whose parents haven't signed up yet
    let jamie_data = Individual {
        id: uuid::Uuid::new_v4().to_string(),
        first_name: "Jamie".to_string(),
        middle_name: None,
        last_name: "Smith".to_string(),
        dob: Some("2015-06-15".to_string()),
        phone: None,
        email: None,
        employer: None,
        updated_on: chrono::Utc::now().to_rfc3339(),
    };

    let claim_system = ClaimSystem::new(db_arc.clone());

    let (proxy, claim_token) = claim_system
        .create_proxy_record(
            &mut ocm,
            &camp_did,
            "Jamie Smith",
            Some("Child at Summer Camp 2024, Parent contact: parent@example.com".to_string()),
            &jamie_data,
        )
        .await?;

    println!("Proxy record created for: {}", proxy.proxy_for_name);
    println!("Claim token generated: {}", claim_token.token);

    // Show camp statistics
    let stats = claim_system.get_claim_statistics(&camp_did)?;
    println!("Camp Statistics:");
    println!("   - Total proxy records: {}", stats.total_proxy_records);
    println!("   - Active claim tokens: {}", stats.tokens_active);

    // Now simulate the parent claiming the record
    println!("\n👨‍👩‍👧 Parent Claims Record:");
    let parent_identity = ocm
        .create_identity(Some("jamie-parent".to_string()))
        .await?;
    let parent_did = parent_identity.did.clone();
    println!("👤 Created parent identity: {}", parent_did);

    let claimed_memory = claim_system
        .claim_proxy_record(&mut ocm, &claim_token.token, &parent_did)
        .await?;
    println!(
        "Parent now owns Jamie's data with memory ID: {}",
        claimed_memory.id
    );

    // Verify the parent now has control
    let parent_memories = db_arc.list_memories_by_did(&parent_did)?;
    println!("📚 Parent's memories count: {}", parent_memories.len());

    // Show updated statistics
    let updated_stats = claim_system.get_claim_statistics(&camp_did)?;
    println!("📊 Updated Camp Statistics:");
    println!("   - Tokens claimed: {}", updated_stats.tokens_claimed);
    println!("   - Claim rate: {:.1}%", updated_stats.claim_rate());

    println!("✅ Claim token system demonstration complete!");
    println!("   This enables organizations to create records for individuals");
    println!("   who can later claim ownership and control of their data.");

    // Step 5: Initialize P2P networking for federation
    let networking = OcmNetworking::new(8080, ocm, db_arc.clone());
    let networking_arc = Arc::new(networking);

    // Start the OCM networking server
    networking_arc.start_server().await?;
    println!("🌐 P2P networking layer started on port 8080");

    // Step 6: Initialize peer discovery mechanism
    let discovery = PeerDiscovery::new(
        networking_arc.local_peer_id.clone(),
        8081, // Discovery port
        8080, // OCM networking port
        Some(identity_did.clone()),
    );

    // Start discovery service
    discovery.start_discovery_service().await?;
    println!("🔍 Peer discovery service started on port 8081");

    // Start periodic discovery broadcasting
    discovery.start_periodic_discovery().await?;

    // Add seed peers for initial network bootstrap (if any known peers)
    let seed_peers = vec!["127.0.0.1"]; // Add known peer IPs here
    discovery.add_seed_peers(seed_peers).await?;

    // Connect to any discovered peers
    discovery.connect_discovered_peers(&networking_arc).await?;

    // Step 7: Initialize memory synchronization manager
    let sync_manager = SyncManager::new(
        networking_arc.local_peer_id.clone(), // Dereference to access the field
        db_arc.clone(),                       // Arc clone (cheap pointer copy)
        networking_arc.clone(),               // Arc clone (cheap pointer copy)
    );

    // Start sync service
    sync_manager.start_sync_service().await?;
    println!("🔄 Memory synchronization service started");

    // Initialize CRDT system with existing database memories
    sync_manager.initialize_crdt_from_database().await?;
    println!("🧠 CRDT conflict resolution system initialized");

    // Start heartbeat for peer health monitoring
    networking_arc.start_heartbeat().await?;

    // Demonstrate federation by broadcasting our memory to any connected peers
    networking_arc.broadcast_memory(&memory).await?;
    println!("📡 Memory broadcasted to federation network");

    // Demonstrate CRDT conflict resolution by creating a simulated conflict
    println!("\n🔧 Demonstrating CRDT conflict resolution...");

    // Find the memory we just created to test CRDT operations
    if let Some(stored_memory) = db_arc.list_signed_memories()?.first() {
        let memory_id = stored_memory.id.clone();

        // Simulate concurrent edits to the same memory field
        let update1 = serde_json::json!("Updated from device A");
        let update2 = serde_json::json!("Updated from device B");

        sync_manager
            .update_memory_field(&memory_id, "first_name", update1)
            .await?;
        println!("🔀 Applied update from simulated device A");

        sync_manager
            .update_memory_field(&memory_id, "first_name", update2)
            .await?;
        println!("🔀 Applied update from simulated device B");
    }

    // Display sync and conflict statistics
    let sync_stats = sync_manager.get_sync_statistics().await;
    let conflict_summary = sync_manager.get_conflict_summary().await;

    println!("📊 Advanced Sync Statistics:");
    println!("   - Total peers synced: {}", sync_stats.total_peers_synced);
    println!("   - Database memories: {}", sync_stats.total_memories);
    println!("   - CRDT-managed memories: {}", sync_stats.crdt_memories);
    println!(
        "   - Unresolved conflicts: {}",
        sync_stats.unresolved_conflicts
    );

    if conflict_summary.total_conflicts > 0 {
        println!(
            "⚠️  Detected {} CRDT conflicts in memories: {:?}",
            conflict_summary.total_conflicts, conflict_summary.conflicted_memory_ids
        );
    } else {
        println!("✅ All CRDT operations resolved successfully");
    }

    println!("\n🎉 OCM Protocol demonstration complete!");
    println!("   - Identity created via PLC");
    println!("   - Memory captured and hashed");
    println!("   - Memory signed cryptographically");
    println!("   - Memory stored in local SQLite");
    println!("   - Memory verified for federation");
    println!("   - P2P networking layer initialized");
    println!("   - Peer discovery mechanism active");
    println!("   - Memory synchronization service running");
    println!("   - CRDT conflict resolution implemented");
    println!("   - Ready for distributed multi-device synchronization");

    // Keep the server running
    println!("\n🔗 OCM node is now running:");
    println!("   - P2P connections: 127.0.0.1:8080");
    println!("   - Peer discovery: 127.0.0.1:8081 (UDP)");
    println!("   Use Ctrl+C to stop the node");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("\n👋 OCM node shutting down gracefully");

    Ok(())
}
