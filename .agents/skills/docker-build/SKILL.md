---
name: docker-build
description: Build Linux ARM64 (Raspberry Pi 4) binaries remotely using Docker on a fast local PC workstation. Use when building or packaging server binaries for Pi deployment.
---

# Docker ARM64 Cross-Compilation Skill

This skill provides step-by-step instructions for cross-compiling Linux ARM64 (`aarch64-unknown-linux-gnu`) binaries for the Raspberry Pi 4 using a fast local Docker build host (`192.168.1.110`).

> [!NOTE]
> Compiling directly on physical Raspberry Pi hardware is slow and risks thermal throttling. Always use this remote Docker pipeline for production ARM64 builds.

---

## 1. Prerequisites

- **Build Host**: Windows PC at `192.168.1.110` (SSH user: `joel`).
- **Docker Engine**: Docker Desktop running on the build host with Linux container support enabled.
- **SSH Key**: Passwordless SSH key configured for `joel@192.168.1.110`.

---

## 2. Standard Remote Docker Build Workflow

To trigger a remote ARM64 cross-compilation run:

```powershell
.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost joel@192.168.1.110
```

### What this script does:
1. Syncs current workspace source code (`server`, `common`, `laserlogic`, `gamepad`, `minigames/*`) to the remote build machine.
2. Spawns/attaches to the `lasertargets-builder` Docker container (`cross-rs` `aarch64-unknown-linux-gnu` toolchain).
3. Runs `cargo build --release --target aarch64-unknown-linux-gnu --package server`.
4. Copies target output artifacts back to local workspace build directory.

---

## 3. Monitoring & Helper Scripts

- **Watch Live Build Progress**:
  ```powershell
  .\scripts\watch-remote-build.ps1
  ```
- **Ensure Remote Docker Service is Running**:
  ```powershell
  .\scripts\ensure-remote-docker.ps1
  ```
- **Restart Remote Docker Container**:
  ```powershell
  .\scripts\restart-remote-docker.ps1
  ```

---

## 4. Troubleshooting & Known Gotchas

- **Docker Container Stuck or File Lock**:
  Run `.\scripts\restart-remote-docker.ps1` to stop lingering cargo build processes and clear stale container locks.
- **Helios DAC Library Requirement**:
  Ensure `libHeliosLaserDAC.so` is included in the deployment bundle when transferring compiled binaries to the Pi.
