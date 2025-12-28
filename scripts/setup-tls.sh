#!/bin/bash

# OCM TLS Setup Script
# Sets up TLS certificates for production deployment

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERTS_DIR="${PROJECT_ROOT}/certs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔒 OCM TLS Certificate Setup${NC}"
echo "=================================="

# Create certs directory
mkdir -p "${CERTS_DIR}"
cd "${CERTS_DIR}"

# Function to generate development certificates
generate_dev_certs() {
    echo -e "${YELLOW}📋 Generating development certificates...${NC}"
    
    # Generate private key
    openssl genrsa -out key.pem 2048 2>/dev/null
    
    # Generate certificate signing request
    cat > csr.conf <<EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = Development
L = Local
O = OCM Development
CN = localhost

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = 127.0.0.1
IP.1 = 127.0.0.1
EOF

    # Generate self-signed certificate
    openssl req -new -x509 -key key.pem -out cert.pem -days 365 -config csr.conf -extensions v3_req 2>/dev/null
    
    # Clean up
    rm csr.conf
    
    echo -e "${GREEN}✅ Development certificates generated${NC}"
    echo -e "📁 Certificate: ${CERTS_DIR}/cert.pem"
    echo -e "📁 Private key: ${CERTS_DIR}/key.pem"
    echo -e "⏰ Valid for: 365 days"
}

# Function to setup Let's Encrypt certificates (production)
setup_letsencrypt() {
    local domain="$1"
    
    echo -e "${YELLOW}🌐 Setting up Let's Encrypt for domain: ${domain}${NC}"
    
    # Check if certbot is installed
    if ! command -v certbot &> /dev/null; then
        echo -e "${RED}❌ certbot is not installed${NC}"
        echo "Please install certbot first:"
        echo "  Ubuntu/Debian: sudo apt install certbot"
        echo "  macOS: brew install certbot"
        echo "  Other: https://certbot.eff.org/"
        exit 1
    fi
    
    echo -e "${BLUE}🔄 Running certbot...${NC}"
    echo "This will:"
    echo "1. Verify domain ownership"
    echo "2. Generate production TLS certificates"
    echo "3. Set up automatic renewal"
    echo ""
    
    # Run certbot
    sudo certbot certonly --standalone -d "${domain}" --agree-tos --no-eff-email
    
    # Copy certificates to our certs directory
    sudo cp "/etc/letsencrypt/live/${domain}/fullchain.pem" cert.pem
    sudo cp "/etc/letsencrypt/live/${domain}/privkey.pem" key.pem
    sudo chown $(whoami):$(whoami) cert.pem key.pem
    
    echo -e "${GREEN}✅ Let's Encrypt certificates installed${NC}"
    echo -e "📁 Certificate: ${CERTS_DIR}/cert.pem"
    echo -e "📁 Private key: ${CERTS_DIR}/key.pem"
    echo -e "🔄 Auto-renewal: Configured via cron"
}

# Function to verify certificates
verify_certs() {
    echo -e "${BLUE}🔍 Verifying certificates...${NC}"
    
    if [[ -f "cert.pem" && -f "key.pem" ]]; then
        # Check certificate validity
        cert_info=$(openssl x509 -in cert.pem -noout -text 2>/dev/null)
        subject=$(openssl x509 -in cert.pem -noout -subject 2>/dev/null | cut -d= -f2-)
        expiry=$(openssl x509 -in cert.pem -noout -enddate 2>/dev/null | cut -d= -f2)
        
        echo -e "${GREEN}✅ Certificates found and valid${NC}"
        echo -e "📋 Subject: ${subject}"
        echo -e "📅 Expires: ${expiry}"
        
        # Check if certificate and key match
        cert_md5=$(openssl x509 -noout -modulus -in cert.pem 2>/dev/null | openssl md5)
        key_md5=$(openssl rsa -noout -modulus -in key.pem 2>/dev/null | openssl md5)
        
        if [[ "${cert_md5}" == "${key_md5}" ]]; then
            echo -e "${GREEN}🔐 Certificate and private key match${NC}"
        else
            echo -e "${RED}❌ Certificate and private key do not match${NC}"
            exit 1
        fi
        
        # Check subject alternative names
        sans=$(openssl x509 -noout -text -in cert.pem 2>/dev/null | grep -A1 "Subject Alternative Name" | tail -1 || echo "None")
        echo -e "🌐 Subject Alt Names: ${sans}"
        
    else
        echo -e "${RED}❌ Certificates not found${NC}"
        exit 1
    fi
}

