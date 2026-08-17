# Remote Docker Build Troubleshooting & Recovery Guide

This guide summarizes technical post-mortems and step-by-step instructions for diagnosing, resetting, and recovering the remote ARM64 Docker build server (`joel@192.168.1.110`).

---

## 1. Post-Mortem & Known Failure Modes

### A. Cargo Target Cache Lock Deadlock (`.cargo-lock`)
- **Symptom:** Build hangs indefinitely at step `[build 4/4] RUN cargo build...` with zero output or CPU activity.
- **Root Cause:** When multiple builds or terminal sessions run concurrently, Cargo creates an exclusive lock file (`/project/target/.cargo-lock`) in the shared Docker cache volume (`cargo_target_cache`). If a build process is interrupted or killed mid-pass, stale lock files block all subsequent builds permanently.

### B. Windows OpenSSH Stdio Timeout (`0xffffffff` / `Connection reset`)
- **Symptom:** Terminal exits with `exit status 0xffffffff` or `client_loop: send disconnect: Connection reset` at ~300 seconds.
- **Root Cause:** Windows OpenSSH drops `dial-stdio` gRPC pipes if no output is received for 300 seconds (e.g. during heavy LLVM machine code linking or DWARF symbol generation).

### C. QEMU Futex Concurrency Deadlock
- **Symptom:** Cargo enters unkillable kernel sleep (`futex_wait_queue_me`) with 0% CPU.
- **Root Cause:** QEMU user-mode emulation (`qemu-aarch64`) on multi-core x86_64 CPUs in WSL2 can deadlock when executing ARM64 atomic memory instructions (`ldxr`/`stxr`).

---

## 2. Server Reset & Recovery Instructions

If a build hangs or encounters connection drops on `192.168.1.110`, follow these recovery steps:

### Quick Reset: Clear Stale Locks & Stale Containers
Run this command in PowerShell to kill stale compiler processes, clear old containers, and unlock the Cargo cache volume:

```powershell
ssh joel@192.168.1.110 "powershell -Command `"docker container prune -f; docker builder prune -f; wsl -d docker-desktop sh -c 'pkill -9 -f cargo; pkill -9 -f rustc; rm -f /var/lib/docker/volumes/cargo_target_cache/_data/.cargo-lock'`""
```

### Deep Reset: Restart Remote Docker & WSL2
If the remote Docker engine becomes unresponsive, restart the Docker Desktop service and WSL2 subsystem on `192.168.1.110`:

```powershell
ssh joel@192.168.1.110 "powershell -Command `"Restart-Service docker -Force; wsl --shutdown`""
```

---

## 3. Clean Build Execution

After resetting the server, launch a clean single-pass build from your workspace:

```powershell
.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost joel@192.168.1.110
```

- **Output Destination:** Compiled ARM64 binaries will be exported to `dist\pi\server` and `dist\pi\libHeliosLaserDAC.so`.
