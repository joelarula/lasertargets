# Server-Specific Instructions

**Applies to:** `server/**/*.rs`

## Context
You are working on the headless LaserTargets server application. This runs at configured (defaults to 50) FPS using `ScheduleRunnerPlugin`. Servers is wher game is taking place, with authoritative state, game logic, and hardware control. It communicates with terminal clients via QUIC using `QuinnetServerPlugin`.

**Performance Focus**: Server is optimized for maximum speed, throughput, and small binary size. Always consider the impact of new dependencies on performance and binary size before adding them.

## Key Responsibilities
- Game logic execution on server side
- Hardware control (laser DAC communication)
- Network message handling for clients
- Game state management
- Scene management and synchronization
- Actor lifecycle management
- Calibration processing

## Important Patterns

### CRITICAL: Network Communication Pattern

**❌ NEVER do this in game plugins:**
```rust
// ❌ DON'T: Use QuinnetServer directly in game plugins
fn my_system(mut server: ResMut<QuinnetServer>) {
    let endpoint = server.get_endpoint_mut();
    endpoint.broadcast_payload(...);
}
```

**✅ ALWAYS do this instead:**
```rust
// ✅ DO: Raise events that network plugin handles

// 1. Define broadcast event in your game plugin
#[derive(Message, Debug, Clone)]
pub struct BroadcastMyEvent {
    pub session_id: Uuid,
    pub data: MyData,
}

// 2. Raise the event in your game logic
fn my_system(mut events: MessageWriter<BroadcastMyEvent>) {
    events.write(BroadcastMyEvent { /* ... */ });
}

// 3. Network plugin listens and broadcasts
// (in server/src/plugins/network.rs)
fn broadcast_my_events(
    mut server: ResMut<QuinnetServer>,
    mut events: MessageReader<BroadcastMyEvent>,
) {
    for event in events.read() {
        let message = NetworkMessage::MyUpdate { /* ... */ };
        endpoint.broadcast_payload(message.to_bytes());
    }
}
```

**Why?** This keeps networking logic in one place (network.rs), makes game plugins testable without network, and follows the internal message pattern used throughout the codebase.

### CRITICAL: Event Location

**Events used by BOTH client and server:**

```rust
// ❌ DON'T: Define minigame events in server crate
// minigames/hunter/src/server.rs
#[derive(Message, Debug, Clone)]
pub struct HunterClickEvent { /* ... */ }  // ❌ NO!

// ❌ DON'T: Define minigame events in common crate
// common/src/game.rs
#[derive(Message, Debug, Clone)]
pub struct HunterClickEvent { /* ... */ }  // ❌ NO!

// ✅ DO: Define minigame events in minigame model.rs
// minigames/hunter/src/model.rs
#[derive(Message, Debug, Clone, Serialize, Deserialize)]
pub struct HunterClickEvent { /* ... */ }  // ✅ YES!
```

**Generic events used across multiple features go in common:**
```rust
// ✅ OK: Generic game session events in common
// common/src/game.rs
#[derive(Message, Debug, Clone)]
pub struct GameSessionCreated { /* ... */ }  // Used by all games
```

**Server-only events stay in server crate:**
```rust
// ✅ OK: Server-only event
#[derive(Message, Debug, Clone)]
pub struct SpawnHunterTargetEvent { /* ... */ }  // Only used by server systems
```

### Fixed Timestep
- Server runs at 50 FPS: `FIXED_TIMESTEP = 1.0 / 50.0`
- Use `FixedUpdate` schedule for deterministic game logic
- Use `Update` schedule for network I/O and non-deterministic tasks

### Plugin Organization
All server plugins are in `server/src/plugins/`:

**Naming Convention**: When a feature has counterparts in terminal/common, use the **same module name**:
- `game.rs` ↔ `terminal/src/plugins/game.rs` ↔ `common/src/game.rs`
- `scene.rs` ↔ `terminal/src/plugins/scene.rs` ↔ `common/src/scene.rs`
- `calibration.rs` ↔ `terminal/src/plugins/calibration.rs` ↔ `common/src/config.rs`

