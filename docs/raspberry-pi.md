# Raspberry Pi 4 â€” Build, Deploy & Run

Complete guide for cross-compiling the LaserTargets server for Raspberry Pi 4, deploying it over SSH, and capturing its output live in your terminal.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [One-Time Raspberry Pi Setup](#2-one-time-raspberry-pi-setup)
   - [Flash the SD Card with Raspberry Pi Imager](#20-flash-the-sd-card-with-raspberry-pi-imager)
   - [Enable SSH (manual fallback)](#21-enable-ssh-manual-fallback)
   - [Find the Pi IP Address](#22-find-the-pis-ip-address)
   - [Set Up SSH Key Authentication](#23-set-up-ssh-key-authentication)
   - [Install Runtime Dependencies on the Pi](#24-install-runtime-dependencies-on-the-pi)
3. [Cross-Compile for Raspberry Pi 4](#3-cross-compile-for-raspberry-pi-4)
   - [How It Works](#31-how-it-works)
   - [Build â€” One Command](#32-build--one-command)
   - [Build â€” Manual Steps](#33-build--manual-steps)
   - [Windows Remote Docker Host](#34-windows-remote-docker-host)
4. [Deploy to the Pi](#4-deploy-to-the-pi)
   - [What the Script Does](#41-what-the-script-does)
   - [Manual Deploy Reference](#42-manual-deploy-reference)
5. [Run and Capture Output](#5-run-and-capture-output)
   - [Option A Run Directly Foreground](#51-option-a-run-directly-foreground)
   - [Option B Run as a systemd Service](#52-option-b-run-as-a-systemd-service)
   - [Useful Service Commands](#53-useful-service-commands)
6. [Troubleshooting SSH](#6-troubleshooting--ssh)
7. [Troubleshooting Runtime and Service](#7-troubleshooting--runtime--service)
8. [File Reference](#8-file-reference)

---

## 1. Prerequisites

### Developer machine (where you build)

| Requirement | Notes |
|---|---|
| **Docker** | Docker Desktop (Windows/macOS) or Docker Engine (Linux). Must be running. |
| **Git Bash / WSL / POSIX shell** | Required on Windows to run `.sh` scripts. Git Bash ships with Git for Windows. |
| **SSH client** (`ssh`, `scp`) | Included with Windows 10+, macOS, Linux. Verify: `ssh -V` |
| **Rust toolchain** | **Not required on the host.** Rust lives entirely inside the Docker cross-compilation image. |

### Raspberry Pi 4

| Requirement | Notes |
|---|---|
| Raspberry Pi OS (64-bit) | Bookworm recommended. The binary targets `aarch64-unknown-linux-gnu`. |
| SSH enabled | See [Section 2.1](#21-enable-ssh). |
| Connected to the same network | Ethernet or Wi-Fi. |

---

## 2. One-Time Raspberry Pi Setup

These steps are only needed once per Pi â€” not on every build/deploy cycle.

> **Readiness check scripts** â€” run these at each stage of setup:
>
> **While SD card is in your Windows machine (before first boot):**
> ```powershell
> .\scripts\check-sd-card.ps1               # checks D:\, creates ssh file if missing
> .\scripts\check-sd-card.ps1 -BootDrive E: # if bootfs is on a different drive letter
> ```
>
> **After the Pi has booted and is on the network:**
> ```powershell
> .\scripts\check-pi-network.ps1 -PiHost 192.168.1.50    # raw IP (most reliable)
> .\scripts\check-pi-network.ps1 -PiHost lasertargets.local # mDNS hostname
> ```
> The network script checks ping, SSH key auth, runtime dependencies, deployed binary, and service status.

### 2.0 Flash the SD Card with Raspberry Pi Imager

**Raspberry Pi Imager** is the official flashing tool. Its built-in **OS Customisation** lets
you configure SSH, your username/password, Wi-Fi, hostname, and SSH public key all before
the first boot â€” no keyboard or monitor needed, and no manual file editing on `D:\`.

#### Step 1 â€” Download and install Raspberry Pi Imager

Download from: **https://www.raspberrypi.com/software/**

Install and launch it on Windows.

---

#### Step 2 â€” Choose the device

Click **Choose Device** â†’ select **Raspberry Pi 4**.

---

#### Step 3 â€” Choose the OS

Click **Choose OS**. The top of the list shows the full desktop versions â€” **Lite is one level deeper**:

1. Scroll down and click **"Raspberry Pi OS (other)"**
2. In the submenu, select **"Raspberry Pi OS Lite (64-bit)"**

```
Choose OS
  â”œâ”€ Raspberry Pi OS (64-bit)          â† full desktop â€” skip this
  â”œâ”€ Raspberry Pi OS (32-bit)          â† skip this
  â””â”€ Raspberry Pi OS (other)           â† click here
       â”œâ”€ Raspberry Pi OS (64-bit)     â† full desktop
       â”œâ”€ Raspberry Pi OS Lite (64-bit)  â† âœ… pick this one
       â”œâ”€ Raspberry Pi OS Lite (32-bit)  â† skip (32-bit won't run our binary)
       â””â”€ Raspberry Pi OS Full (64-bit)  â† skip (has extra apps, not needed)
```

> **Why Lite?** The server runs headless â€” no desktop is needed.
> Lite is smaller, boots faster, and uses less RAM.
>
> **Why 64-bit?** The cross-compiled binary targets `aarch64-unknown-linux-gnu`
> which requires a 64-bit kernel (`kernel8.img`). The 32-bit variants will not run it.
>
> **Alternative:** If you cannot find Lite, the regular **"Raspberry Pi OS (64-bit)"**
> (first item in the list) also works â€” it just includes a desktop that will never be used.

---

#### Step 4 â€” Choose the SD card

Click **Choose Storage** â†’ select your SD card.

> âš ï¸ Double-check the drive letter and size â€” the card will be completely erased.

---

#### Step 5 â€” OS Customisation (the important part)

Click **Next**. Imager will ask:

> *Would you like to apply OS customisation settings?*

Click **Edit Settings**. Fill in the following tabs:

**General tab:**

| Field | Recommended value |
|---|---|
| **Set hostname** | `lasertargets` (or any name you like) |
| **Set username and password** | Username: `lasertargets` Â· Password: choose a strong one |
| **Configure wireless LAN** | Enter your Wi-Fi SSID and password (if using Wi-Fi) |
| **Wireless LAN country** | Set to your country code (e.g., `FI`, `US`, `DE`) |
| **Set locale settings** | Set timezone and keyboard layout |

**Services tab:**

| Field | Value |
|---|---|
| **Enable SSH** | âœ… Check this |
| Authentication | Select **"Allow public-key authentication only"** (recommended) |
| **Authorised keys** | Paste the contents of `~/.ssh/id_ed25519.pub` (or `id_rsa.pub`) |

To get your public key:

```powershell
# In PowerShell:
Get-Content $env:USERPROFILE\.ssh\id_ed25519.pub
```

Copy the entire line (starts with `ssh-ed25519 ...`) and paste it into the Authorised keys field.

> **No SSH key yet?** Generate one first:
> ```powershell
> ssh-keygen -t ed25519 -C "lasertargets-deploy"
> ```
> Then re-open Imager settings and paste the key.

**Options tab:**

| Field | Value |
|---|---|
| **Eject media when finished** | âœ… Recommended |
| **Enable telemetry** | Your preference |

Click **Save**, then click **Yes** to apply.

---

#### Step 6 â€” Write the image

Click **Yes** on the confirmation dialog. Imager will:
1. Download the OS image (if not cached)
2. Write it to the SD card
3. Verify the write
4. Eject the card

This takes ~5â€“10 minutes depending on your internet speed and card speed.

---

#### Step 7 â€” Verify with the check script

Before ejecting (or after re-inserting), confirm everything looks correct:

```powershell
.\scripts\check-sd-card.ps1
```

Expected output:
```
  [PASS] Drive D: is mounted
  [PASS] Raspberry Pi bootfs detected
  [PASS] 64-bit kernel present (kernel8.img)
  [PASS] SSH enable file exists
  [PASS] D: is writable
```

> If the Imager already applied OS Customisation correctly, the `ssh` file will already
> exist on `D:\` â€” the check script will confirm this.

---

#### Step 8 â€” First boot

1. Insert the SD card into the Pi
2. Connect Ethernet (recommended for first boot) or rely on the Wi-Fi you configured
3. Power on
4. Wait ~60 seconds for the first boot to complete
5. Verify connectivity:

```powershell
.\scripts\check-pi-network.ps1 -PiHost lasertargets.local
# or use the raw IP:
.\scripts\check-pi-network.ps1 -PiHost 192.168.1.50
```

If key auth was configured in Imager, SSH login will work immediately â€” no `ssh-copy-id` needed.

---

### 2.1 Enable SSH (manual fallback)

**Option A â€” Headless via SD card (recommended â€” no keyboard/monitor needed)**

When flashing the SD card on Windows, the `bootfs` partition is mounted as `D:\`.
Create an empty file named `ssh` (no extension, no content) in the root of that partition:

```powershell
# In PowerShell â€” with bootfs mounted as D:
New-Item -Path D:\ssh -ItemType File
```

Or in File Explorer: right-click the `D:\` root â†’ New â†’ Text Document â†’ rename it to `ssh`
(make sure Windows is not hiding the `.txt` extension â€” the file must have **no extension at all**).

Eject and insert the SD card. SSH will be enabled on first boot.

**Option B â€” Via `raspi-config` (on the Pi, if you have keyboard/monitor)**

```bash
sudo raspi-config
# Navigate: Interface Options â†’ SSH â†’ Enable
```

**Option C â€” Via Desktop**

Preferences â†’ Raspberry Pi Configuration â†’ Interfaces â†’ SSH â†’ Enable

**Verify SSH is active after boot:**

```bash
ssh lasertargets@<IP>
# You should get a shell prompt. Type 'exit' to leave.
```

---

### 2.2 Find the Pi IP Address

**On the Pi itself:**

```bash
hostname -I
```

**From a router:** Check the DHCP client table (usually at `192.168.1.1` or `192.168.0.1`).

**Using mDNS (Avahi):** If Avahi is installed on the Pi, it can be reached as `lasertargets.local`:

```bash
ping lasertargets.local
```

> **Tip:** Assign a static IP via your router DHCP reservation so the address never changes between reboots.

---

### 2.3 Set Up SSH Key Authentication

> **Skip if you used Raspberry Pi Imager** and pasted your public key in the Services tab
> (Section [2.0 Step 5](#step-5--os-customisation-the-important-part)). Key auth is already
> configured on the Pi â€” verify with `.\scripts\check-pi-network.ps1 -PiHost <IP>`.

Password prompts during `deploy-pi.sh` interrupt automation. If you did not configure a key in Imager, set one up now:

```bash
# Generate a deploy key (skip if you already have one)
ssh-keygen -t ed25519 -C "lasertargets-deploy"

# Copy the public key to the Pi
ssh-copy-id lasertargets@<IP>

# Test â€” should log in without a password prompt
ssh lasertargets@<IP> 'echo "Key auth works"'
```

> **Windows PowerShell** (if `ssh-copy-id` is not available):
> ```powershell
> type $env:USERPROFILE\.ssh\id_ed25519.pub | ssh lasertargets@<IP> "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
> ```

**Optional: create an SSH config entry** (`~/.ssh/config`) so you can type `ssh lasertargets` instead of `ssh lasertargets@<IP>`:

```
Host lasertargets
    HostName <IP>
    User lasertargets
    IdentityFile ~/.ssh/id_ed25519
    ServerAliveInterval 30
    ConnectTimeout 10
```

---

### 2.4 Install Runtime Dependencies on the Pi

Run this **once** from the project root on your developer machine:

```bash
scp deploy/install-pi-deps.sh lasertargets@<IP>:/tmp/
ssh lasertargets@<IP> sudo bash /tmp/install-pi-deps.sh
```

What it does:

- Installs `libusb-1.0-0` (required by the Helios DAC)
- Creates `/opt/lasertargets/lib/` and sets ownership to `lasertargets`
- Installs udev rule `/etc/udev/rules.d/99-helios-dac.rules` so the Helios DAC USB device is accessible without root

After the script, add `lasertargets` to the `plugdev`group (required for USB device access):

```bash
ssh lasertargets@<IP> 'sudo usermod -aG plugdev lasertargets'
```

> The group change takes effect at next login â€” reboot the Pi or log out and back in.

---

## 3. Cross-Compile for Raspberry Pi 4

### 3.1 How It Works

The build uses two Docker images chained together:

```
docker/Dockerfile.aarch64
  â†’ Image: lasertargets-cross-aarch64
    Contains: Rust toolchain, aarch64-linux-gnu-gcc, libusb (arm64), Helios DAC SDK (ARM64)

docker/Dockerfile.rpi4
  â†’ Image: lasertargets-server-rpi4-artifact
    Runs: cargo build -p server --target aarch64-unknown-linux-gnu --release
    Outputs: /dist/server, /dist/libHeliosLaserDAC.so
```

**No `cross` CLI tool is required.** The custom `Dockerfile.aarch64` replaces it entirely â€” all
build tooling is self-contained inside Docker.

---

### 3.2 Build â€” One Command

From the project root (Git Bash, WSL, or Linux/macOS terminal):

```bash
./scripts/build-pi.sh
```

The script:
1. Prunes dangling Docker images and stopped containers (frees disk space)
2. Builds the `lasertargets-cross-aarch64` cross-toolchain image
3. Builds the `lasertargets-server-rpi4-artifact` image (compiles the server inside Docker)
4. Extracts artifacts to `dist/pi/`

Expected artifacts:

```
dist/pi/server                  â† ARM64 ELF binary
dist/pi/libHeliosLaserDAC.so    â† Helios DAC shared library (ARM64)
```

> **First build** takes 15â€“30 min (downloads base images, compiles Helios DAC SDK).
> **Subsequent builds** are fast â€” Docker caches intermediate layers.

---

### 3.3 Build â€” Manual Steps

For debugging or when you need finer control:

**Step 1: Build the cross-compilation toolchain image**

Only needed once, or when `docker/Dockerfile.aarch64` changes:

```bash
docker build \
  -f docker/Dockerfile.aarch64 \
  -t lasertargets-cross-aarch64 \
  .
```

**Step 2: Compile the server inside Docker**

```bash
docker build \
  --build-arg BASE_IMAGE=lasertargets-cross-aarch64 \
  --build-arg TARGET_TRIPLE=aarch64-unknown-linux-gnu \
  -f docker/Dockerfile.rpi4 \
  -t lasertargets-server-rpi4-artifact \
  .
```

Force a full recompile:

```bash
docker build --no-cache \
  --build-arg BASE_IMAGE=lasertargets-cross-aarch64 \
  --build-arg TARGET_TRIPLE=aarch64-unknown-linux-gnu \
  -f docker/Dockerfile.rpi4 \
  -t lasertargets-server-rpi4-artifact \
  .
```

**Step 3: Extract artifacts from the image**

```bash
mkdir -p dist/pi
CID=$(docker create lasertargets-server-rpi4-artifact)
docker cp "$CID:/dist/." dist/pi/
docker rm "$CID"

# Verify
ls -lh dist/pi/
```

---

### 3.4 Windows: Remote Docker Host

If you want to offload the build to a remote Windows machine (e.g., `192.168.1.110`), the build runs via the SSH daemon.

#### Step 1 — Ensure Remote Docker is Running (Headless)
Since logging out of an SSH session normally terminates user processes on Windows, Docker Desktop must be launched via a WMI/CIM detached process to keep it running persistently.

A startup script has been placed on the remote machine at `C:/Users/joel/start-docker.ps1`. Trigger it remotely over SSH:

```powershell
ssh joel@192.168.1.110 "powershell -ExecutionPolicy Bypass -File C:/Users/joel/start-docker.ps1"
```
*(Wait ~15 seconds after starting for the VM engine to fully boot).*

#### Step 2 — Run the Build from the Project Root
> ⚠️ **CRITICAL:** You must run this command from the **project root directory** (`C:\Users\joela\dev\lasertargets`), NOT from inside the `scripts/` folder, otherwise Docker will not be able to find the context files.

```powershell
# Go to project root
cd C:\Users\joela\dev\lasertargets

# Ensure local DOCKER_HOST env var is clean
Remove-Item Env:\DOCKER_HOST -ErrorAction SilentlyContinue

# Start remote build
.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost joel@192.168.1.110
```

| Parameter | Default | Description |
|---|---|---|
| `-RemoteHost` | *(empty = local Docker)* | IP or hostname of the remote Docker host |
| `-BuildProgress` | `auto` | Use `plain` for verbose line-by-line output |
| `-NoCache` | `$false` | Set `$true` to force a full rebuild |
| `-ExportArtifact` | `$true` | Copy artifacts locally after the build |
| `-LocalArtifactDir` | `.\dist\pi` | Local destination for extracted artifacts |
| `-DryRun` | *(switch)* | Print commands without running them |

---


## 4. Deploy to the Pi

After a successful build:

```bash
./scripts/deploy-pi.sh lasertargets@<IP>

# mDNS hostname:
./scripts/deploy-pi.sh lasertargets@lasertargets.local

# User defaults to 'lasertargets' if omitted:
./scripts/deploy-pi.sh lasertargets.local
```

### 4.1 What the Script Does

| Step | Action |
|---|---|
| 1 | `systemctl stop lasertargets-server` â€” stops any running instance |
| 2 | `mkdir -p /opt/lasertargets/lib` â€” ensures directory structure exists |
| 3 | Copies `dist/pi/server` â†’ `/opt/lasertargets/server` |
| 4 | Copies `dist/pi/libHeliosLaserDAC.so` â†’ `/opt/lasertargets/lib/` |
| 5 | Copies `deploy/lasertargets-server.service` â†’ `/etc/systemd/system/` |
| 6 | `systemctl daemon-reload && systemctl enable lasertargets-server` |
| 7 | `systemctl start lasertargets-server` |
| 8 | Prints `systemctl status` output |

### 4.2 Manual Deploy Reference

```bash
PI=lasertargets@<IP>

ssh $PI 'sudo systemctl stop lasertargets-server 2>/dev/null || true'

scp dist/pi/server              $PI:/opt/lasertargets/server
scp dist/pi/libHeliosLaserDAC.so $PI:/opt/lasertargets/lib/
ssh $PI 'chmod +x /opt/lasertargets/server'

scp deploy/lasertargets-server.service $PI:/tmp/
ssh $PI 'sudo mv /tmp/lasertargets-server.service /etc/systemd/system/ \
  && sudo systemctl daemon-reload \
  && sudo systemctl enable lasertargets-server \
  && sudo systemctl start lasertargets-server'
```

---

## 5. Run and Capture Output

### 5.1 Option A: Run Directly (Foreground)

Runs the server in the foreground. All stdout and stderr stream directly to your terminal.
`Ctrl+C` stops the server.

```bash
ssh lasertargets@<IP> 'LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/server'
```

With verbose logging:

```bash
ssh lasertargets@<IP> 'RUST_LOG=debug LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/server'
```

Available log levels: `error` `warn` `info` (default) `debug` `trace`

---

### 5.2 Option B: Run as a systemd Service

The deployed service auto-starts on boot and restarts on crash.

**Start the service and immediately follow its output in your terminal:**

```bash
ssh lasertargets@<IP> 'sudo systemctl restart lasertargets-server && sudo journalctl -u lasertargets-server -f'
```

This is the **recommended approach for normal operation** â€” the server stays alive even if your SSH session drops. Press `Ctrl+C` to stop following logs; the server keeps running.

**Follow logs only (without restarting the service):**

```bash
ssh lasertargets@<IP> 'sudo journalctl -u lasertargets-server -f'
```

---

### 5.3 Useful Service Commands

```bash
PI=lasertargets@<IP>

# Start / stop / restart
ssh $PI 'sudo systemctl start   lasertargets-server'
ssh $PI 'sudo systemctl stop    lasertargets-server'
ssh $PI 'sudo systemctl restart lasertargets-server'

# Status snapshot
ssh $PI 'sudo systemctl status lasertargets-server --no-pager'

# Live log stream
ssh $PI 'sudo journalctl -u lasertargets-server -f'

# Last 50 lines
ssh $PI 'sudo journalctl -u lasertargets-server -n 50 --no-pager'

# Logs from the last 10 minutes
ssh $PI 'sudo journalctl -u lasertargets-server --since "10 minutes ago" --no-pager'

# Disable auto-start on boot
ssh $PI 'sudo systemctl disable lasertargets-server'
```

The service unit lives at `deploy/lasertargets-server.service`.
Key settings: `RUST_LOG=info`, `LD_LIBRARY_PATH=/opt/lasertargets/lib`, restarts on failure up to 3 times per 60 seconds.

---

## 6. Troubleshooting â€” SSH

### `Connection refused` or `Connection timed out`

1. SSH enabled? On the Pi: `sudo systemctl status ssh`
2. Correct IP? On the Pi: `hostname -I`
3. Same network? From your machine: `ping <IP>`
4. Firewall blocking port 22?
   ```bash
   ssh $PI 'sudo ufw status'
   ssh $PI 'sudo ufw allow ssh'   # if ufw is active
   ```

### `Permission denied (publickey)`

SSH key not installed. Re-run:

```bash
ssh-copy-id lasertargets@<IP>
```

Windows PowerShell fallback:

```powershell
type $env:USERPROFILE\.ssh\id_ed25519.pub | ssh lasertargets@<IP> "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
```

### `Host key verification failed`

The Pi host key changed (e.g., SD card re-flashed). Remove the stale entry:

```bash
ssh-keygen -R <IP>
ssh-keygen -R lasertargets.local
```

Reconnect â€” accept the new host key prompt.

### mDNS hostname not resolving

- **Install Avahi on the Pi:** `sudo apt install avahi-daemon`
- **Windows:** Install [Bonjour Print Services](https://support.apple.com/kb/DL999) or use the raw IP
- **Linux developer machine:** `sudo apt install avahi-daemon`
- **Fallback:** Use the IP address directly instead of the hostname

### SSH is slow to connect

Caused by reverse DNS lookup. Fix: add `UseDNS no` to `/etc/ssh/sshd_config` on the Pi and restart SSH:

```bash
ssh $PI 'echo "UseDNS no" | sudo tee -a /etc/ssh/sshd_config && sudo systemctl restart ssh'
```

---

## 7. Troubleshooting â€” Runtime and Service

### `Exec format error` â€” binary does not run

Binary is the wrong architecture. Verify on the Pi:

```bash
file /opt/lasertargets/server
# Expected: ELF 64-bit LSB executable, ARM aarch64
```

If it shows `x86-64`, the wrong binary was copied. Rebuild with `./scripts/build-pi.sh` and redeploy.

### `libHeliosLaserDAC.so: cannot open shared object file`

The shared library is missing or not in the search path. Check:

```bash
# Does the library exist?
ls -lh /opt/lasertargets/lib/libHeliosLaserDAC.so

# Run manually with the path set
LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/server

# Is LD_LIBRARY_PATH in the service unit?
sudo systemctl cat lasertargets-server | grep LD_LIBRARY_PATH
```

If the library is missing from `dist/pi/`, check the build log for
`WARNING: libHeliosLaserDAC.so not found` and rebuild.

### Helios DAC USB device not detected

1. Check udev rule:
   ```bash
   cat /etc/udev/rules.d/99-helios-dac.rules
   ```
   Missing? Re-run `sudo bash /tmp/install-pi-deps.sh`.

2. Check group membership:
   ```bash
   groups lasertargets   # should include: plugdev
   sudo usermod -aG plugdev lasertargets   # then reboot
   ```

3. Verify the device is recognised:
   ```bash
   lsusb   # look for Helios or VID 1209
   ```

4. Reload udev rules:
   ```bash
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```

### Service keeps restarting

View the crash reason:

```bash
ssh $PI 'sudo journalctl -u lasertargets-server -n 100 --no-pager'
```

| Symptom in logs | Cause | Fix |
|---|---|---|
| `SIGABRT` / `thread panicked` | Application bug | Read the panic message |
| `cannot open shared object file` | Missing library | See section above |
| `Permission denied` | Binary not executable | `chmod +x /opt/lasertargets/server` |
| Exit code 1 immediately | Config error | Run manually with `RUST_LOG=debug` |

Stop the restart loop while debugging:

```bash
ssh $PI 'sudo systemctl stop lasertargets-server && sudo systemctl disable lasertargets-server'
# Run manually to see output:
ssh $PI 'RUST_LOG=debug LD_LIBRARY_PATH=/opt/lasertargets/lib /opt/lasertargets/server'
```

### `Permission denied` running the binary directly

```bash
ssh $PI 'chmod +x /opt/lasertargets/server'
```

---

## 8. File Reference

| File | Description |
|---|---|
| `docker/Dockerfile.aarch64` | Cross-compilation toolchain image. Installs Rust, `aarch64-linux-gnu-gcc`, `libusb` (arm64), and compiles the Helios DAC SDK for ARM64. |
| `docker/Dockerfile.rpi4` | Build image. Uses `Dockerfile.aarch64` as base, runs `cargo build -p server --release`, stages artifacts to `/dist/`. |
| `Cross.toml` | Declares `lasertargets-cross-aarch64` as the Docker image for `aarch64-unknown-linux-gnu` target (used if `cross` CLI is invoked). |
| `scripts/build-pi.sh` | One-command build. Runs the full Docker pipeline, extracts artifacts to `dist/pi/`. |
| `scripts/deploy-pi.sh` | Deploys `dist/pi/` to the Pi over SSH, installs and starts the systemd service. |
| `scripts/check-sd-card.ps1` | Validates SD card bootfs on `D:\` before first boot. Creates the `ssh` enable file if missing. |
| `scripts/check-pi-network.ps1` | Checks ping, SSH key auth, runtime deps, binary, and service status after the Pi is booted. |
| `scripts/docker-build-rpi4-remote.ps1` | PowerShell build script for a remote Docker host. |
| `scripts/docker-build-common.ps1` | Shared PowerShell helpers (image build, artifact export). |
| `deploy/install-pi-deps.sh` | One-time Pi setup. Installs `libusb`, creates `/opt/lasertargets`, sets up Helios DAC udev rules. |
| `deploy/lasertargets-server.service` | systemd unit. Sets `LD_LIBRARY_PATH`, `RUST_LOG=info`, redirects logs to USB, restarts on failure. |
| `deploy/lasertargets-logrotate` | logrotate configuration. Rotates the USB log file weekly with compression. |
| `dist/pi/server` | Compiled ARM64 binary (generated — not in git). |
| `dist/pi/libHeliosLaserDAC.so` | Helios DAC shared library for ARM64 (generated — not in git). |
| `/opt/lasertargets/server` | Deployed binary on the Pi. |
| `/opt/lasertargets/lib/libHeliosLaserDAC.so` | Deployed library on the Pi. |

---

## 9. Storing Logs on a USB Memory Stick (Recommended)

To protect the Pi's operating system SD card from wear-out and corruption caused by constant logging, write logs to a USB memory stick instead.

### 9.1 Prepare the USB stick on the Pi

1. Plug the USB stick into one of the Pi 4's USB ports (use a blue USB 3.0 port).
2. SSH into the Pi and run `lsblk` to identify the device partition (e.g. `sda1`).
3. Get the unique UUID of the partition:
   ```bash
   sudo blkid /dev/sda1
   ```
   *Look for `UUID="..."` in the output (e.g. `E3F4-9A2B`).*

### 9.2 Configure Auto-Mount on Boot

1. Create the mount directory:
   ```bash
   sudo mkdir -p /mnt/usb-logs
   sudo chown -R lasertargets:lasertargets /mnt/usb-logs
   ```
2. Open `/etc/fstab` with `nano`:
   ```bash
   sudo nano /etc/fstab
   ```
3. Add this line at the bottom (replace `E3F4-9A2B` with your actual UUID, and use `vfat` if formatted as FAT32, or `ext4` if ext4):
   ```text
   UUID=E3F4-9A2B  /mnt/usb-logs  vfat  defaults,nofail,noatime,uid=lasertargets,gid=lasertargets,umask=007  0  0
   ```
   *The `nofail` parameter ensures that the Pi boots up normally even if you unplug the USB stick.*
4. Test mount:
   ```bash
   sudo mount -a
   ls -la /mnt/usb-logs
   ```

### 9.3 Deploy & Verify

Once the mount path is ready on the Pi, deploy the updated systemd service and logrotate configurations from your workstation:

```powershell
# Deploy the files to the Pi
.\scripts\deploy-pi.sh lasertargets@lasertargets.local
```

The systemd service will output stdout/stderr logs directly to `/mnt/usb-logs/server.log`.

Verify the logs are writing to the USB stick:
```bash
ssh lasertargets@lasertargets.local "tail -f /mnt/usb-logs/server.log"
```

