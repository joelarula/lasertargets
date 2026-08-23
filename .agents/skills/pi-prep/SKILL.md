---
name: pi-prep
description: Prepare and configure fresh Raspberry Pi 4 OS environment, udev rules, directories, systemd services, and performance tweaks for real-time laser server execution. Use when setting up or provisioning a new Raspberry Pi.
---

# Raspberry Pi 4 System Preparation & Provisioning Skill

This skill provides step-by-step instructions for provisioning a fresh Raspberry Pi 4 OS environment (`lasertargets@lasertargets.local`) to host the LaserTargets server and drive the physical USB Helios Laser DAC.

---

## 1. Operating System & Host Setup

1. **OS Image**: Install Raspberry Pi OS Lite (64-bit ARM64, Debian Bookworm).
2. **Network & User Configuration**:
   - Hostname: `lasertargets.local` (or static IP).
   - Primary User: `lasertargets`.
   - Enable SSH service with public key authentication.
3. **Dependencies**:
   ```bash
   sudo apt-get update
   sudo apt-get install -y libusb-1.0-0 libusb-1.0-0-dev ca-certificates libssl-dev powershell
   ```

---

## 2. Directory Creation & USB Hardware Permissions

1. **Create Target Directory**:
   ```bash
   sudo mkdir -p /opt/lasertargets/assets
   sudo chown -R lasertargets:lasertargets /opt/lasertargets
   ```
2. **Configure USB Helios DAC Udev Rules (`/etc/udev/rules.d/99-helios.rules`)**:
   - Grants unprivileged `lasertargets` user direct access to the Helios USB DAC hardware vendor endpoint (`1209:e500`):
   ```bash
   sudo bash -c 'cat << "EOF" > /etc/udev/rules.d/99-helios.rules
   SUBSYSTEM=="usb", ATTR{idVendor}=="1209", ATTR{idProduct}=="e500", MODE="0666", GROUP="plugdev"
   EOF'
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

---

## 3. System Performance & Latency Optimization

Run the automated OS optimization script to lock CPU frequencies and disable USB power management:

```powershell
.\scripts\optimize-pi-system.ps1 -TargetHost lasertargets@lasertargets.local
```

### Optimizations Applied:
- **CPU Scaling Governor**: Locked to `performance` across all 4 ARM cores via `cpu-performance.service`.
- **USB Autosuspend**: Disabled via `usbcore.autosuspend=-1` in `/boot/firmware/cmdline.txt`.
- **Real-Time Priority Capability**:
  ```bash
  sudo setcap cap_sys_nice+ep /opt/lasertargets/server
  ```

---

## 4. Systemd Auto-Start Service Setup

1. **Deploy Systemd Unit File** (`deploy/lasertargets-server.service`):
   ```bash
   sudo cp /opt/lasertargets/lasertargets-server.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable lasertargets-server.service
   ```
2. **Service Management Commands**:
   - Start: `sudo systemctl start lasertargets-server`
   - Stop: `sudo systemctl stop lasertargets-server`
   - Status: `sudo systemctl status lasertargets-server`
   - Logs: `journalctl -u lasertargets-server -f`

---

## 5. Pre-Flight Verification & Health Checks

Run hardware diagnostics scripts to verify setup before deploying production server binaries:

- **Verify Network Stability**:
  ```powershell
  .\scripts\check-pi-network.ps1
  ```
- **Verify SD Card Read/Write Speed**:
  ```powershell
  .\scripts\check-sd-card.ps1
  ```
- **Test USB Helios DAC Output**:
  ```powershell
  .\scripts\run-dac-test-pi.ps1
  ```