Server plugins:
- `network.rs`: QuinnetServerPlugin, handles all NetworkMessage variants
- `actor.rs`: Manages actor registration and lifecycle
- `game.rs`: Game session management and state transitions
- `projector.rs`: Laser DAC output and hardware control
- `scene.rs`: Scene state synchronization
- `path.rs`: Path synchronization with clients


### State Management
- Server owns the authoritative state
- Use `ServerState` and `GameState` (SubState of InGame)
- Broadcast state changes to all connected clients

## Hardware Integration

### Helios Laser DAC
- Windows-only DLL (`HeliosLaserDAC.dll`) in `server/libs/`
- `build.rs` copies DLL to target directory during compilation
- `server/src/dac/helios.rs` wraps the C API via `libloading`
- DAC outputs laser point coordinates for projection
- Wrap all DAC calls in proper error handling

### Projector Output
- Projector systems run in FixedUpdate
- Convert world coordinates to DAC coordinates
- Apply calibration transforms before sending to hardware

## Development Workflow

### Running the Server
Always use dynamic linking during development:
```bash
cargo run -p server --features bevy/dynamic_linking
```

This reduces compile times significantly. Only build without the flag for release.

### Cross-Compilation for Raspberry Pi 4
The server can be cross-compiled for Raspberry Pi 4 (aarch64) using `cross` with a custom Docker image that also builds `libHeliosLaserDAC.so` for ARM64.

**Prerequisites:**
- Docker installed and running
- `cargo install cross`

**Quick build (recommended):**
```bash
# Build everything (Docker image + cross-compile) in one step
./scripts/build-pi.sh
```

**Manual steps:**
```bash
# 1. Build the custom cross Docker image (includes Helios DAC C++ SDK for ARM64)
docker build -f docker/Dockerfile.aarch64 -t lasertargets-cross-aarch64 .

# 2. Cross-compile the server
cross build -p server --target aarch64-unknown-linux-gnu --release

# Binary: target/aarch64-unknown-linux-gnu/release/server
# Library: target/aarch64-unknown-linux-gnu/release/libHeliosLaserDAC.so
```

**Deployment:**
```bash
# Automated deployment to Pi (copies binary + .so + systemd service)
./scripts/deploy-pi.sh raspberrypi.local

# Or manual:
scp target/aarch64-unknown-linux-gnu/release/server pi@raspberrypi.local:/opt/lasertargets/
scp target/aarch64-unknown-linux-gnu/release/libHeliosLaserDAC.so pi@raspberrypi.local:/opt/lasertargets/lib/
```

**First-time Pi setup:**
```bash
# Run on the Pi to install libusb and configure USB permissions
ssh pi@raspberrypi.local 'sudo bash -s' < deploy/install-pi-deps.sh
```

**Configuration files:**
- `Cross.toml` — points `cross` to the custom Docker image
- `docker/Dockerfile.aarch64` — custom image with Helios C++ SDK cross-compiled for ARM64
- `deploy/lasertargets-server.service` — systemd unit file
- `deploy/install-pi-deps.sh` — one-time Pi dependency setup
- `scripts/build-pi.sh` — build automation
- `scripts/deploy-pi.sh` — deployment automation

## Testing
- Integration tests in `server/test/`
- Use `create_server_app()` from `lib.rs` for test apps
- Mock DAC hardware in tests (don't require actual hardware)
- Run tests: `cargo test -p server`

## Common Pitfalls
- ❌ Don't use `DefaultPlugins` (this is headless)
- ❌ Don't add window or rendering systems
- ❌ Don't trust client input without validation
- ✅ Always broadcast authoritative state changes
- ✅ Use Fixed timestep for game logic
- ✅ Handle disconnections gracefully
