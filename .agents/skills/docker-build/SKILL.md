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

This workstation has no local Docker installed; builds run inside a Docker container on the dedicated remote build server (`joel@192.168.1.110`).

To trigger a remote ARM64 cross-compilation run:

```powershell
# Via xtask:
cargo xtask build-arm64

# Or via PowerShell script:
.\scripts\build-arm64.ps1
```

### What this does:
1. Packages local workspace source code via `tar` into a temporary archive (excluding `target`, `dist`, `.git`).
2. Transfers archive to the remote Docker build host (`joel@192.168.1.110`) via `scp`.
3. Unpacks and runs `docker run` on the remote server mounting `cargo_registry_cache` and `cargo_target_cache` volumes in the `lasertargets-cross-aarch64` container to compile `server` and `dac-test`.
4. Copies compiled ARM64 binaries (`server`, `dac-test`, `libHeliosLaserDAC.so`) via `scp` into local `dist\pi\`.

---

## 3. Monitoring & Helper Scripts

- **Watch Live Build Progress**:
  ```powershell
  .\scripts\remote-docker.ps1 watch -RemoteHost joel@192.168.1.110
  ```
- **Ensure Remote Docker Service is Running**:
  ```powershell
  .\scripts\remote-docker.ps1 status -RemoteHost joel@192.168.1.110
  ```
- **Restart Remote Docker Container**:
  ```powershell
  .\scripts\remote-docker.ps1 restart -RemoteHost joel@192.168.1.110
  ```

---

## 4. Troubleshooting & Known Gotchas

- **Docker Container Stuck or File Lock**:
  Run `.\scripts\remote-docker.ps1 restart -RemoteHost joel@192.168.1.110` to stop lingering cargo build processes and clear stale container locks.
- **Helios DAC Library Requirement**:
  Ensure `libHeliosLaserDAC.so` is included in the deployment bundle when transferring compiled binaries to the Pi.
