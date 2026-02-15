# AI Agent Instructions for LaserTargets

> **Quick Start**: Read [copilot-instructions.md](copilot-instructions.md) for architecture overview, then consult path-specific instructions in `.github/instructions/` for the area you're working in.

This guide provides comprehensive workflows, code examples, and troubleshooting for AI coding agents working with the LaserTargets codebase.

## Understanding This Project

### First Steps
1. **Read the main instructions**: Start with [copilot-instructions.md](copilot-instructions.md) for project overview
2. **Check path-specific instructions**: Files in `.github/instructions/` provide targeted guidance
3. **Understand the architecture**: This is a client-server augmented reality laser game platform with:
   - **Server** (headless, 50 FPS): Game logic + hardware control
   - **Terminal** (UI client): Configuration + visualization
   - **Common**: Shared data models + network protocol

### Architecture Quick Reference
```
┌─────────────┐                    ┌──────────────┐
│   Terminal  │◄──── QUIC/QUINNET ─┤    Server    │
│  (UI Client)│      Binary msgs   │  (Headless)  │
└─────────────┘                    └──────┬───────┘
                                          │
                                   ┌──────▼───────┐
                                   │  Helios DAC  │
                                   │ (Laser HW)   │
                                   └──────────────┘
```

## Path-Specific Guidance

The project has detailed instructions for different areas. **Always consult the relevant instruction file** before making changes:

| Working in... | Read... | Key Focus |
|---------------|---------|-----------|
| `server/` | [server.instructions.md](instructions/server.instructions.md) | Headless app, 50 FPS fixed, hardware control, authoritative state |
| `terminal/` | [terminal.instructions.md](instructions/terminal.instructions.md) | UI with egui, client networking, visualization only |
| `common/` | [common.instructions.md](instructions/common.instructions.md) | Data structures only, no logic, must serialize |
| `minigames/` | [minigames.instructions.md](instructions/minigames.instructions.md) | Plugin structure, model/common/server/terminal split |
| Network code | [network.instructions.md](instructions/network.instructions.md) | Message patterns, QUIC protocol, naming conventions |
| Calibration | [calibration.instructions.md](instructions/calibration.instructions.md) | Coordinate systems, transformations, UI/hardware sync |

## Making Changes: Step-by-Step

### 1. Understand the Request
- [ ] What is the user trying to achieve?
- [ ] Which crate(s) are affected? (server, terminal, common, minigame)
- [ ] Does this require network protocol changes?
- [ ] Will this affect both server and terminal?

### 2. Gather Context
```bash
# Find relevant files
rg "pattern" --type rust

# Check existing implementations
rg "similar_function" -A 5

# Review related types
rg "struct MyType" -A 10
```

### 3. Plan the Implementation
- [ ] Identify all files that need changes
- [ ] Check if new network messages are needed
- [ ] Determine if this needs state management
- [ ] Consider serialization requirements
- [ ] Plan testing approach
- [ ] Update workspace structure docs if adding/removing files or directories
- [ ] Update relevant instruction files if changing patterns or architecture

### 4. Implement Following Patterns
- **Plugins**: One feature = one plugin
- **Components**: Data with validation (constructors/getters OK), no game/business logic
- **Systems**: Pure functions operating on components (where game logic lives)
- **Resources**: Global state (derive `Resource`)
- **Messages**: Bevy 0.17 uses `#[derive(Message)]` not `Event`

### 5. Validate
- [ ] Code compiles: `cargo build`
- [ ] Follows project conventions (see copilot-instructions.md)
- [ ] Network messages properly serialized
- [ ] Both server and terminal updated if needed
- [ ] Documentation added for public APIs

## Common Workflows

### Adding a New Feature

<details>
<summary><b>1. Feature in Server Only</b></summary>

```rust
// server/src/plugins/myfeature.rs
pub struct MyFeaturePlugin;

impl Plugin for MyFeaturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, my_feature_system);
    }
}

fn my_feature_system() {
    // Server-side logic
}

// server/src/plugins/mod.rs
pub mod myfeature;

// server/src/lib.rs or main.rs
.add_plugins(MyFeaturePlugin)
```
</details>

