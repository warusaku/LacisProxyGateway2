#!/bin/bash
# LacisProxyGateway2 Deploy Script
# Usage: ./scripts/deploy.sh [all|frontend|backend]

set -e

# Configuration
SERVER_USER="akihabara_admin"
SERVER_HOST="192.168.3.242"
SERVER_DIR="/home/akihabara_admin/lacis-proxy-gateway"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for uncommitted changes
check_git_status() {
    log_info "Checking git status..."
    
    if [[ -n $(git status --porcelain) ]]; then
        log_error "There are uncommitted changes. Please commit before deploying."
        git status --short
        exit 1
    fi
    
    # Check if local is ahead of remote
    git fetch origin main 2>/dev/null || true
    LOCAL=$(git rev-parse HEAD)
    REMOTE=$(git rev-parse origin/main 2>/dev/null || echo "")
    
    if [[ -n "$REMOTE" && "$LOCAL" != "$REMOTE" ]]; then
        log_warn "Local branch differs from remote. Make sure to push your changes."
    fi
    
    log_info "Git status OK"
}

# Deploy backend
deploy_backend() {
    log_info "Deploying backend..."
    
    # Sync source
    rsync -avz --delete \
        --exclude 'target' \
        --exclude '.git' \
        backend/ ${SERVER_USER}@${SERVER_HOST}:${SERVER_DIR}/backend/
    
    # Build on server
    log_info "Building backend on server..."
    ssh ${SERVER_USER}@${SERVER_HOST} "cd ${SERVER_DIR}/backend && ~/.cargo/bin/cargo build --release"
    
    # Restart service
    log_info "Restarting backend service..."
    ssh ${SERVER_USER}@${SERVER_HOST} "sudo systemctl restart lacis-proxy-gateway || echo 'Service not configured yet'"
    
    log_info "Backend deployed successfully!"
}

# Deploy frontend
deploy_frontend() {
    log_info "Deploying frontend..."
    
    # Sync source
    rsync -avz --delete \
        --exclude 'node_modules' \
        --exclude '.next' \
        --exclude '.git' \
        frontend/ ${SERVER_USER}@${SERVER_HOST}:${SERVER_DIR}/frontend/
    
    # Install dependencies and build on server
    log_info "Building frontend on server..."
    ssh ${SERVER_USER}@${SERVER_HOST} "cd ${SERVER_DIR}/frontend && npm install && npm run build"
    
    # Restart frontend
    log_info "Restarting frontend..."
    ssh ${SERVER_USER}@${SERVER_HOST} "pkill -f 'next-server.*lacis' || true; cd ${SERVER_DIR}/frontend && nohup npm start > /tmp/lacis-frontend.log 2>&1 &"
    
    log_info "Frontend deployed successfully!"
}

# Show status
show_status() {
    log_info "Checking service status..."
    ssh ${SERVER_USER}@${SERVER_HOST} "systemctl status lacis-proxy-gateway --no-pager 2>/dev/null || echo 'Backend service not configured'"
    echo ""
    ssh ${SERVER_USER}@${SERVER_HOST} "pgrep -f 'next-server.*lacis' > /dev/null && echo 'Frontend: Running' || echo 'Frontend: Not running'"
}

# Initialize database
init_database() {
    log_info "Initializing database..."

    # Create remote scripts directory
    ssh ${SERVER_USER}@${SERVER_HOST} "mkdir -p ${SERVER_DIR}/scripts"

    # Copy SQL script
    scp scripts/init_mysql.sql ${SERVER_USER}@${SERVER_HOST}:${SERVER_DIR}/scripts/

    # Run MySQL init
    log_info "Running MySQL initialization..."
    ssh ${SERVER_USER}@${SERVER_HOST} "mariadb -u akihabara_admin -p'akihabara12345@' < ${SERVER_DIR}/scripts/init_mysql.sql"

    log_info "Database initialized successfully"
}

# Setup systemd service
setup_systemd() {
    log_info "Setting up systemd service..."

    # Create systemd service file
    ssh ${SERVER_USER}@${SERVER_HOST} "sudo tee /etc/systemd/system/lacis-proxy-gateway.service > /dev/null << 'EOFSERVICE'
[Unit]
Description=LacisProxyGateway2 Reverse Proxy
After=network.target mariadb.service mongod.service

[Service]
Type=simple
User=akihabara_admin
WorkingDirectory=/home/akihabara_admin/lacis-proxy-gateway/backend
ExecStart=/home/akihabara_admin/lacis-proxy-gateway/backend/target/release/lacis-proxy-gateway
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOFSERVICE"

    ssh ${SERVER_USER}@${SERVER_HOST} "sudo systemctl daemon-reload"
    ssh ${SERVER_USER}@${SERVER_HOST} "sudo systemctl enable lacis-proxy-gateway"

    log_info "Systemd service configured"
}

# Main
main() {
    local target="${1:-all}"
    
    log_info "LacisProxyGateway2 Deploy Script"
    log_info "Target: ${target}"
    echo ""
    
    check_git_status
    
    case "$target" in
        all)
            deploy_backend
            deploy_frontend
            ;;
        backend)
            deploy_backend
            ;;
        frontend)
            deploy_frontend
            ;;
        init-db)
            init_database
            exit 0
            ;;
        setup-systemd)
            setup_systemd
            exit 0
            ;;
        status)
            show_status
            exit 0
            ;;
        *)
            log_error "Unknown target: $target"
            echo "Usage: $0 [all|frontend|backend|init-db|setup-systemd|status]"
            exit 1
            ;;
    esac
    
    echo ""
    show_status
    
    echo ""
    log_info "Deployment complete!"
    log_info "Access: http://${SERVER_HOST}/LacisProxyGateway2"
}

# Run
main "$@"
