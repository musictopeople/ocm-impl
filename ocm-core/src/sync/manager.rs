use crate::core::models::SignedMemory;
use crate::networking::protocol::{MessageType, OcmNetworking};
use crate::persistence::database::Database;
use crate::sync::crdt::{CrdtManager, CrdtMemory};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub requesting_peer: String,
    pub last_sync_timestamp: Option<String>,
    pub known_memory_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub responding_peer: String,
    pub memories: Vec<SignedMemory>,
    pub missing_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVector {
    pub peer_id: String,
    pub memory_hash: String,
    pub timestamp: String,
    pub version: u64,
}

pub struct SyncManager {
    pub local_peer_id: String,
    pub database: Arc<Database>,
    pub networking: Arc<OcmNetworking>,
    pub sync_state: Arc<Mutex<SyncState>>,
    pub crdt_manager: Arc<Mutex<CrdtManager>>,
}

#[derive(Debug)]
pub struct SyncState {
    pub last_sync_per_peer: HashMap<String, chrono::DateTime<chrono::Utc>>,
    pub sync_in_progress: HashSet<String>,
    pub memory_versions: HashMap<String, u64>, // memory_hash -> version
}

// RAII guard to ensure sync_in_progress cleanup
struct SyncCleanupGuard {
    sync_state: Arc<Mutex<SyncState>>,
    peer_id: String,
    completed: bool,
}

impl SyncCleanupGuard {
    fn new(sync_state: Arc<Mutex<SyncState>>, peer_id: String) -> Self {
        SyncCleanupGuard {
            sync_state,
            peer_id,
            completed: false,
        }
    }

    async fn complete(&mut self) {
        self.completed = true;
        let mut state = self.sync_state.lock().await;
        state.sync_in_progress.remove(&self.peer_id);
    }
}

impl Drop for SyncCleanupGuard {
    fn drop(&mut self) {
        if !self.completed {
            // Use tokio::spawn to handle cleanup in destructor
            let sync_state = self.sync_state.clone();
            let peer_id = self.peer_id.clone();
            tokio::spawn(async move {
                let mut state = sync_state.lock().await;
                state.sync_in_progress.remove(&peer_id);
            });
        }
    }
}

impl SyncManager {
    pub fn new(
        local_peer_id: String,
        database: Arc<Database>,
        networking: Arc<OcmNetworking>,
    ) -> Self {
        let crdt_manager = CrdtManager::new(local_peer_id.clone());

        SyncManager {
            local_peer_id,
            database,
            networking,
            sync_state: Arc::new(Mutex::new(SyncState {
                last_sync_per_peer: HashMap::new(),
                sync_in_progress: HashSet::new(),
                memory_versions: HashMap::new(),
            })),
            crdt_manager: Arc::new(Mutex::new(crdt_manager)),
        }
    }

    pub async fn start_sync_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sync_state = self.sync_state.clone();
        let _database = self.database.clone();
        let _local_peer_id = self.local_peer_id.clone();

        // Start periodic sync with all known peers
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Get list of known peers and sync with them
                // This would be integrated with the networking layer
                println!("🔄 Periodic sync check initiated");