<details>
<summary><b>2. Feature in Terminal Only</b></summary>

```rust
// terminal/src/plugins/myfeature.rs
use bevy_egui::{EguiContexts, egui};

pub struct MyFeaturePlugin;

impl Plugin for MyFeaturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, my_feature_ui);
    }
}

fn my_feature_ui(mut contexts: EguiContexts) {
    egui::Window::new("My Feature")
        .show(contexts.ctx_mut(), |ui| {
            // UI code
        });
}

// terminal/src/plugins/mod.rs
pub mod myfeature;

// terminal/src/main.rs
.add_plugins(MyFeaturePlugin)
```
</details>

<details>
<summary><b>3. Feature Requiring Network Sync</b></summary>

**Step 1**: Add message to common
```rust
// common/src/network.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // ...existing variants
    QueryMyFeature,
    UpdateMyFeature(MyFeatureData),
    MyFeatureUpdate(MyFeatureData),
}
```

**Step 2**: Add data type to common
```rust
// common/src/myfeature.rs
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct MyFeatureData {
    pub value: f32,
}

// common/src/lib.rs
pub mod myfeature;
```

**Step 3**: Handle on server
```rust
// server/src/plugins/network.rs
fn handle_client_messages(/* ... */) {
    match message {
        NetworkMessage::QueryMyFeature => {
            let data = my_feature.data.clone();
            endpoint.send_message(client_id, 
                NetworkMessage::MyFeatureUpdate(data));
        }
        NetworkMessage::UpdateMyFeature(data) => {
            commands.insert_resource(data.clone());
            endpoint.broadcast_message(
                NetworkMessage::MyFeatureUpdate(data));
        }
        // ...
    }
}
```

**Step 4**: Handle on terminal
```rust
// terminal/src/plugins/network.rs
fn handle_server_messages(/* ... */) {
    match message {
        NetworkMessage::MyFeatureUpdate(data) => {
            commands.insert_resource(data);
        }
        // ...
    }
}

// terminal/src/plugins/myfeature.rs
fn sync_changes(
    data: Res<MyFeatureData>,
    mut client: ResMut<ClientEndpoint>,
) {
    if data.is_changed() && !data.is_added() {
        client.send_message(
            NetworkMessage::UpdateMyFeature(data.clone()));
    }
}
```
</details>

### Creating a New Minigame

See [minigames.instructions.md](instructions/minigames.instructions.md) for complete guide. Quick checklist:

- [ ] Create crate: `minigames/mygame/`
- [ ] Implement modules: `model.rs`, `common.rs`, `server.rs`, `terminal.rs`
- [ ] Add to workspace `Cargo.toml` members
- [ ] Add dependency to `server/Cargo.toml` and `terminal/Cargo.toml`
- [ ] Register plugin in both `server/src/main.rs` and `terminal/src/main.rs`
- [ ] Register game in `GameRegistry`

### Debugging Issues

<details>
<summary><b>Compilation Errors</b></summary>

```bash
# Full rebuild
cargo clean
cargo build

# Check specific crate
cargo build -p server
cargo build -p terminal
cargo build -p common

# With better error messages
cargo build --message-format=json
```
</details>

<details>
<summary><b>Network Issues</b></summary>

```bash
# Enable network logging (with dynamic linking for faster rebuilds)
RUST_LOG=bevy_quinnet=debug,server=debug cargo run -p server --features bevy/dynamic_linking

# In code, add logging
debug!("Sending message: {:?}", message);
```

Check:
- [ ] Server is running and listening
- [ ] Firewall allows connection
- [ ] Messages properly serialized
- [ ] Both sides handle message type
</details>

<details>
<summary><b>Runtime Errors</b></summary>

```bash
# Enable full logging (with dynamic linking for faster rebuilds)
RUST_LOG=debug cargo run -p terminal --features bevy/dynamic_linking

# Backtrace
RUST_BACKTRACE=1 cargo run -p server --features bevy/dynamic_linking
```

Common causes:
- Resource not initialized
- Query mismatch (wrong component combinations)
- State not in expected value
- Network message not handled
</details>

## Code Quality Guidelines

### DO ✅

