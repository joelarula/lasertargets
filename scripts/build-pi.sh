#!/usr/bin/env bash
# Build the LaserTargets server for Raspberry Pi 4 (aarch64)
#
# Prerequisites:
#   - Docker installed and running
#
# Usage:
#   ./scripts/build-pi.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET="aarch64-unknown-linux-gnu"
IMAGE_NAME="lasertargets-cross-aarch64"
ARTIFACT_IMAGE_NAME="lasertargets-server-rpi4-artifact"
DIST_DIR="$PROJECT_ROOT/dist/pi"

echo "=== LaserTargets Raspberry Pi Build ==="

# Step 0: Free up Docker disk space
echo ""
echo "--- Cleaning up Docker (dangling images, stopped containers, build cache) ---"
docker system prune -f
echo ""
docker system df

# Step 1: Build the custom cross-compilation Docker image
echo ""
echo "--- Building cross Docker image: $IMAGE_NAME ---"
docker build \
    -f "$PROJECT_ROOT/docker/Dockerfile.aarch64" \
    -t "$IMAGE_NAME" \
    "$PROJECT_ROOT"

# Step 2: Build artifact image for Raspberry Pi 4
echo ""
echo "--- Building server artifact image: $ARTIFACT_IMAGE_NAME ---"
docker build \
    --build-arg BASE_IMAGE="$IMAGE_NAME" \
    --build-arg TARGET_TRIPLE="$TARGET" \
    -f "$PROJECT_ROOT/docker/Dockerfile.rpi4" \
    -t "$ARTIFACT_IMAGE_NAME" \
    "$PROJECT_ROOT"

# Step 3: Stage output for deployment
echo ""
echo "--- Staging build artifacts ---"
mkdir -p "$DIST_DIR"

CID="$(docker create "$ARTIFACT_IMAGE_NAME")"
cleanup() {
    docker rm -f "$CID" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker cp "$CID:/dist/." "$DIST_DIR/"

if [ -f "$DIST_DIR/server" ]; then
    echo "  Binary: $DIST_DIR/server"
else
    echo "  ERROR: Binary not found in artifact image at /dist/server"
    exit 1
fi

if [ -f "$DIST_DIR/libHeliosLaserDAC.so" ]; then
    echo "  Library: $DIST_DIR/libHeliosLaserDAC.so"
else
    echo "  WARNING: libHeliosLaserDAC.so not found in artifact image — DAC will be unavailable on Pi"
fi

echo ""
echo "=== Build complete ==="
echo "Artifacts staged in: $DIST_DIR"
echo ""
echo "To deploy, run: ./scripts/deploy-pi.sh <pi-hostname>"
