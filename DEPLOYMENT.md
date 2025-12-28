# OCM Implementation - Production Deployment Guide

This guide covers deploying OCM (Our Collective Memory) to production environments.

## Quick Start

### Development Environment
```bash
# Install dependencies
npm install

# Run database migrations
npm run migrate

# Start development server
npm start
```

### Production Deployment

#### Option 1: Docker (Recommended)
```bash
# Build and run with Docker Compose
docker-compose up -d

# Check logs
docker-compose logs -f

# Stop services
docker-compose down
```

#### Option 2: Manual Deployment
```bash
# Build for production
npm run deploy:build

# Run migrations
./target/release/migrate

# Start web server
./target/release/web-server
```

## Architecture Overview

### Services
- **Web Server (Port 8000)**: WASM interface and HTTP API
- **Relay Server (Port 8082)**: WebSocket relay for P2P communication
- **Database**: SQLite with WAL mode for persistence

### Security Features
- ✅ WebCrypto API integration for secure key management
- ✅ HTTPS enforcement in production
- ✅ OPFS (Origin Private File System) for browser persistence
- ✅ Cryptographic signatures for all memories
- ✅ Secure random number generation

## Environment Configuration

### Required Environment Variables
```bash
# Logging level
RUST_LOG=info

# Data directory (optional, defaults to ./data)
OCM_DATA_DIR=/app/data

# Network configuration (optional)
OCM_BIND_ADDRESS=127.0.0.1:8000
OCM_RELAY_ADDRESS=127.0.0.1:8082
```

### Security Configuration

#### HTTPS Setup (Production)
```bash
# Generate TLS certificates (Let's Encrypt recommended)
certbot certonly --standalone -d your-domain.com

# Configure reverse proxy (nginx/caddy)
# See examples in ./docs/nginx.conf.example
```

#### Content Security Policy
```html
<meta http-equiv="Content-Security-Policy" content="
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  connect-src 'self' wss: https:;
  img-src 'self' data:;
  style-src 'self' 'unsafe-inline';
">
```

## Database Management

### Migrations
```bash
# Run all pending migrations
cargo run --bin migrate

# Check migration status
sqlite3 data/ocm-impl.db ".tables"
```

### Backup
```bash
# Create database backup
cp data/ocm-impl.db backups/ocm-$(date +%Y%m%d).db

# Or use SQLite backup command
sqlite3 data/ocm-impl.db ".backup backups/ocm-$(date +%Y%m%d).db"
```

### Recovery
```bash
# Restore from backup
cp backups/ocm-20231228.db data/ocm-impl.db

# Verify integrity
sqlite3 data/ocm-impl.db "PRAGMA integrity_check;"
```

## Monitoring & Operations

### Health Checks
```bash
# Web server health
curl http://localhost:8000/

# Relay server health  
curl http://localhost:8082/health
```

### Logging
```bash
# View application logs
tail -f logs/ocm.log

# Docker logs
docker-compose logs -f ocm-web
docker-compose logs -f ocm-relay
```

### Metrics (Optional)
```bash
# Start monitoring stack
docker-compose --profile monitoring up -d

# Access Grafana dashboard
open http://localhost:3000
# Username: admin, Password: admin
```

## Scaling Considerations

### Horizontal Scaling
- Multiple relay servers can run behind a load balancer
- Web servers are stateless and can be scaled horizontally
- Database remains local to each user (local-first architecture)

### Performance Tuning
```bash
# SQLite optimization
echo "PRAGMA journal_mode=WAL;" | sqlite3 data/ocm-impl.db
echo "PRAGMA synchronous=NORMAL;" | sqlite3 data/ocm-impl.db
echo "PRAGMA cache_size=10000;" | sqlite3 data/ocm-impl.db
```

## Security Hardening

### Production Checklist
- [ ] HTTPS enabled with valid certificates
- [ ] Content Security Policy implemented
- [ ] Security headers configured (HSTS, X-Frame-Options, etc.)
- [ ] Regular security updates scheduled
- [ ] Database encryption at rest enabled
- [ ] Access logs configured
- [ ] Rate limiting implemented
- [ ] Firewall rules configured

### Key Management
- Private keys stored in browser's secure storage (IndexedDB)
- No private keys on server
- Regular key rotation recommended
- Backup procedures for key recovery

## Troubleshooting

### Common Issues

#### WASM Module Loading Fails
```javascript
// Check browser console for errors
// Ensure Content-Type: application/wasm is served correctly
```

#### Database Lock Errors
```bash
# Check WAL mode is enabled
sqlite3 data/ocm-impl.db "PRAGMA journal_mode;"

# Should return "wal"
```

#### OPFS Not Supported
```javascript
// Fallback to in-memory storage
// Check if running in secure context (HTTPS)
```

### Performance Issues
```bash
# Check system resources
top -p $(pgrep web-server)

# Check database performance
sqlite3 data/ocm-impl.db ".timer on" ".tables"
```

## Backup Strategy

### Automated Backups
```bash
#!/bin/bash
# backup.sh
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="/backups"

# Create backup
sqlite3 data/ocm-impl.db ".backup ${BACKUP_DIR}/ocm_${DATE}.db"

# Compress backup
gzip "${BACKUP_DIR}/ocm_${DATE}.db"

# Remove old backups (keep 30 days)
find ${BACKUP_DIR} -name "ocm_*.db.gz" -mtime +30 -delete
```

### Disaster Recovery
1. Stop OCM services
2. Restore database from latest backup
3. Verify database integrity
4. Restart services
5. Validate functionality

## Support & Maintenance

### Regular Maintenance
- Weekly: Check logs for errors
- Monthly: Update dependencies
- Quarterly: Security audit
- Annually: Disaster recovery test

### Updates
```bash
# Update dependencies
cargo update

# Rebuild WASM
npm run build:wasm

# Test in staging environment
npm test

# Deploy to production
npm run deploy:build
```

For additional support, see:
- [API Documentation](./docs/API.md)
- [Security Guide](./docs/SECURITY.md)
- [Contributing Guidelines](./CONTRIBUTING.md)