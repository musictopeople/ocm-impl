# OCM Implementation
OCM (Our Collective Memory) Protocol Implementation

Our Collective Memory (OCM) is a distributed memory protocol that allows individuals to organize themselves into corporeal experiences while maintaining data sovereignty. At its core, OCM enables **selective database synchronization** - your personal data stays in your local database, and you cryptographically sign only specific pieces of information to share with organizations or collectives.

**The Data Flow:**
1. **Personal Database:** You maintain your own encrypted SQLite database (family, preferences, history)
2. **Selective Sharing:** When joining experiences, you share only necessary data ("Jan wants to participate") while keeping sensitive details local
3. **Cryptographic Proof:** All shared data is signed with your decentralized identity, preventing forgery
4. **Collective Memory:** Organizations receive verified participation requests without storing your personal data permanently

This approach breaks the centralized model where organizations collect and monetize personal data. Instead, the individual retains control while still enabling collective coordination.

This implementation leverages core concepts following the principles of "convivial tools", a phrase attributed to Ivan Illich and those in conversation about a divergent path to typical landmarks of technical revolution. This proposal is in no way opinionated beyond a desire for openness, responsible use of resources, and ease of use in a variety of social contexts and environments.

## Architecture Overview

### 1. The Persistence Layer (SQLite) Status: IMPLEMENTED

Each node maintains its own SQLite instance acting as the "Individual Memory."

* **Advantages:** Extremely low latency, works offline, users physically own their data file (`data/ocm-impl.db`)
* **Concurrency:** Uses Write-Ahead Logging (WAL) mode to prevent database locks during concurrent operations
* **Status:** Complete with migrations, CRUD operations, and claim token system

### 2. The Identity & Verification Layer (PLC) Status: PARTIALLY IMPLEMENTED

Public Ledger of Credentials (PLC) acts as the root of trust.

* **Role:** When a memory is created, it is signed using a decentralized identifier (DID) managed via PLC
* **Security:** Cryptographically ties authorship to individuals, preventing data forgery
* **Status:** ED25519 identity creation and signing implemented, network connectivity simulated

### 3. The Logic Engine (Rust) Status: IMPLEMENTED

The Rust core acts as the "Synapse" between local database and the outside world.

* **Concurrency:** Tokio async runtime handles simultaneous peer-to-peer connections
* **Safety:** Memory safety ensures protocol stability during critical collective events
* **Security:** Recent security hardening includes authenticated messaging and secure key storage

### 4. The Web Interface Layer (WebAssembly) Status: IMPLEMENTED

**Complete OPFS + SQLite Integration:** Full browser deployment with persistent storage

* **OPFS Persistence:** SQLite database stored in Origin Private File System, survives browser restarts
* **sql.js Integration:** Complete SQLite operations in browser with professional web interface
* **Zero Install:** Browser-native OCM nodes with cryptographic identity and memory management
* **Status:** Full end-to-end functionality from identity creation to memory storage and retrieval

## Current Implementation Status

### Fully Implemented Features

1. **SQLite Persistence** - Complete CRUD operations with migrations and WAL mode
2. **Cryptographic Identity** - Production-grade ED25519 key generation with secure memory-zeroing storage
3. **Signed Memory System** - SHA256 hashing and cryptographic attestation of data
4. **Claim Token System** - Organizations can create proxy records with claimable tokens (128-bit cryptographically secure)
5. **Network Authentication** - HMAC-SHA256 message authentication with replay protection, rate limiting, and timing attack prevention
6. **P2P Networking Foundation** - Comprehensive TCP server/client with length-prefixed messaging, connection management, and heartbeat protocols
7. **Advanced CRDT Conflict Resolution** - Sophisticated vector clock implementation with operational transforms, LWW, and manual resolution strategies

### Partially Implemented Features

1. **Bluesky PLC Integration** - Complete ED25519 cryptographic implementation with proper PLC DID generation; network API calls implemented but commented out
2. **Database Migrations** - V1 migration complete, but V2 (signed_memory) and V3 (claim_tokens) migrations missing from filesystem
3. **Peer Discovery** - UDP broadcast service exists but needs relay infrastructure for internet-scale deployment

### Recently Completed

1. **Complete OPFS + SQLite Integration** - Full browser persistence with sql.js and OPFS working end-to-end
2. **Production Web Interface** - Professional HTML/CSS/JS frontend with complete database operations
3. **WASM Package Generation** - Complete TypeScript bindings and npm-ready package structure
4. **Database Schema Compatibility** - Browser storage matches native SQLite schema exactly

### Not Yet Implemented

