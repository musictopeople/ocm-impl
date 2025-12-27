-- Claim tokens allow organizations to create "proxy" records that individuals can later claim
CREATE TABLE claim_token (
    id TEXT PRIMARY KEY,
    token TEXT UNIQUE NOT NULL,  -- Short code like "CAMP2024-JAMIE-AB3C" 
    memory_id TEXT NOT NULL,     -- The signed memory this token can claim
    organization_did TEXT NOT NULL,  -- DID of organization that created the token
    expiry_timestamp TEXT NOT NULL,  -- ISO 8601 timestamp when token expires
    claimed_by_did TEXT,         -- DID that successfully claimed this token (NULL if unclaimed)
    claimed_timestamp TEXT,      -- When the token was claimed
    created_timestamp TEXT NOT NULL,
    updated_on TEXT NOT NULL,
    FOREIGN KEY (memory_id) REFERENCES signed_memory(id) ON DELETE CASCADE
);

-- Efficient lookups
CREATE INDEX idx_claim_token_token ON claim_token(token);
CREATE INDEX idx_claim_token_organization ON claim_token(organization_did);
CREATE INDEX idx_claim_token_expiry ON claim_token(expiry_timestamp);
CREATE INDEX idx_claim_token_claimed_by ON claim_token(claimed_by_did);

-- Proxy memories are memories created by organizations for individuals who don't have OCM yet
-- These get "transferred" to the individual's DID when they claim the token
CREATE TABLE proxy_memory (
    id TEXT PRIMARY KEY,
    proxy_for_name TEXT NOT NULL,    -- "Jamie Smith" - the person this record represents
    proxy_for_info TEXT,             -- Additional identifying info (birthdate, parent contact, etc.)
    organization_did TEXT NOT NULL,  -- Organization that created this proxy
    memory_data TEXT NOT NULL,       -- JSON data about the individual
    created_timestamp TEXT NOT NULL,
    claim_token_id TEXT,             -- Link to the claim token (if one exists)
    FOREIGN KEY (claim_token_id) REFERENCES claim_token(id) ON DELETE SET NULL
);

CREATE INDEX idx_proxy_memory_organization ON proxy_memory(organization_did);
CREATE INDEX idx_proxy_memory_name ON proxy_memory(proxy_for_name);