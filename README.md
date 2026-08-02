# lasertargets

Augmented reality laser game platform.

## Local Development Build

Server:

cargo build --package server --features bevy/dynamic_linking
cargo run --package server --features bevy/dynamic_linking

Terminal:

cargo build --package terminal --features bevy/dynamic_linking
cargo run --package terminal --features bevy/dynamic_linking

## Raspberry Pi 4

> **Full guide**: [docs/raspberry-pi.md](docs/raspberry-pi.md) — SSH setup, cross-compilation,
> deploy, live output capture, and troubleshooting.

### Quick Start

**1. Build** (cross-compile for aarch64 inside Docker, no Rust toolchain needed on host):

*Local build (Linux/macOS/Git Bash):*
```bash
./scripts/build-pi.sh
```

*Remote build (Windows PowerShell):*
```powershell
.\scripts\docker-build-rpi4-remote.ps1 -RemoteHost joel@192.168.1.110
```

Outputs `dist/pi/server` and `dist/pi/libHeliosLaserDAC.so`.

**2. Deploy** (copy binary to Pi over SSH and install systemd service):

```bash
./scripts/deploy-pi.sh lasertargets@<IP>
```

**3. Follow live output** in your terminal:

```bash
ssh lasertargets@<IP> 'sudo journalctl -u lasertargets-server -f'
```

## Game Console Controls

When a game console controller (Xbox / PlayStation / DirectInput / XInput) is connected to the server, the following button mappings are available:

| Button | Action | Description |
| :--- | :--- | :--- |
| **A (South)** | **Status Report** | Logs diagnostic status report of server mode, peripherals, and network connections |
| **B (East)** | **Click / Shoot** | Fires virtual mouse click at gamepad cursor position |
| **X (West)** | **Laser Power Toggle** | Toggles laser projector output On / Off |
| **Y (North)** | **Calibration Toggle** | Toggles Calibration mode On / Off |
| **Start** | **Start Hunter Game** | Initializes a new Hunter game session |
| **Select** | **Start Snake / Exit Game** | Initializes Snake game (in Menu) or exits current active game session |
| **DPad Up / Down** | **Adjust Height** | Increases / decreases target scene height (when Calibration mode is ON) |
| **DPad Left / Right** | **Adjust Width** | Increases / decreases target scene width (when Calibration mode is ON) |
| **LB / RB** | **Adjust Distance** | Moves scene origin closer / further (when Calibration mode is ON) |
| **LT / RT** | **Adjust Altitude** | Shifts scene vertical offset up / down (when Calibration mode is ON) |