1. **Production Build Automation** - Build system needs npm scripts and deployment automation
2. **Multi-Device Synchronization** - Browser storage works locally, needs cross-device sync
3. **Production PLC Network** - Network connectivity simulated (implementation exists but not connected)
4. **NAT Traversal** - Relay infrastructure for real-world P2P connections through firewalls

## Recent Security Improvements

**Critical vulnerabilities have been addressed:**

- **Fixed weak token generation** - Now uses 128-bit cryptographically secure random
- **Implemented secure private key storage** - Automatic memory zeroing with zeroize crate
- **Added SQL injection protection** - Wildcard escaping in search functions  
- **Enhanced network protocol** - HMAC-SHA256 authentication, replay protection, message size limits
- **Proper base32 encoding** - Fixed PLC ID generation to use RFC4648 standard

## Installation & Usage

### Prerequisites
```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Quick Start

**Native OCM Node:**
```bash
# Initialize database
cargo run --bin migrate

# Start OCM node
cargo run --bin ocm-impl
```

**Browser OCM Interface:**
```bash
# Build WASM package (if not already built)
cd ocm-wasm && wasm-pack build --target web --out-dir pkg

# Start web server  
cargo run --bin web-server

# Open http://127.0.0.1:8000 in browser
# Full OPFS + SQLite persistence now working
```

### Network Configuration
The node will start with:
- **P2P Federation:** TCP connections on `127.0.0.1:8080`
- **Peer Discovery:** UDP broadcasts on `127.0.0.1:8081`
- **Local Database:** SQLite at `data/ocm-impl.db`

### Network Architecture
```
┌─────────────┐    UDP Discovery    ┌─────────────┐
│   OCM Node  │◄──────────────────► │   OCM Node  │
│   :8080     │                     │   :8080     │
│   :8081     │    TCP Federation   │   :8081     │
└─────────────┘◄──────────────────► └─────────────┘
      │                                     │
      ▼                                     ▼
