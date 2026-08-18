# lasertargets

Augmented reality laser game platform.

> 📖 **Developer & Stability Guidelines**: See [INSTRUCTIONS.md](INSTRUCTIONS.md) for USB DAC hardware rules, Bevy ECS conventions, and network state invariants.

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

*If logging to systemd:*
```bash
ssh lasertargets@<IP> 'sudo journalctl -u lasertargets-server -f'
```

*If logging to USB stick:*
```bash
ssh lasertargets@<IP> 'tail -f /mnt/usb-logs/server.log'
```

## Game Console Controls

When a game console controller (Xbox / PlayStation / DirectInput / XInput) is connected to the server, the following button mappings are available:

| Button | Action | Description |
| :--- | :--- | :--- |
| **A (South)** | **Cycle Target Selection** | Cycles through reticle modes in Hunter game (GunShot $\rightarrow$ Red Circle $\rightarrow$ Yellow Balloon $\rightarrow$ Cyan Circle $\rightarrow$ Magenta Balloon) |
| **B (East) / RT** | **Spawn Target / Shoot** | Spawns selected target shape at cursor position (if target selected) or shoots at target (in GunShot mode) |
| **X (West)** | **Game Menu Switcher** | Cycles through Hunter $\rightarrow$ Snake $\rightarrow$ Main Menu |
| **Y (North)** | **Calibration Toggle** | Toggles Calibration overlay On / Off |
| **LB / RB** | **Target Size / Distance** | Increases / decreases target radius in Hunter game, or adjusts scene distance (in Calibration mode) |
| **Start** | **Laser Power Toggle** | Toggles laser projector power output On / Off |
| **Select** | **Status Report** | Logs diagnostic status report of server mode, peripherals, and network connections |
| **DPad Up / Down** | **Adjust Height / Direction** | Adjusts scene height (in Calibration mode) or controls Snake direction |
| **DPad Left / Right** | **Adjust Width / Direction** | Adjusts scene width (in Calibration mode) or controls Snake direction |