```rust
// Clear, descriptive names
fn handle_player_input(input: Res<PlayerInput>) { }

// Proper documentation
/// Converts world coordinates to DAC space
/// 
/// # Arguments
/// * `world_pos` - Position in world space (meters)
/// * `config` - Projector configuration with calibration
fn world_to_dac(world_pos: Vec3, config: &ProjectorConfiguration) -> Vec2 { }

// Error handling
fn load_config() -> Result<Config, std::io::Error> {
    let data = std::fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&data)?)
}

// Proper plugin organization
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_game)
            .add_systems(Update, (
                update_game_state,
                handle_player_actions,
            ).chain());
    }
}
```

### DON'T ❌

```rust
// Vague names
fn do_stuff(x: f32) { }

// Missing documentation on public APIs
pub fn important_function() { }

// Unwrap without thought
let config = std::fs::read_to_string("config.json").unwrap();

// Business logic in components (use systems instead)
#[derive(Component)]
struct Player {
    health: f32,
}

impl Player {
    fn take_damage(&mut self, enemy: &Enemy) { // ❌ Game logic in component
        self.health -= enemy.damage;
    }
}

// ✅ But validation/constructors ARE good:
impl Player {
    pub fn new(health: f32) -> Result<Self, String> {
        if health <= 0.0 || health > 100.0 {
            return Err("Health must be between 0 and 100".to_string());
        }
        Ok(Self { health })
    }
    
    pub fn health(&self) -> f32 { self.health } // Getters OK
    
    pub fn set_health(&mut self, value: f32) -> Result<(), String> {
        if value < 0.0 { return Err("Health cannot be negative".to_string()); }
        self.health = value;
        Ok(())
    }
}

// Mixing concerns
fn update_player_and_ui_and_network(/* too many params */) {
    // ❌ Do one thing per system
}
```

## Bevy 0.17 Specifics

### Key Changes from Earlier Versions

**Events → Messages**
```rust
// Old (0.16 and earlier)
#[derive(Event)]
struct MyEvent;

fn system(mut events: EventWriter<MyEvent>) {
    events.send(MyEvent);
}

// New (0.17)
#[derive(Message)]
struct MyEvent;

fn system(mut events: MessageWriter<MyEvent>) {
    events.send(MyEvent);
}
```

**SystemSets**
```rust
// Use SystemSet trait
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
struct MySystemSet;

app.configure_sets(Update, MySystemSet);
app.add_systems(Update, my_system.in_set(MySystemSet));
```

**States**
```rust
// States trait
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum MyState {
    #[default]
    Menu,
    InGame,
}

// SubStates trait
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(ParentState = ParentState::Active)]
enum MySubState {
    #[default]
    Default,
}
```

## Workspace Management

### Building
```bash
# DEVELOPMENT BUILDS (always use dynamic linking for speed)
cargo run -p server --features bevy/dynamic_linking
cargo run -p terminal --features bevy/dynamic_linking
cargo build --features bevy/dynamic_linking

# Specific crate with dynamic linking
cargo build -p server --features bevy/dynamic_linking
cargo build -p terminal --features bevy/dynamic_linking
cargo build -p common  # (common doesn't need the flag)
cargo build -p hunter  # (minigames don't need the flag)

# Release build (no dynamic linking)
cargo build --release

# Cross-compile server for Raspberry Pi 4
cross build -p server --target aarch64-unknown-linux-gnu --release  # 64-bit
cross build -p server --target armv7-unknown-linux-gnueabihf --release  # 32-bit
```

### Testing
```bash
# All tests
cargo test

# Specific crate tests
cargo test -p common
cargo test -p server

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Documentation
```bash
# Generate and open docs
cargo doc --open

# Include private items
cargo doc --document-private-items --open

# Specific crate
cargo doc -p common --open
```

## Security & Validation

### Server-Side Validation
**Always validate client input on server:**

```rust
// ✅ Good
fn handle_player_action(
    message: &PlayerActionMessage,
    player: &Player,
) -> Result<(), ValidationError> {
    // Validate before applying
    if message.position.distance(player.position) > MAX_MOVE_DISTANCE {
        return Err(ValidationError::InvalidMove);
    }
    // Apply action
    Ok(())
}