┌─────────────┐                     ┌─────────────┐
│ SQLite DB   │                     │ SQLite DB   │
│ (Personal)  │                     │ (Personal)  │
└─────────────┘                     └─────────────┘
```

## Documentation

**Comprehensive guides available:**
- `docs/API.md` - Complete API documentation with usage examples
- `docs/DEPLOYMENT.md` - Production deployment guide with Docker/Kubernetes configs

## Current Challenges & Next Steps

### Production Readiness Priorities

1. **Fix Race Conditions** - Complete sync manager concurrent operation safety
2. **Input Validation & Rate Limiting** - Add comprehensive validation layers
3. **WebAssembly Compilation** - Enable browser deployment
4. **Real PLC Network Integration** - Connect to actual Bluesky infrastructure
5. **Enhanced Relay Infrastructure** - NAT traversal for real-world P2P

### The "Claim Token" Challenge SOLVED

**Problem:** Organizations need to create proxy records for individuals who don't have OCM yet.

**Solution Implemented:** 
- Organizations can create proxy records with cryptographically secure claim tokens
- Parents/guardians can later claim ownership using these tokens
- Data sovereignty transfers from organization to individual upon claiming

### Missing Pieces

**Conflict Resolution:** CRDT implementation needs completion for offline multi-device synchronization.

**Relay Infrastructure:** Real-world P2P requires discovery nodes and NAT traversal - currently only works on local networks.

## Contributing

This is a proof-of-concept implementation focusing on data sovereignty and decentralized identity. The core cryptographic and persistence layers are production-ready, but networking and UI layers need further development for real-world deployment.

## PDR - Product Design Record

**OCM Protocol v1.0**

**Status:** Concept/Prototyping  
**Core Philosophy:** Convivial Tools / Local-First / Zero-Install

### 1. The Problem Statement

Human coordination currently requires a "choice of evils":

- **Centralized Cloud:** Fast, but requires 24/7 internet and creates "Digital Honeypots" for surveillance
- **Paper/Manual:** Secure and offline, but impossible to audit or scale during a crisis

**OCM Solution:** A "Digital Logbook" that is offline-native, browser-based (no app store), and cryptographically owned by the individual, not the agency.

### 2. Minimum Viable Product (MVP) Features

| Feature | Description | Strategic Value |
|---------|-------------|-----------------|
| Identity Anchor | Integration with Bluesky PLC | Outsources security to a proven public ledger |
| WASM Persistence | SQLite running in the browser via OPFS | Allows 100% offline data entry with no app download |
| The "Handover" | Proxy Record + Claim Token system | Allows NGOs to register people without phones and "transfer" ownership at a later time |
| Delta Sync | Incremental P2P sync (8080/8081) | Only sends what has changed, saving battery and data bandwidth |
| Blind Verification | QR-based "Proof of Eligibility" | NGO verifies a family is "on the list" without seeing their full history |

### 3. User Experience (UX) Flow

Design for the "Three-Second Stress Test" (Field workers have ~3 seconds to make a decision in a crowd).

- **NGO Side:** Open the OCM Web Dashboard → Scan Family QR → Green Checkmark (Verified) → Distribute Aid → Offline Log Updated
- **User Side:** Visit URL → Enter Family Passcode → Show "Ration Token" QR → Receive Receipt

### 4. Technical Constraints & Mitigation

**Constraint:** iOS/Android aggressive browser cache clearing
- **Mitigation:** Use Origin Private File System (OPFS) for "hard" storage that persists even if the user clears history

**Constraint:** No Global Internet for PLC resolution
- **Mitigation:** NGO "Relay Nodes" carry a cached snapshot of the PLC directory to the field

**Constraint:** Conflict Resolution (Two people edit one family record)
- **Mitigation:** Use LWW (Last-Write-Wins) CRDTs for simple fields (name/phone) and Add-Only Sets for distribution logs

  What's Working

  - Core persistence layer - SQLite with WAL mode, migrations, comprehensive CRUD operations
  - Cryptographic identity - Production-grade ED25519 signatures with secure memory management
  - Advanced CRDT system - Vector clocks with operational transforms and conflict resolution
  - Claim token system - Proxy records with cryptographically secure 128-bit tokens
  - Comprehensive P2P networking - TCP federation with authentication, rate limiting, and discovery
  - Security hardening - HMAC-SHA256 authentication, replay protection, timing attack prevention

  Critical Gaps for Production

  1. Security Vulnerabilities (IMMEDIATE PRIORITY)

  Critical Issues:
  - No TLS encryption - All P2P traffic is plaintext
  - Private keys in memory - No HSM/secure hardware integration
  - Hardcoded shared secrets in networking code
  - Minimal input validation on network messages
  - No database encryption at rest

  Required: Full security audit + hardening (3-4 months)

  2. Infrastructure for Real-World Use

  Networking Limitations:
  - No NAT traversal - Only works on local networks
  - UDP broadcast discovery - Doesn't scale beyond LAN
  - No relay infrastructure - Can't connect through firewalls
  - Hardcoded connection limits (50 peers max)

  Missing Production Infrastructure:
  - No container orchestration - Docker configs incomplete
  - No monitoring/metrics - Prometheus planned but not implemented
  - No backup/recovery - Manual database dumps only
  - No load balancing or auto-scaling

  3. Regulatory/Compliance Gaps

  Data Protection:
  - No GDPR compliance - No right to be forgotten, consent management
  - No audit logging - Required for medical/sensitive data
  - No data residency controls - Cross-border data transfer issues
  - No access controls - Basic authentication only

  4. User Experience for Non-Technical Users

  Major UX Challenges:
  - Command-line only - No user-friendly interface
  - JSON editing required - Technical knowledge needed
  - No mobile apps - Web-only approach
  - Complex key management - No backup/recovery UX
  - No accessibility compliance

  Prioritized Next Steps (12-15 Month Roadmap)

  Phase 1: Security & Browser Deployment (4-5 months)

  1. Build production web interface - Professional HTML/CSS/JS frontend complete
  2. Complete OPFS + SQLite integration - Full browser persistence working with sql.js
  3. Implement TLS 1.3 for all network communications
  4. Security audit by external firm

  Phase 2: Real-World Infrastructure (4-5 months)

  1. NAT traversal with STUN/TURN relay servers
  2. Real PLC network integration (connect to Bluesky)
  3. Container orchestration with Kubernetes
  4. Monitoring stack (Prometheus/Grafana)
  5. Backup/disaster recovery procedures

  Phase 3: Production Hardening (3-4 months)

  1. HSM integration for key management
  2. Compliance framework (GDPR/HIPAA)
  3. API development for external integrations
  4. Mobile applications (React Native/Flutter)
  5. Performance optimization and scaling tests

  Critical Success Factors

  Technical Team Needed:
  - Security expert (cryptography + network security)
  - Frontend/WASM specialist (browser deployment)
  - Infrastructure engineer (Kubernetes + monitoring)
  - UX designer (non-technical user experience)

  Estimated Investment: $800K-$1.2M over 12-15 months

  ⚡ Current Status & Next Actions

  1. WASM build complete - All browser compilation working
  2. Complete browser deployment - OPFS + SQLite + web interface functional
  3. Production build automation - Add npm scripts and deployment tools
  4. Security audit - Current networking needs production hardening
  5. Real PLC integration - Move beyond simulated identity
  6. Multi-device sync - Extend browser storage to cross-device synchronization