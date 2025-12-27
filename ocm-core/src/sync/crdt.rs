use crate::core::models::SignedMemory;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorClock {
    pub clock: BTreeMap<String, u64>, // peer_id -> logical_clock
}

impl VectorClock {
    pub fn new() -> Self {
        VectorClock {
            clock: BTreeMap::new(),
        }
    }

    pub fn increment(&mut self, peer_id: &str) {
        let current = self.clock.get(peer_id).unwrap_or(&0);
        self.clock.insert(peer_id.to_string(), current + 1);
    }

    pub fn update(&mut self, other: &VectorClock) {
        for (peer_id, &timestamp) in &other.clock {
            self.clock
                .entry(peer_id.clone())
                .and_modify(|current| *current = (*current).max(timestamp))
                .or_insert(timestamp);
        }
    }

    pub fn compare(&self, other: &VectorClock) -> ClockOrdering {
        let mut self_less = false;
        let mut other_less = false;

        // We use peekable iterators to walk both sorted maps at once
        let mut it1 = self.clock.iter().peekable();
        let mut it2 = other.clock.iter().peekable();

        loop {
            match (it1.peek(), it2.peek()) {
                // Case 1: Both clocks have the same Peer ID
                (Some((k1, v1)), Some((k2, v2))) if k1 == k2 => {
                    if v1 < v2 {
                        self_less = true;
                    } else if v1 > v2 {
                        other_less = true;
                    }
                    it1.next();
                    it2.next();
                }
                // Case 2: Self has a Peer ID that Other doesn't have
                (Some((k1, v1)), Some((k2, _))) if k1 < k2 => {
                    if **v1 > 0 {
                        other_less = true;
                    }
                    it1.next();
                }
                // Case 3: Other has a Peer ID that Self doesn't have
                (Some(_), Some((_k2, v2))) => {
                    if **v2 > 0 {
                        self_less = true;
                    }
                    it2.next();
                }
                // Case 4: Self has remaining Peer IDs after Other is exhausted
                (Some((_, v1)), None) => {
                    if **v1 > 0 {
                        other_less = true;
                    }
                    it1.next();
                }
                // Case 5: Other has remaining Peer IDs after Self is exhausted
                (None, Some((_, v2))) => {
                    if **v2 > 0 {
                        self_less = true;
                    }
                    it2.next();
                }
                // Case 6: Both iterators exhausted
                (None, None) => break,
            }
        }

        match (self_less, other_less) {
            (false, false) => ClockOrdering::Equal,
            (true, false) => ClockOrdering::Less,
            (false, true) => ClockOrdering::Greater,
            (true, true) => ClockOrdering::Concurrent,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClockOrdering {
    Less,       // self < other (self happened before other)
    Greater,    // self > other (self happened after other)
    Equal,      // self == other (same state)
    Concurrent, // neither < nor > (concurrent/conflicting)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtMemory {
    pub base_memory: SignedMemory,
    pub vector_clock: VectorClock,
    pub operations: Vec<MemoryOperation>,
    // New: O(1) lookup index for operation IDs (not serialized)
    #[serde(skip)]
    pub operation_index: HashSet<String>,
    pub merge_metadata: MergeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperation {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub field_path: String,
    pub value: serde_json::Value,
    pub vector_clock: VectorClock,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Set,    // Set field to value
    Delete, // Delete field
    Append, // Append to array/string
    Merge,  // Merge objects
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeMetadata {
    pub merged_from: Vec<String>, // List of peer IDs that contributed to this memory
    pub conflict_resolution_strategy: ConflictStrategy,
    pub last_merge_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictStrategy {
    LastWriterWins,       // Use timestamp to resolve conflicts
    OperationalTransform, // Use operational transformation
    ManualResolution,     // Require manual conflict resolution
}

impl CrdtMemory {
    pub fn new(base_memory: SignedMemory, peer_id: &str) -> Self {
        let mut vector_clock = VectorClock::new();
        vector_clock.increment(peer_id);

        CrdtMemory {
            base_memory,
            vector_clock,
            operations: Vec::new(),
            operation_index: HashSet::new(),
            merge_metadata: MergeMetadata {
                merged_from: vec![peer_id.to_string()],
                conflict_resolution_strategy: ConflictStrategy::LastWriterWins,
                last_merge_timestamp: chrono::Utc::now().to_rfc3339(),
            },
        }
    }

    /// Essential for restoring the index after deserialization
    pub fn rebuild_index(&mut self) {
        // We use &self.operations to borrow, then collect into the HashSet
        self.operation_index = self.operations
            .as_slice() // The "Magic" fix for your error
            .iter()
            .map(|op| op.operation_id.clone())
            .collect();
    }

    pub fn apply_operation(
        &mut self,
        operation: MemoryOperation,
        peer_id: &str,
    ) -> Result<(), CrdtError> {
        // Prevent duplicate application
        if self.operation_index.contains(&operation.operation_id) {
            return Ok(());
        }

        self.vector_clock.update(&operation.vector_clock);
        self.vector_clock.increment(peer_id);

        match operation.operation_type {
            OperationType::Set => self.apply_set_operation(&operation)?,
            OperationType::Delete => self.apply_delete_operation(&operation)?,
            OperationType::Append => self.apply_append_operation(&operation)?,
            OperationType::Merge => self.apply_merge_operation(&operation)?,
        }

        self.operation_index.insert(operation.operation_id.clone());
        self.operations.push(operation);
        self.merge_metadata.last_merge_timestamp = chrono::Utc::now().to_rfc3339();

        Ok(())
    }

    fn apply_set_operation(&mut self, operation: &MemoryOperation) -> Result<(), CrdtError> {
        let mut memory_data: serde_json::Value =
            serde_json::from_str(&self.base_memory.memory_data)
                .map_err(|_| CrdtError::InvalidMemoryData)?;

        let path_parts: Vec<&str> = operation.field_path.split('.').collect();
        let mut current = &mut memory_data;
        let val_to_insert = operation.value.clone();

        for (i, part) in path_parts.as_slice().iter().enumerate() {
            let is_last = i == path_parts.len() - 1;

            if let serde_json::Value::Object(obj) = current {
                if is_last {
                    obj.insert(part.to_string(), val_to_insert);
                    break;
                } else {
                    // Entry API: single lookup for navigate-or-create
                    current = obj
                        .entry(part.to_string())
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                }
            } else {
                return Err(CrdtError::OperationFailed(format!(
                    "Path '{}' is not an object",
                    part
                )));
            }
        }

        self.finalize_change(memory_data);
        Ok(())
    }

    fn apply_append_operation(&mut self, operation: &MemoryOperation) -> Result<(), CrdtError> {
        let mut memory_data: serde_json::Value = serde_json::from_str::<serde_json::Value>(&self.base_memory.memory_data[..])
            .map_err(|_| CrdtError::InvalidMemoryData)?;

        let path_parts: Vec<&str> = operation.field_path.split('.').collect();
        let mut current = &mut memory_data;

        // Navigate to the target field
        for part in &path_parts {
            if let serde_json::Value::Object(obj) = current {
                current = obj.entry(part.to_string()).or_insert(serde_json::Value::Null);
            }
        }

        // Perform the append based on type
        match current {
            serde_json::Value::Array(arr) => {
                arr.push(operation.value.clone());
            }
            serde_json::Value::String(s) => {
                if let Some(to_append) = operation.value.as_str() {
                    s.push_str(to_append);
                }
            }
            _ => return Err(CrdtError::OperationFailed("Target is not appendable".to_string())),
        }

        self.finalize_change(memory_data);
        Ok(())
    }

    fn apply_delete_operation(&mut self, operation: &MemoryOperation) -> Result<(), CrdtError> {
        let mut memory_data: serde_json::Value = serde_json::from_str(&self.base_memory.memory_data)
            .map_err(|_| CrdtError::InvalidMemoryData)?;

        let path_parts: Vec<&str> = operation.field_path.split('.').collect();
        let mut current = &mut memory_data;

        for (i, part) in path_parts.iter().enumerate() {
            let is_last = i == path_parts.len() - 1;

            if let serde_json::Value::Object(obj) = current {
                if is_last {
                    obj.remove(*part);
                    break;
                } else if let Some(next) = obj.get_mut(*part) {
                    current = next;
                } else {
                    return Ok(()); // Path doesn't exist, deletion is "complete"
                }
            }
        }

        self.finalize_change(memory_data);
        Ok(())
    }

    fn apply_merge_operation(&mut self, operation: &MemoryOperation) -> Result<(), CrdtError> {
        let mut memory_data: serde_json::Value = serde_json::from_str(&self.base_memory.memory_data)
            .map_err(|_| CrdtError::InvalidMemoryData)?;

        let path_parts: Vec<&str> = operation.field_path.split('.').collect();
        let mut current = &mut memory_data;

        // Navigate to the target field
        for (i, part) in path_parts.iter().enumerate() {
            let is_last = i == path_parts.len() - 1;

            if let serde_json::Value::Object(obj) = current {
                if is_last {
                    // Merge the operation value with the existing value
                    let existing_value = obj.get(*part).cloned().unwrap_or(serde_json::Value::Null);
                    let merged_value = self.merge_json_values(existing_value, operation.value.clone())?;
                    obj.insert(part.to_string(), merged_value);
                    break;
                } else {
                    current = obj
                        .entry(part.to_string())
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                }
            } else {
                return Err(CrdtError::OperationFailed(format!(
                    "Path '{}' is not an object",
                    part
                )));
            }
        }

        self.finalize_change(memory_data);
        Ok(())
    }

    fn merge_json_values(&self, existing: serde_json::Value, new: serde_json::Value) -> Result<serde_json::Value, CrdtError> {
        match (existing, new) {
            (serde_json::Value::Object(mut existing_obj), serde_json::Value::Object(new_obj)) => {
                // Merge objects by combining their properties
                for (key, value) in new_obj {
                    existing_obj.insert(key, value);
                }
                Ok(serde_json::Value::Object(existing_obj))
            }
            (serde_json::Value::Array(mut existing_arr), serde_json::Value::Array(new_arr)) => {
                // Merge arrays by concatenating them
                existing_arr.extend(new_arr);
                Ok(serde_json::Value::Array(existing_arr))
            }
            (_, new_value) => {
                // For other types, the new value overwrites the existing value
                Ok(new_value)
            }
        }
    }

    // Helper to dry up the hash/timestamp update logic
    fn finalize_change(&mut self, data: serde_json::Value) {
        self.base_memory.memory_data = data.to_string();
        self.base_memory.content_hash = SignedMemory::compute_hash(&self.base_memory.memory_data);
        self.base_memory.updated_on = chrono::Utc::now().to_rfc3339();
    }

    pub fn merge_with(
        &mut self,
        other: &CrdtMemory,
        peer_id: &str,
    ) -> Result<Vec<ConflictInfo>, CrdtError> {
        let mut conflicts = Vec::new();

        match self.vector_clock.compare(&other.vector_clock) {
            ClockOrdering::Less => {
                for operation in &other.operations {
                    // HashSet lookup is O(1)
                    if !self.operation_index.contains(&operation.operation_id) {
                        self.apply_operation(operation.clone(), peer_id)?;
                    }
                }
            }
            ClockOrdering::Greater | ClockOrdering::Equal => return Ok(conflicts),
            ClockOrdering::Concurrent => {
                conflicts = self.resolve_concurrent_operations(other, peer_id)?;
            }
        }

        self.merge_metadata
            .merged_from
            .extend(other.merge_metadata.merged_from.clone());
        self.merge_metadata.merged_from.sort_unstable();
        self.merge_metadata.merged_from.dedup();

        Ok(conflicts)
    }

    fn has_operation(&self, operation_id: &str) -> bool {
        self.operations.as_slice()
            .iter()
            .any(|op| op.operation_id == operation_id)
    }

    fn resolve_concurrent_operations(
        &mut self,
        other: &CrdtMemory,
        peer_id: &str,
    ) -> Result<Vec<ConflictInfo>, CrdtError> {
        let mut conflicts = Vec::new();

        // Find operations that conflict (same field path, different values)
        for other_op in &other.operations {
            if self.has_operation(&other_op.operation_id.as_str()) {
                continue; // Already applied this operation
            }

            // Check if there's a conflicting operation
            let conflicting_ops: Vec<&MemoryOperation> = self
                .operations.as_slice()
                .iter()
                .filter(|op| op.field_path == other_op.field_path && op.value != other_op.value)
                .collect();

            if !conflicting_ops.is_empty() {
                // Handle conflict based on strategy
                match self.merge_metadata.conflict_resolution_strategy {
                    ConflictStrategy::LastWriterWins => {
                        // Compare timestamps
                        let other_time = chrono::DateTime::parse_from_rfc3339(&other_op.timestamp)
                            .map_err(|_| CrdtError::InvalidTimestamp)?;

                        let mut should_apply = true;
                        for our_op in &conflicting_ops {
                            let our_time = chrono::DateTime::parse_from_rfc3339(&our_op.timestamp)
                                .map_err(|_| CrdtError::InvalidTimestamp)?;
                            if our_time >= other_time {
                                should_apply = false;
                                break;
                            }
                        }

                        if should_apply {
                            self.apply_operation(other_op.clone(), peer_id)?;
                        }
                    }
                    ConflictStrategy::OperationalTransform => {
                        // Apply operational transformation
                        let transformed_op =
                            self.transform_operation(other_op, &conflicting_ops)?;
                        self.apply_operation(transformed_op, peer_id)?;
                    }
                    ConflictStrategy::ManualResolution => {
                        // Record conflict for manual resolution
                        conflicts.push(ConflictInfo {
                            field_path: other_op.field_path.clone(),
                            local_operation: conflicting_ops[0].clone(),
                            remote_operation: other_op.clone(),
                            conflict_type: super::manager::ConflictType::ContentMismatch,
                        });
                    }
                }
            } else {
                // No conflict, apply the operation
                self.apply_operation(other_op.clone(), peer_id)?;
            }
        }

        Ok(conflicts)
    }

    fn transform_operation(
        &self,
        operation: &MemoryOperation,
        conflicting_ops: &[&MemoryOperation],
    ) -> Result<MemoryOperation, CrdtError> {
        // Simple operational transform - could be more sophisticated
        let mut transformed = operation.clone();

        match operation.operation_type {
            OperationType::Set => {
                // For set operations, use a combination strategy
                if let (serde_json::Value::String(remote), serde_json::Value::String(local)) =
                    (&operation.value, &conflicting_ops[0].value)
                {
                    // Combine strings
                    transformed.value =
                        serde_json::Value::String(format!("{} | {}", local, remote));
                }
            }
            OperationType::Append => {
                // Append operations are naturally commutative
            }
            OperationType::Delete => {
                // Delete operations need careful handling
                // For now, defer to manual resolution
            }
            OperationType::Merge => {
                // Merge operations can be combined
            }
        }

        Ok(transformed)
    }
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub field_path: String,
    pub local_operation: MemoryOperation,
    pub remote_operation: MemoryOperation,
    pub conflict_type: super::manager::ConflictType,
}

#[derive(Debug, Clone)]
pub enum CrdtError {
    InvalidMemoryData,
    InvalidTimestamp,
    OperationFailed(String),
}

impl std::fmt::Display for CrdtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrdtError::InvalidMemoryData => write!(f, "Invalid memory data format"),
            CrdtError::InvalidTimestamp => write!(f, "Invalid timestamp format"),
            CrdtError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for CrdtError {}

#[derive(Debug)]
pub struct CrdtManager {
    pub peer_id: String,
    pub memories: HashMap<String, CrdtMemory>, // memory_id -> CrdtMemory
}

impl CrdtManager {
    pub fn new(peer_id: String) -> Self {
        CrdtManager {
            peer_id,
            memories: HashMap::new(),
        }
    }

    pub fn add_memory(&mut self, memory: SignedMemory) -> String {
        let memory_id = memory.id.clone();
        let crdt_memory = CrdtMemory::new(memory, &self.peer_id);
        self.memories.insert(memory_id.clone(), crdt_memory);
        memory_id
    }

    pub fn update_memory(
        &mut self,
        memory_id: &str,
        field_path: &str,
        value: serde_json::Value,
    ) -> Result<(), CrdtError> {
        if let Some(crdt_memory) = self.memories.get_mut(memory_id) {
            let operation = MemoryOperation {
                operation_id: uuid::Uuid::new_v4().to_string(),
                operation_type: OperationType::Set,
                field_path: field_path.to_string(),
                value,
                vector_clock: {
                    let mut clock = crdt_memory.vector_clock.clone();
                    clock.increment(&self.peer_id);
                    clock
                },
                timestamp: chrono::Utc::now().to_rfc3339(),
            };

            crdt_memory.apply_operation(operation, &self.peer_id)?;
        }

        Ok(())
    }

    pub fn merge_memory(
        &mut self,
        memory_id: &str,
        remote_memory: CrdtMemory,
    ) -> Result<Vec<ConflictInfo>, CrdtError> {
        if let Some(local_memory) = self.memories.get_mut(memory_id) {
            local_memory.merge_with(&remote_memory, &self.peer_id.as_str())
        } else {
            // New memory from remote peer
            self.memories.insert(memory_id.to_string(), remote_memory);
            Ok(Vec::new())
        }
    }

    pub fn get_memory(&self, memory_id: &str) -> Option<&CrdtMemory> {
        self.memories.get(memory_id)
    }

    pub fn list_conflicts(&self) -> Vec<String> {
        // Return list of memory IDs that have unresolved conflicts
        self.memories
            .iter()
            .filter(|(_, memory)| {
                memory.merge_metadata.conflict_resolution_strategy
                    == ConflictStrategy::ManualResolution
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