// ❌ Bad - Trust client blindly
fn handle_player_action(message: &PlayerActionMessage) {
    player.position = message.position; // No validation!
}
```

### Input Sanitization
```rust
// Validate ranges
fn update_config(value: f32) -> Result<(), String> {
    if !(0.0..=100.0).contains(&value) {
        return Err(format!("Value {} out of range", value));
    }
    Ok(())
}
```

## Performance Considerations

### Server (50 FPS)
- Systems run every 20ms (1/50 second)
- Use `FixedUpdate` for game logic
- Minimize allocations in hot paths
- Profile with `cargo flamegraph`

### Terminal (VSync)
- UI can be slower (immediate mode is cheap)
- Cache computed values in resources
- Lazy update: only redraw when needed

### Network
- Keep messages small
- Use binary serialization (bincode)
- Batch updates when possible
- Don't send unchanged data

## When You're Stuck

### Information Gathering
1. **Search codebase**: Use ripgrep for patterns
   ```bash
   rg "similar_feature" --type rust
   ```

2. **Check existing implementations**: Look at similar features
   - Hunter minigame is a good reference
   - Network plugin shows message patterns
   - Calibration shows coordinate transforms

3. **Read tests**: Tests show intended usage
   ```bash
   find . -name "*test*.rs" -type f
   ```

4. **Consult documentation**: When uncertain about syntax or version-specific APIs
   - Use `cargo doc --open` to view local crate documentation
   - Check Bevy 0.17 docs: https://docs.rs/bevy/0.17.1
   - Check dependency docs for correct syntax: https://docs.rs/
   - Verify version compatibility in `Cargo.toml`

### Ask for Clarification
Don't guess! When requirements are unclear:
- Ask about intended behavior
- Confirm which crates are affected
- Verify network protocol decisions
- Check if breaking changes are acceptable

### Start Small
1. Get it working in one place first
2. Add network sync if needed
3. Add UI if needed
4. Refine and optimize
5. Add tests

## Project Conventions Summary

| Aspect | Convention |
|--------|-----------|
| **Module Structure** | Features spanning crates use same name: `common/src/game.rs`, `server/src/plugins/game.rs`, `terminal/src/plugins/game.rs` |
| **Naming** | snake_case functions, PascalCase types |
| **Plugins** | One plugin per feature, in plugins/ directory |
| **Messages** | Query*, Update*, *Update, *Event patterns |
| **States** | ServerState, GameState (SubState), TerminalState |
| **Serialization** | All common types derive Serialize + Deserialize |
| **Resources** | Derive Resource, Default when possible |
| **Components** | Data with validation - constructors, getters/setters OK; no game logic |
| **Systems** | Small, focused, pure functions |
| **Errors** | Return Result, use ? operator, don't unwrap carelessly |
| **Documentation** | /// for public APIs, // for implementation notes |

## Quick Reference Links

- [Main Instructions](copilot-instructions.md) - Project overview
- [Server Instructions](instructions/server.instructions.md) - Server patterns
- [Terminal Instructions](instructions/terminal.instructions.md) - UI patterns  
- [Common Instructions](instructions/common.instructions.md) - Shared types
- [Network Instructions](instructions/network.instructions.md) - Protocol
- [Minigames Instructions](instructions/minigames.instructions.md) - Game dev
- [Calibration Instructions](instructions/calibration.instructions.md) - AR setup

## Final Checklist Before Submitting Changes

- [ ] Code compiles: `cargo build`
- [ ] Tests pass: `cargo test`
- [ ] Follows project conventions
- [ ] Documentation added for public items
- [ ] Network protocol updated if needed
- [ ] Both server and terminal updated if needed
- [ ] No unwrap() in production paths
- [ ] Serialization works for network types
- [ ] Consulted relevant instruction files
- [ ] Changes are minimal and focused
- [ ] Workspace structure in copilot-instructions.md updated if files/directories added or removed
- [ ] Instruction files updated if patterns, architecture, or conventions changed

---

**Remember**: This is a distributed AR system. Server is authoritative. Terminal is for UI. Common is for shared data. Always think about network sync.