# Function to setup nginx configuration
setup_nginx() {
    local domain="${1:-localhost}"
    
    echo -e "${BLUE}🌐 Generating nginx configuration...${NC}"
    
    cat > nginx-ocm.conf <<EOF
# OCM Secure Web Server Configuration
# Production-ready nginx setup with TLS

server {
    listen 80;
    server_name ${domain};
    
    # Redirect HTTP to HTTPS
    return 301 https://\$server_name\$request_uri;
}

server {
    listen 443 ssl http2;
    server_name ${domain};
    
    # TLS Configuration
    ssl_certificate ${CERTS_DIR}/cert.pem;
    ssl_certificate_key ${CERTS_DIR}/key.pem;
    
    # Modern TLS configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' wss: https:; img-src 'self' data:; style-src 'self' 'unsafe-inline';" always;
    
    # OCM Application
    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        
        # WebSocket support (for future relay integration)
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";
    }
    
    # Health check endpoint
    location /health {
        proxy_pass http://127.0.0.1:8000/health;
        access_log off;
    }
    
    # API endpoints
    location /api/ {
        proxy_pass http://127.0.0.1:8000/api/;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }
}
EOF

    echo -e "${GREEN}✅ nginx configuration generated${NC}"
    echo -e "📁 Config file: ${CERTS_DIR}/nginx-ocm.conf"
    echo ""
    echo "To enable this configuration:"
    echo "1. Copy to nginx sites directory:"
    echo "   sudo cp nginx-ocm.conf /etc/nginx/sites-available/ocm"
    echo "2. Enable the site:"
    echo "   sudo ln -s /etc/nginx/sites-available/ocm /etc/nginx/sites-enabled/"
    echo "3. Test configuration:"
    echo "   sudo nginx -t"
    echo "4. Reload nginx:"
    echo "   sudo systemctl reload nginx"
}

# Main script logic
case "${1:-dev}" in
    "dev"|"development")
        echo -e "${YELLOW}🔧 Setting up development certificates${NC}"
        generate_dev_certs
        verify_certs
        setup_nginx "localhost"
        ;;
    "prod"|"production")
        if [[ -z "${2:-}" ]]; then
            echo -e "${RED}❌ Domain name required for production setup${NC}"
            echo "Usage: $0 production your-domain.com"
            exit 1
        fi
        echo -e "${YELLOW}🌐 Setting up production certificates for: $2${NC}"
        setup_letsencrypt "$2"
        verify_certs
        setup_nginx "$2"
        ;;
    "verify")
        verify_certs
        ;;
    *)
        echo "OCM TLS Certificate Setup"
        echo ""
        echo "Usage: $0 [command] [domain]"
        echo ""
        echo "Commands:"
        echo "  dev          Generate development certificates (default)"
        echo "  production   Setup Let's Encrypt certificates for production"
        echo "  verify       Verify existing certificates"
        echo ""
        echo "Examples:"
        echo "  $0 dev                           # Development setup"
        echo "  $0 production example.com        # Production setup"
        echo "  $0 verify                        # Verify certificates"
        ;;
esac

echo ""
echo -e "${GREEN}🎉 TLS setup complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Start OCM server: npm run dev:secure"
echo "2. Visit: https://127.0.0.1:8443 (or your domain)"
echo "3. Accept self-signed certificate (development only)"
echo "4. Deploy with reverse proxy for production"