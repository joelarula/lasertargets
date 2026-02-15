# Terminal-Specific Instructions

**Applies to:** `terminal/**/*.rs`

## Context
You are working on the LaserTargets terminal (client) application. This provides the UI for configuration, calibration, and game control, and connects to the server via QUIC.

## Key Responsibilities
- User interface (Bevy UI for toolbar, egui for settings only)
- Scene editor and configuration
- Calibration tools
- Visualization of game state
- Input handling (mouse, keyboard)
- Client-side prediction/rendering

## Important Patterns

### Plugin Organization
All terminal plugins are in `terminal/src/plugins/`:

**Naming Convention**: When a feature has counterparts in server/common, use the **same module name**:
- `game.rs` ↔ `server/src/plugins/game.rs` ↔ `common/src/game.rs`
- `scene.rs` ↔ `server/src/plugins/scene.rs` ↔ `common/src/scene.rs`
- `calibration.rs` ↔ `server/src/plugins/calibration.rs` ↔ `common/src/config.rs`

Terminal plugins:
- `network.rs`: Client connection and message handling
- `toolbar.rs`: Main UI toolbar (Bevy UI)
- `scene.rs`: Scene editor
- `calibration.rs`: Calibration tools and visualization
- `settings.rs`: Settings panel (egui - see [settings.instructions.md](settings.instructions.md))
- `projector.rs`: Projector preview
- `game.rs`: Game UI integration
- `mouse.rs`, `keyboard.rs`: Input handling
- `target.rs`, `basictarget.rs`: Target management

### UI Framework
- **Toolbar**: Bevy native UI (Node, Button, Text) - see [toolbar.instructions.md](toolbar.instructions.md)
- **Settings**: egui immediate mode GUI - see [settings.instructions.md](settings.instructions.md)
- **Other UI elements**: Primarily Bevy UI with some egui for complex forms

### Network Communication
- Connect to server in `NetworkPlugin`
- Query state from server on startup
- Send updates to server when config changes
- Listen for broadcast updates from server

Example pattern:
```rust
fn sync_config_to_server(
    config: Res<MyConfig>,
    mut client: ResMut<ClientEndpoint>,
) {
    if config.is_changed() && !config.is_added() {
        client.send_message(NetworkMessage::UpdateMyConfig(config.clone()));
    }
}
```

### State Synchronization
- Terminal displays server state (not authoritative)
- Use `TerminalState` for connection status
- Cache server responses in Resources
- Re-query on reconnection

### Input Handling
- Mouse and keyboard input captured in dedicated plugins
- Convert screen coordinates to world coordinates
- Send input events to server for processing
- Local prediction for responsiveness (optional)

## Rendering

### Bevy Rendering
- Uses `DefaultPlugins` with window
- `PresentMode::AutoNoVsync` for responsive UI
- 2D camera for scene visualization

### Lyon for Shapes
- Use `bevy_prototype_lyon` for 2D vector shapes
- Render paths, targets, and scene elements
- Convert lyon paths to Bevy entities

## Development Workflow

### Running the Terminal
Always use dynamic linking during development:
```bash
cargo run -p terminal --features bevy/dynamic_linking
```

This reduces compile times significantly. Only build without the flag for release.

## Testing
- Test UI logic separately from rendering
- Mock network responses in tests
- Test state synchronization logic
- Run tests: `cargo test -p terminal`

## Common Pitfalls
- ❌ Don't store authoritative game state here
- ❌ Don't implement game logic (belongs on server)
- ❌ Don't block the main thread with network calls
- ✅ Always query server state on connection
- ✅ Use Resources to cache server state
- ✅ Handle connection loss gracefully
- ✅ Provide visual feedback for async operations
