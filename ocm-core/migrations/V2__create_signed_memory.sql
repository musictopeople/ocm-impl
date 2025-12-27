CREATE TABLE signed_memory (
    id TEXT PRIMARY KEY,
    did TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    memory_data TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    signature TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    updated_on TEXT NOT NULL
);

CREATE INDEX idx_signed_memory_did ON signed_memory(did);
CREATE INDEX idx_signed_memory_timestamp ON signed_memory(timestamp);
CREATE INDEX idx_signed_memory_type ON signed_memory(memory_type);