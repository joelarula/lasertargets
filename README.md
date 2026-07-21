# lasertargets

Augmented reality laser game platform.

## Local Development Build

Server:

cargo build --package server --features bevy/dynamic_linking
cargo run --package server --features bevy/dynamic_linking

Terminal:

cargo build --package terminal --features bevy/dynamic_linking
cargo run --package terminal --features bevy/dynamic_linking

## Raspberry Pi 4 Build (Docker)

The Raspberry Pi flow is Docker-first and exports deployable artifacts.

### Local Docker host

Run:

./scripts/build-pi.sh

Outputs:

dist/pi/server
dist/pi/libHeliosLaserDAC.so

### Remote Docker host (PowerShell)

Run:

./scripts/docker-build-rpi4-remote.ps1 -RemoteHost 192.168.1.110

Useful options:

-BuildProgress plain
-NoCache $true
-LocalArtifactDir .\\dist\\pi
-ExportArtifact $true

This produces the same deployable files under dist/pi.

## Raspberry Pi 4 Deploy

After building artifacts, deploy to Pi:

./scripts/deploy-pi.sh raspberrypi.local

Or:

./scripts/deploy-pi.sh pi@192.168.1.50

The deploy script installs:

/opt/lasertargets/server
/opt/lasertargets/lib/libHeliosLaserDAC.so

And updates/starts:

deploy/lasertargets-server.service

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