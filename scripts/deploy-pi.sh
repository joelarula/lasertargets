#!/usr/bin/env bash
# Deploy the LaserTargets server to a Raspberry Pi
#
# Usage:
#   ./scripts/deploy-pi.sh <hostname>          # e.g., raspberrypi.local
#   ./scripts/deploy-pi.sh <user@hostname>     # e.g., pi@192.168.1.50
#
# Prerequisites:
#   - Run ./scripts/build-pi.sh first
#   - SSH key configured for the Pi
#   - Run deploy/install-pi-deps.sh on the Pi once (initial setup)

set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <hostname|user@hostname>"
    echo "  Example: $0 raspberrypi.local"
    echo "  Example: $0 pi@192.168.1.50"
    exit 1
fi

TARGET_HOST="$1"
# Default to pi@ if no user specified
if [[ "$TARGET_HOST" != *@* ]]; then
    TARGET_HOST="pi@$TARGET_HOST"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist/pi"

echo "=== Deploying LaserTargets to $TARGET_HOST ==="

# Verify build artifacts exist
if [ ! -f "$DIST_DIR/server" ]; then
    echo "ERROR: Build artifacts not found in $DIST_DIR"
    echo "Run ./scripts/build-pi.sh first"
    exit 1
fi

# Stop the service if it's running
echo "--- Stopping existing service (if running) ---"
ssh "$TARGET_HOST" "sudo systemctl stop lasertargets-server 2>/dev/null || true"

# Create directories on the Pi
echo "--- Ensuring directory structure ---"
ssh "$TARGET_HOST" "sudo mkdir -p /opt/lasertargets/lib && sudo chown -R pi:pi /opt/lasertargets"

# Deploy binary
echo "--- Deploying server binary ---"
scp "$DIST_DIR/server" "$TARGET_HOST:/opt/lasertargets/server"
ssh "$TARGET_HOST" "chmod +x /opt/lasertargets/server"

# Deploy shared library (if available)
if [ -f "$DIST_DIR/libHeliosLaserDAC.so" ]; then
    echo "--- Deploying Helios DAC library ---"
    scp "$DIST_DIR/libHeliosLaserDAC.so" "$TARGET_HOST:/opt/lasertargets/lib/"
else
    echo "--- Skipping Helios DAC library (not built) ---"
fi

# Deploy systemd service file
echo "--- Installing systemd service ---"
scp "$PROJECT_ROOT/deploy/lasertargets-server.service" "$TARGET_HOST:/tmp/lasertargets-server.service"
ssh "$TARGET_HOST" "sudo mv /tmp/lasertargets-server.service /etc/systemd/system/ && sudo systemctl daemon-reload"

# Enable and start the service
echo "--- Starting service ---"
ssh "$TARGET_HOST" "sudo systemctl enable lasertargets-server && sudo systemctl start lasertargets-server"

# Check status
echo ""
echo "--- Service status ---"
ssh "$TARGET_HOST" "sudo systemctl status lasertargets-server --no-pager" || true

echo ""
echo "=== Deployment complete ==="
echo ""
echo "Useful commands:"
echo "  ssh $TARGET_HOST 'sudo systemctl status lasertargets-server'"
echo "  ssh $TARGET_HOST 'sudo journalctl -u lasertargets-server -f'"
echo "  ssh $TARGET_HOST 'sudo systemctl restart lasertargets-server'"