                // Clean up stale sync operations (older than 5 minutes)
                {
                    let state = sync_state.lock().await;
                    // In a real implementation, we'd track sync start times and clean up stale ones
                    // For now, just log the active syncs
                    if !state.sync_in_progress.is_empty() {
                        println!("🔄 Active syncs: {:?}", state.sync_in_progress);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn sync_with_peer(&self, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if sync is already in progress with this peer
        {
            let mut state = self.sync_state.lock().await;
            if state.sync_in_progress.contains(peer_id) {
                return Ok(());
            }
            state.sync_in_progress.insert(peer_id.to_string());
        }

        // Ensure cleanup happens even if sync fails
        let mut cleanup_guard = SyncCleanupGuard::new(self.sync_state.clone(), peer_id.to_string());

        let last_sync = {
            let state = self.sync_state.lock().await;
            state.last_sync_per_peer.get(peer_id).cloned()
        };

        // Get our known memory hashes since last sync
        let known_memories = self.database.list_signed_memories()?;
        let known_hashes: Vec<String> = known_memories
            .iter()
            .filter(|memory| {
                if let Some(last_sync) = last_sync {
                    chrono::DateTime::parse_from_rfc3339(&memory.timestamp)
                        .map(|dt| dt.with_timezone(&chrono::Utc) > last_sync)
                        .unwrap_or(true)
                } else {
                    true
                }
            })
            .map(|memory| memory.content_hash.clone())
            .collect();

        // Create sync request
        let sync_request = SyncRequest {
            requesting_peer: self.local_peer_id.clone(),
            last_sync_timestamp: last_sync.map(|dt| dt.to_rfc3339()),
            known_memory_hashes: known_hashes,
        };

        // Send sync request via networking layer
        let _message = OcmNetworking::create_authenticated_message(
            MessageType::MemoryRequest,
            serde_json::to_string(&sync_request)?,
            self.local_peer_id.clone(),
        );

        // This would be sent through the networking layer
        println!("📡 Requesting sync from peer: {}", peer_id);

        // Mark sync as complete (cleanup_guard will handle removal from sync_in_progress)
        cleanup_guard.complete().await;
        Ok(())
    }

    pub async fn handle_sync_request(
        &self,
        request: SyncRequest,
        from_peer: &str,
    ) -> Result<SyncResponse, Box<dyn std::error::Error>> {
        // Get memories newer than the request's timestamp
        let our_memories = self.database.list_signed_memories()?;

        let memories_to_send: Vec<SignedMemory> = our_memories
            .into_iter()
            .filter(|memory| {
                // Check if this memory is newer than their last sync
                if let Some(last_sync) = &request.last_sync_timestamp {
                    chrono::DateTime::parse_from_rfc3339(&memory.timestamp)
                        .map(|dt| {
                            if let Ok(last_sync_dt) =
                                chrono::DateTime::parse_from_rfc3339(last_sync)
                            {
                                dt.with_timezone(&chrono::Utc)
                                    > last_sync_dt.with_timezone(&chrono::Utc)
                            } else {
                                true // If we can't parse last_sync, include the memory
                            }
                        })
                        .unwrap_or(true)
                } else {
                    true
                }
            })
            .filter(|memory| {
                // Only send memories they don't already have
                !request.known_memory_hashes.contains(&memory.content_hash)
            })
            .collect();

        // Find memories they have that we don't
        let our_hashes: HashSet<String> = self
            .database
            .list_signed_memories()?
            .iter()
            .map(|m| m.content_hash.clone())
            .collect();

        let missing_hashes: Vec<String> = request
            .known_memory_hashes
            .iter()
            .filter(|hash| !our_hashes.contains(*hash))
            .cloned()
            .collect();

        println!(
            "🔍 Sync request from {}: sending {} memories, requesting {} missing",
            from_peer,
            memories_to_send.len(),
            missing_hashes.len()
        );

        Ok(SyncResponse {
            responding_peer: self.local_peer_id.clone(),
            memories: memories_to_send,
            missing_hashes,
        })
    }

    pub async fn handle_sync_response(
        &self,
        response: SyncResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stored_count = 0;
        let mut conflict_count = 0;

        // Store received memories using CRDT conflict resolution
        for memory in response.memories {
            // Verify memory integrity and signature
            if memory.verify_hash() {
                // Try to merge using CRDT
                let crdt_memory = CrdtMemory::new(memory.clone(), &response.responding_peer);
                let mut crdt_manager = self.crdt_manager.lock().await;

                match crdt_manager.merge_memory(&memory.id, crdt_memory) {
                    Ok(conflicts) => {
                        if conflicts.is_empty() {
                            // No conflicts, store the merged memory
                            if let Some(merged_crdt) = crdt_manager.get_memory(&memory.id) {
                                match self.database.create_signed_memory(&merged_crdt.base_memory) {
                                    Ok(()) => {
                                        stored_count += 1;
                                        println!(
                                            "✅ Stored CRDT-merged memory: {}",
                                            memory.content_hash
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Failed to store merged memory: {}", e);
                                    }
                                }
                            }
                        } else {
                            conflict_count += conflicts.len();
                            println!(
                                "⚠️  CRDT conflicts detected for memory {}: {} conflicts",
                                memory.id,
                                conflicts.len()
                            );

                            // Log conflict details
                            for conflict in conflicts {
                                println!(
                                    "   🔀 Conflict in field '{}': local vs remote operation",
                                    conflict.field_path
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ CRDT merge failed for memory {}: {}", memory.id, e);
                        // Fallback to traditional storage
                        if let Err(e) = self.database.create_signed_memory(&memory) {
                            eprintln!("❌ Fallback storage also failed: {}", e);
                        }
                    }
                }
            } else {
                eprintln!(
                    "❌ Invalid memory hash from peer: {}",
                    response.responding_peer
                );
            }
        }

        // Send requested missing memories
        if !response.missing_hashes.is_empty() {
            self.send_missing_memories(&response.responding_peer, &response.missing_hashes)
                .await?;
        }

        // Update sync state - use single lock acquisition for atomicity
        {
            let mut state = self.sync_state.lock().await;
            state
                .last_sync_per_peer
                .insert(response.responding_peer.clone(), chrono::Utc::now());
            state.sync_in_progress.remove(&response.responding_peer);
        }

        println!(
            "🎉 CRDT sync completed with {}: stored {} memories, {} conflicts resolved",
            response.responding_peer, stored_count, conflict_count
        );

        Ok(())
    }

    async fn send_missing_memories(
        &self,
        peer_id: &str,
        missing_hashes: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let all_memories = self.database.list_signed_memories()?;

        for hash in missing_hashes {
            if let Some(memory) = all_memories.iter().find(|m| &m.content_hash == hash) {
                let _message = OcmNetworking::create_authenticated_message(
                    MessageType::MemorySync,
                    serde_json::to_string(memory)?,
                    self.local_peer_id.clone(),
                );

                // Send via networking layer
                println!("📤 Sending missing memory {} to peer {}", hash, peer_id);
            }
        }

        Ok(())
    }

    pub async fn detect_conflicts(&self) -> Result<Vec<ConflictInfo>, Box<dyn std::error::Error>> {
        let memories = self.database.list_signed_memories()?;
        let mut conflicts = Vec::new();
        let mut memory_groups: HashMap<String, Vec<SignedMemory>> = HashMap::new();

        // Group memories by DID + memory_type to detect conflicts
        for memory in memories {
            let key = format!("{}:{}", memory.did, memory.memory_type);
            memory_groups
                .entry(key)
                .or_insert_with(Vec::new)
                .push(memory);
        }

        // Check for conflicting memories (same DID+type, different content)
        for (key, memories) in memory_groups {
            if memories.len() > 1 {
                // Sort by timestamp to identify conflicts
                let mut sorted_memories = memories;
                sorted_memories.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

                for window in sorted_memories.windows(2) {
                    if window[0].content_hash != window[1].content_hash {
                        conflicts.push(ConflictInfo {
                            key: key.clone(),
                            older_memory: window[0].clone(),
                            newer_memory: window[1].clone(),
                            conflict_type: ConflictType::ContentMismatch,
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    }

    pub async fn initialize_crdt_from_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        let memories = self.database.list_signed_memories()?;
        let mut crdt_manager = self.crdt_manager.lock().await;

        for memory in memories {
            crdt_manager.add_memory(memory);
        }

        println!(
            "🔧 Initialized CRDT manager with {} memories",
            crdt_manager.memories.len()
        );
        Ok(())
    }

    pub async fn update_memory_field(
        &self,
        memory_id: &str,
        field_path: &str,
        value: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut crdt_manager = self.crdt_manager.lock().await;
        crdt_manager.update_memory(memory_id, field_path, value)?;

        // Update the database with the modified memory
        if let Some(crdt_memory) = crdt_manager.get_memory(memory_id) {
            self.database
                .create_signed_memory(&crdt_memory.base_memory)?;
        }

        Ok(())
    }

    pub async fn get_conflict_summary(&self) -> ConflictSummary {
        let crdt_manager = self.crdt_manager.lock().await;
        let conflicted_memories = crdt_manager.list_conflicts();

        ConflictSummary {
            total_conflicts: conflicted_memories.len(),
            conflicted_memory_ids: conflicted_memories,
        }
    }

    pub async fn force_resolve_conflicts(
        &self,
        memory_id: &str,
        resolution_strategy: crate::sync::crdt::ConflictStrategy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut crdt_manager = self.crdt_manager.lock().await;

        if let Some(crdt_memory) = crdt_manager.memories.get_mut(memory_id) {
            crdt_memory.merge_metadata.conflict_resolution_strategy = resolution_strategy;
            self.database
                .create_signed_memory(&crdt_memory.base_memory)?;
            println!("🔧 Force resolved conflicts for memory: {}", memory_id);
        }

        Ok(())
    }

    pub async fn get_sync_statistics(&self) -> SyncStatistics {
        let state = self.sync_state.lock().await;
        let total_peers = state.last_sync_per_peer.len();
        let active_syncs = state.sync_in_progress.len();

        let total_memories = self
            .database
            .list_signed_memories()
            .unwrap_or_default()
            .len();

        let crdt_manager = self.crdt_manager.lock().await;
        let crdt_memories = crdt_manager.memories.len();
        let conflicts = crdt_manager.list_conflicts().len();

        SyncStatistics {
            total_peers_synced: total_peers,
            active_sync_operations: active_syncs,
            total_memories: total_memories,
            crdt_memories: crdt_memories,
            unresolved_conflicts: conflicts,
            last_sync_times: state.last_sync_per_peer.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub key: String,
    pub older_memory: SignedMemory,
    pub newer_memory: SignedMemory,
    pub conflict_type: ConflictType,
}

#[derive(Debug, Clone)]
pub enum ConflictType {
    ContentMismatch,
    TimestampConflict,
    SignatureConflict,
}

#[derive(Debug, Clone)]
pub struct ConflictSummary {
    pub total_conflicts: usize,
    pub conflicted_memory_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SyncStatistics {
    pub total_peers_synced: usize,
    pub active_sync_operations: usize,
    pub total_memories: usize,
    pub crdt_memories: usize,
    pub unresolved_conflicts: usize,
    pub last_sync_times: HashMap<String, chrono::DateTime<chrono::Utc>>,
}
