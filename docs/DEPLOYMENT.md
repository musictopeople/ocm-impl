# OCM Deployment Guide

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        OCM Node Architecture                    │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │
│  │   Web UI        │    │   REST API      │    │   CLI Tools     │ │
│  │   (Future)      │    │   (Future)      │    │   (Current)     │ │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                        Application Layer                        │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │
│  │   Identity      │◄──►│   Networking    │◄──►│   Sync/CRDT     │ │
│  │   Management    │    │   P2P Protocol  │    │   Conflict      │ │
│  │   (PLC/DID)     │    │   (TCP/UDP)     │    │   Resolution    │ │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘ │
│           ▲                        ▲                        ▲     │
│           │                        │                        │     │
│           ▼                        ▼                        ▼     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐ │
│  │   Claims        │    │   Persistence   │    │   Configuration │ │
│  │   System        │    │   SQLite DB     │    │   Management    │ │
│  │   (Proxy)       │    │   (WAL Mode)    │    │   (Env/File)    │ │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Network Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    OCM Federation Network                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐      UDP:8081      ┌─────────────┐             │
│  │   OCM Node  │◄────Discovery─────►│   OCM Node  │             │
│  │   Alice     │                    │   Bob       │             │
│  │   :8080     │      TCP:8080      │   :8080     │             │
│  └─────────────┘◄──Mem Federation──►└─────────────┘             │
│         │                                   │                   │
│         ▼                                   ▼                   │
│  ┌─────────────┐                    ┌─────────────┐             │
│  │ SQLite DB   │                    │ SQLite DB   │             │
│  │ (Personal)  │                    │ (Personal)  │             │
│  └─────────────┘                    └─────────────┘             │
│                                                                 │
│                       ┌─────────────┐                          │
│                       │ PLC Network │                          │
│                       │ (Bluesky)   │                          │
│                       │ Identity    │                          │
│                       │ Directory   │                          │
│                       └─────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         OCM Data Flow                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. CAPTURE                                                     │
│  ┌─────────────┐    JSON Data    ┌─────────────┐               │
│  │ Individual  │────────────────►│ SignedMemory│               │
│  │ Experience  │    (Serialize)   │ (SHA256)    │               │
│  │ Location    │                 │             │               │
│  └─────────────┘                 └─────────────┘               │
│                                         │                      │
│                                         ▼                      │
│  2. ATTESTATION                                                 │
│  ┌─────────────┐    Private Key   ┌─────────────┐              │
│  │ PLC Identity│────────────────►│ ED25519     │              │
│  │ (DID)       │    (Sign)        │ Signature   │              │
│  └─────────────┘                 └─────────────┘              │
│                                         │                      │
│                                         ▼                      │
│  3. FEDERATION                                                  │
│  ┌─────────────┐    P2P Network   ┌─────────────┐              │
│  │ Local DB    │◄────────────────►│ Remote Peers│              │
│  │ (SQLite)    │    (TCP/UDP)     │ (Verify)    │              │
│  └─────────────┘                 └─────────────┘              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## System Requirements

### Hardware Requirements

**Minimum:**
- CPU: 1 core, 1GHz
- RAM: 256MB
- Storage: 100MB
- Network: 1Mbps

**Recommended:**
- CPU: 2+ cores, 2GHz+
- RAM: 1GB+
- Storage: 1GB+ (for larger memory databases)
- Network: 10Mbps+ (for federation)

### Software Requirements

**Operating System:**
- Linux (Ubuntu 20.04+, CentOS 8+, Debian 11+)
- macOS (10.15+)
- Windows 10+ (with WSL2 recommended)

**Dependencies:**
- Rust 1.75+ (with Cargo)
- SQLite 3.35+
- OpenSSL 1.1+

## Installation

### 1. Install Rust

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Clone and Build

```bash
# Clone the repository
git clone <repository-url>
cd ocm-impl

# Build the application
cargo build --release

# Initialize database
cargo run --release --bin migrate
```

### 3. Configuration

#### Environment Variables

Create a `.env` file or set environment variables:

```bash
# Database configuration
export OCM_DATABASE_PATH="./data/ocm-impl.db"

# Network configuration
export OCM_NETWORK_PORT="8080"
export OCM_DISCOVERY_PORT="8081"
export OCM_BIND_ADDRESS="127.0.0.1"

# Identity configuration
export OCM_IDENTITY_HANDLE="your-handle"
export OCM_PLC_DIRECTORY_URL="https://plc.directory"

# Logging configuration
export OCM_LOG_LEVEL="info"
export OCM_LOG_FORMAT="json"
```

