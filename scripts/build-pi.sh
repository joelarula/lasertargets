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

# Step 2: Cross-compile the server inside the Docker container
echo ""
echo "--- Cross-compiling server for $TARGET ---"
docker run --rm \
    -v "$PROJECT_ROOT:/project" \
    -w /project \
    "$IMAGE_NAME" \
    cargo build -p server --target "$TARGET" --release

# Step 3: Stage output for deployment
echo ""
echo "--- Staging build artifacts ---"
mkdir -p "$DIST_DIR"

BINARY="$PROJECT_ROOT/target/$TARGET/release/server"
SO_FILE="$PROJECT_ROOT/target/$TARGET/release/libHeliosLaserDAC.so"

if [ -f "$BINARY" ]; then
    cp "$BINARY" "$DIST_DIR/"
    echo "  Binary: $DIST_DIR/server"
else
    echo "  ERROR: Binary not found at $BINARY"
    exit 1
fi

if [ -f "$SO_FILE" ]; then
    cp "$SO_FILE" "$DIST_DIR/"
    echo "  Library: $DIST_DIR/libHeliosLaserDAC.so"
else
    echo "  WARNING: libHeliosLaserDAC.so not found — DAC will be unavailable on Pi"
fi

echo ""
echo "=== Build complete ==="
echo "Artifacts staged in: $DIST_DIR"
echo ""
echo "To deploy, run: ./scripts/deploy-pi.sh <pi-hostname>"
