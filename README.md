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

## 🚀 Live Demo - See OCM in Action

**Experience the full OCM stack with real-time synchronization between browser tabs.**

### Quick Start (2 minutes)

```bash
# Clone and setup
git clone <repo-url> && cd ocm-impl
npm install

# Start the OCM web server
cargo run -p ocm-core --bin web-server
```

Then open **multiple browser tabs** to:
- 🌐 **http://127.0.0.1:8000** (OCM web interface)

### What You'll See

1. **🔐 Cryptographic Identity Creation**
   - Click "Create Identity" to generate Ed25519 keypair
   - Your DID (Decentralized Identifier) appears instantly
   - Private keys stored securely in browser's WebCrypto API

2. **💾 Local-First Data Storage** 
   - Create memories with the "Add Memory" form
   - Data persists in SQLite database via browser's OPFS
   - Refresh the page - your data survives browser restarts

3. **⚡ Real-Time P2P Synchronization**
   - Open multiple browser tabs to the same URL
   - Create a memory in one tab → **instantly appears in others**
   - No central server storing your data - it's pure P2P

4. **🛡️ Production-Grade Security**
   - TLS 1.3 encryption for all communications
   - Input validation prevents XSS/injection attacks  
   - Rate limiting protects against abuse
   - Security headers ensure browser protection

### Architecture in Action

```
Browser Tab A ──┐    WebSocket     ┌── Browser Tab B
    │           │    Relay         │       │
    │           └──► :8082 ◄────────┘       │
    │                                       │
    ▼                                       ▼
 SQLite OPFS                          SQLite OPFS
 (Your Data)                         (Your Data)
```

### Behind the Demo

- **🦀 Rust Core**: SQLite operations, Ed25519 signing, CRDT sync logic
- **🌐 WebAssembly**: Rust compiled to run in browser with zero installation
- **📡 WebSocket Relay**: Real-time communication server for P2P messaging
- **🔒 Security Stack**: TLS, validation, rate limiting, audit logging

## Current Implementation Status

### ✅ Production Ready

1. **🏗️ Core OCM Protocol** - Identity, memory storage, cryptographic signing
2. **🌐 Browser Deployment** - Full WebAssembly + OPFS SQLite integration  
3. **📡 Real-Time Sync** - WebSocket relay enables instant tab synchronization
4. **🛡️ Enterprise Security** - TLS 1.3, input validation, rate limiting, audit logs
5. **⚙️ Build Automation** - Complete npm scripts for dev and production

### 🚧 Next Phase

1. **📱 Cross-Device Sync** - Extend relay network for phone ↔ desktop
2. **🌍 Production PLC Network** - Connect to decentralized identity network
3. **🚀 Mobile Apps** - React Native + WebAssembly for native mobile
4. **🔗 Federation** - Relay network with multiple server nodes

### Production Stack Architecture

```
🌐 Browser Tabs (Multiple)
    │
    │ HTTPS (TLS 1.3)
    ▼
🔒 Secure Web Server (:8443)
    │ Security Middleware
    │ ├── Rate Limiting  
    │ ├── Input Validation
    │ ├── Security Headers
    │ └── Audit Logging
    │
    ▼
📦 WebAssembly OCM Core
    │ ├── Ed25519 Signing
    │ ├── SQLite Operations 
    │ └── CRDT Sync Logic
    │
    ├─── WebSocket ────► 📡 Relay Server (:8082)
    │                      │
    └─── Storage ────────► 💾 OPFS SQLite
                             (Persistent)
```

### Security Features

- **🔐 Transport Security**: TLS 1.3 with automatic HTTP→HTTPS redirect
- **🛡️ Input Protection**: XSS/injection prevention with comprehensive validation
- **⚡ Rate Limiting**: IP-based DDoS protection with configurable burst limits  
- **📋 Security Headers**: CSP, HSTS, X-Frame-Options, XSS-Protection
- **🔑 Authentication**: API key and session management with SHA256 hashing
- **📊 Audit Logging**: Complete request/response logging with threat detection

## Documentation

**Comprehensive guides available:**
- `DEPLOYMENT.md` 