#### Configuration File (Alternative)

Create `config/ocm.toml`:

```toml
[database]
path = "./data/ocm-impl.db"
wal_mode = true
journal_mode = "WAL"

[network]
port = 8080
discovery_port = 8081
bind_address = "127.0.0.1"
enable_network_calls = true
seed_peers = ["127.0.0.1:8080"]

[identity]
handle = "your-handle"
plc_directory_url = "https://plc.directory"

[logging]
level = "info"
format = "json"
```

## Deployment Options

### 1. Standalone Node

**Basic deployment for development or single-user:**

```bash
# Start the node
cargo run --release --bin ocm-impl

# Or using the built binary
./target/release/ocm-impl
```

**Service configuration (systemd):**

Create `/etc/systemd/system/ocm-node.service`:

```ini
[Unit]
Description=OCM Node
After=network.target

[Service]
Type=simple
User=ocm
WorkingDirectory=/opt/ocm-impl
Environment=OCM_DATABASE_PATH=/opt/ocm-impl/data/ocm-impl.db
Environment=OCM_LOG_LEVEL=info
ExecStart=/opt/ocm-impl/target/release/ocm-impl
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### 2. Docker Deployment

**Dockerfile:**

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    sqlite3 \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/ocm-impl /app/
COPY --from=builder /app/target/release/migrate /app/
COPY migrations/ /app/migrations/

RUN mkdir -p /app/data
RUN /app/migrate

EXPOSE 8080 8081/udp

CMD ["./ocm-impl"]
```

**Docker Compose:**

```yaml
version: '3.8'

services:
  ocm-node:
    build: .
    ports:
      - "8080:8080"
      - "8081:8081/udp"
    volumes:
      - ocm-data:/app/data
    environment:
      - OCM_DATABASE_PATH=/app/data/ocm-impl.db
      - OCM_NETWORK_PORT=8080
      - OCM_DISCOVERY_PORT=8081
      - OCM_LOG_LEVEL=info
    restart: unless-stopped

volumes:
  ocm-data:
```

### 3. Cloud Deployment

#### AWS EC2

```bash
# Launch EC2 instance (t3.micro or larger)
aws ec2 run-instances \
    --image-id ami-0c7217cdde317cfec \
    --instance-type t3.micro \
    --key-name your-key \
    --security-group-ids sg-your-sg \
    --user-data file://cloud-init.sh

# cloud-init.sh
#!/bin/bash
yum update -y
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
git clone <repository-url>
cd ocm-impl
cargo build --release
cargo run --release --bin migrate
nohup cargo run --release --bin ocm-impl &
```

#### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ocm-node
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ocm-node
  template:
    metadata:
      labels:
        app: ocm-node
    spec:
      containers:
      - name: ocm-node
        image: ocm-impl:latest
        ports:
        - containerPort: 8080
        - containerPort: 8081
          protocol: UDP
        env:
        - name: OCM_DATABASE_PATH
          value: "/data/ocm-impl.db"
        - name: OCM_BIND_ADDRESS
          value: "0.0.0.0"
        volumeMounts:
        - name: data
          mountPath: /data
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: ocm-data

---
apiVersion: v1
kind: Service
metadata:
  name: ocm-service
spec:
  selector:
    app: ocm-node
  ports:
  - name: tcp
    port: 8080
    targetPort: 8080
  - name: udp
    port: 8081
    targetPort: 8081
    protocol: UDP
  type: LoadBalancer
```

## Network Configuration

### Firewall Rules

**For standalone deployment:**

```bash
# Allow OCM ports
sudo ufw allow 8080/tcp comment "OCM Federation"
sudo ufw allow 8081/udp comment "OCM Discovery"

# For production, restrict to known peers
sudo ufw allow from 10.0.0.0/8 to any port 8080
```

### Port Mapping

| Port | Protocol | Purpose | Access |
|------|----------|---------|---------|
| 8080 | TCP | P2P Federation | Peers only |
| 8081 | UDP | Peer Discovery | Broadcast |

### NAT Traversal

For nodes behind NAT/firewalls:

```bash
# Port forwarding (router configuration)
# Forward external:8080 -> internal:8080 (TCP)
# Forward external:8081 -> internal:8081 (UDP)

