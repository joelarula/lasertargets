#!/usr/bin/env bash
# One-time setup script for the Raspberry Pi
#
# Run on the Pi itself (or via SSH):
#   sudo bash install-pi-deps.sh
#
# What it does:
#   1. Installs libusb runtime (required by Helios DAC)
#   2. Creates the /opt/lasertargets directory structure
#   3. Installs udev rules for non-root Helios DAC USB access

set -euo pipefail

echo "=== LaserTargets Pi Setup ==="

# Install runtime dependencies
echo "--- Installing runtime dependencies ---"
apt-get update
apt-get install -y libusb-1.0-0

# Create directory structure
echo "--- Creating /opt/lasertargets ---"
mkdir -p /opt/lasertargets/lib
chown -R pi:pi /opt/lasertargets

# Install udev rules for Helios DAC USB device
# Helios DAC uses a custom USB device — this rule grants access without root
echo "--- Installing Helios DAC udev rules ---"
cat > /etc/udev/rules.d/99-helios-dac.rules << 'EOF'
# Helios Laser DAC — allow non-root access
# Vendor ID 0x1209, Product ID 0xe500 (Helios DAC)
SUBSYSTEM=="usb", ATTR{idVendor}=="1209", ATTR{idProduct}=="e500", MODE="0666", GROUP="plugdev"
EOF

udevadm control --reload-rules
udevadm trigger

echo ""
echo "=== Setup complete ==="
echo "Make sure user 'pi' is in the 'plugdev' group:"
echo "  sudo usermod -aG plugdev pi"
