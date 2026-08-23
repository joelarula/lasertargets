---
name: pi-deploy
description: Deploy, run, monitor, and optimize LaserTargets server on Raspberry Pi 4 hardware over SSH. Use when deploying binaries, starting server services, or tuning Pi OS performance.
---

# Raspberry Pi 4 Deployment & Management Skill

This skill documents the full lifecycle for deploying, launching, monitoring, and optimizing the `lasertargets` server on physical Raspberry Pi 4 hardware (`lasertargets@lasertargets.local`).

---

## 1. Target Hardware Specifications

- **Device**: Raspberry Pi 4 Model B (ARM64)
- **Hostname**: `lasertargets.local` (or IP e.g. `192.168.1.120`)
- **User**: `lasertargets`
- **Installation Directory**: `/opt/lasertargets/`
- **Systemd Service**: `lasertargets-server.service`

---

## 2. Deployment Workflow

Deploy compiled ARM64 binaries, dynamic libraries, shape templates, and assets to the Pi:

```powershell
.\scripts\deploy-pi.ps1 -TargetHost lasertargets@lasertargets.local
```

### Assets Deployed to `/opt/lasertargets/`:
- `server` (ARM64 ELF executable)
- `libHeliosLaserDAC.so` (Helios USB Laser DAC C shared library)
- `assets/` (shapes, templates, vector fonts, configurations)
- `lasertargets-server.service` (Systemd unit file)

---

## 3. Interactive Execution & Log Monitoring

To launch the host server interactively over SSH with live log streaming:

```powershell
.\scripts\run-server-pi.ps1 -TargetHost lasertargets@lasertargets.local
```

- Streams real-time `tracing`/`log` logs via `journalctl -u lasertargets-server -f`.
- Allows graceful shutdown via `Ctrl+C`.

---

## 4. Raspberry Pi OS Tuning & Hardware Optimization

To ensure smooth 60 FPS DAC streaming and low network latency, tune system parameters on the Pi:

```powershell
.\scripts\optimize-pi-system.ps1
```

### Optimization Actions Performed:
1. Sets CPU scaling governor to **`performance`** across all 4 ARM cores.
2. Increases USB buffer depth and udev permissions for `/dev/bus/usb/*`.
3. Grants `CAP_SYS_NICE` capabilities to `server` binary for real-time thread priority.

---

## 5. Hardware Diagnostics & Testing

- **Run Standalone DAC Hardware Test on Pi**:
  ```powershell
  .\scripts\run-dac-test-pi.ps1
  ```
- **Check Pi Network Interface & Latency**:
  ```powershell
  .\scripts\check-pi-network.ps1
  ```
- **Check SD Card Read/Write Health**:
  ```powershell
  .\scripts\check-sd-card.ps1
  ```