# UPnP (automatic, if enabled)
export OCM_ENABLE_UPNP=true
```

## Monitoring and Logging

### Structured Logging

```bash
# JSON logs for production
export OCM_LOG_FORMAT=json

# Example log output
{"timestamp":"2024-01-15T10:30:00Z","level":"INFO","message":"OCM node started","peer_id":"alice-123","port":8080}
```

### Metrics Collection

**Prometheus metrics (future enhancement):**

```
# OCM Node Metrics
ocm_memories_total{node="alice"} 150
ocm_peers_connected{node="alice"} 3
ocm_sync_operations_total{node="alice"} 1200
ocm_conflicts_resolved_total{node="alice"} 5
```

### Health Checks

```bash
# Check node status
curl http://localhost:8080/health  # (Future API endpoint)

# Check database
sqlite3 data/ocm-impl.db "SELECT COUNT(*) FROM signed_memory;"

# Check connectivity
nc -zv localhost 8080
nc -zuv localhost 8081
```

## Security Considerations

### Network Security

```bash
# TLS termination (use nginx/traefik for production)
server {
    listen 443 ssl;
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    location / {
        proxy_pass http://localhost:8080;
    }
}
```

### Key Management

⚠️ **Critical Security Notice:**

1. **Private Keys**: Currently stored in database - implement hardware security modules (HSM) for production
2. **Backup Strategy**: Encrypt database backups with separate keys
3. **Access Control**: Restrict database file permissions (600)
4. **Network Access**: Use VPN or private networks for federation

```bash
# Secure file permissions
chmod 600 data/ocm-impl.db
chown ocm:ocm data/ocm-impl.db

# Backup with encryption
sqlite3 data/ocm-impl.db ".backup backup.db"
gpg --symmetric --cipher-algo AES256 backup.db
```

## Troubleshooting

### Common Issues

**Database corruption:**
```bash
# Check database integrity
sqlite3 data/ocm-impl.db "PRAGMA integrity_check;"

# Repair if needed
sqlite3 data/ocm-impl.db ".recover" | sqlite3 recovered.db
```

**Network connectivity:**
```bash
# Test peer discovery
nc -zuv 127.0.0.1 8081

# Test federation port
nc -zv peer-ip 8080

# Check firewall
sudo ufw status verbose
```

**Memory issues:**
```bash
# Check memory usage
ps aux | grep ocm-impl

# Database size
du -sh data/ocm-impl.db

# Vacuum database
sqlite3 data/ocm-impl.db "VACUUM;"
```

### Debug Mode

```bash
# Enable debug logging
export OCM_LOG_LEVEL=debug

# Trace network operations
export RUST_LOG=ocm_impl::networking=trace
```

## Performance Tuning

### Database Optimization

```sql
-- Optimize SQLite for OCM workload
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;  -- 64MB cache
PRAGMA temp_store = MEMORY;
```

### Network Optimization

```bash
# Increase network buffers (Linux)
echo 'net.core.rmem_max = 16777216' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 16777216' >> /etc/sysctl.conf
sysctl -p
```

## Backup and Recovery

### Database Backup

```bash
#!/bin/bash
# backup-ocm.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backup/ocm"
DB_PATH="data/ocm-impl.db"

# Create backup
sqlite3 $DB_PATH ".backup $BACKUP_DIR/ocm_$DATE.db"

# Encrypt backup
gpg --symmetric --cipher-algo AES256 "$BACKUP_DIR/ocm_$DATE.db"
rm "$BACKUP_DIR/ocm_$DATE.db"

# Retention (keep 30 days)
find $BACKUP_DIR -name "*.gpg" -mtime +30 -delete
```

### Recovery

```bash
# Decrypt and restore
gpg --decrypt ocm_20240115.db.gpg > ocm_restored.db

# Verify integrity
sqlite3 ocm_restored.db "PRAGMA integrity_check;"

# Replace current database
cp data/ocm-impl.db data/ocm-impl.db.backup
mv ocm_restored.db data/ocm-impl.db
```

## Scaling Considerations

### Horizontal Scaling

- Each node operates independently
- Federation allows natural horizontal scaling
- Consider relay nodes for large networks

### Vertical Scaling

- Database size grows with memories
- Consider partitioning by time/type
- Monitor SQLite performance limits (TB scale)

### Future Enhancements

- WebAssembly for browser nodes
- Mobile applications
- Relay infrastructure for NAT traversal
- Web UI for administration
- Enhanced monitoring and metrics