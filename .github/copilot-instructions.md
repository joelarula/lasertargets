# GitHub Copilot Instructions for LaserTargets

> **For AI Agents**: See [AGENTS.md](AGENTS.md) for comprehensive workflow guidance, code examples, and troubleshooting. This file provides high-level project context and architecture.

## Project Overview
LaserTargets is a Rust-based augmented reality laser game platform built with the Bevy game engine. The system consists of a headless server that controls laser projector hardware (via Helios DAC) and a terminal client application with a UI for scene configuration, calibration, and game control. The platform is designed to be modular, extensible, and performant, supporting multiple minigames and networked communication between server and terminal.

## Architecture

### Core Framework
- **Engine**: Bevy 0.17.1 game engine with ECS architecture
- **Language**: Rust (edition 2024)
- **Workspace Structure**: Cargo workspace with general purpose crates in `server/`, `terminal/`, `common/`,  
  and game specific crates in `minigames/hunter`, `minigames/snake/`

### Application Architecture
1. **Server Application** (headless, 50 FPS): Runs the game logic, controls laser hardware, and manages game state
2. **Terminal Application** (UI client): Provides UI for configuration, calibration, scene setup, and game control
3. **Network Protocol**: Client-server communication via `bevy_quinnet` (QUIC-based)
4. **Minigames**: Plugin-based game modules (Hunter, Snake) that run on both server and terminal, separated to server and terminal logic

### Architecture Diagram
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

## Workspace Structure

> **Important:** Keep this section up to date when adding/removing crates, directories, or restructuring the project.

```
lasertargets/                         # Workspace root
├── Cargo.toml                        # Workspace manifest with shared dependencies
├── README.md                         # Build instructions
├── WORKSPACE_README.md               # Detailed workspace documentation
├── server/                           # Headless server application
│   ├── Cargo.toml
│   ├── build.rs                      # Copies HeliosLaserDAC.dll to target/debug
│   ├── src/
│   │   ├── main.rs                   # Server entry point with ScheduleRunnerPlugin
│   │   ├── lib.rs                    # create_server_app() function
│   │   ├── dac/                      # DAC (Digital-to-Analog) hardware interfaces
│   │   │   ├── mod.rs
│   │   │   └── helios.rs             # Helios laser DAC integration
│   │   └── plugins/                  # Server-side plugins
│   │       ├── mod.rs
│   │       ├── actor.rs              # Actor management
│   │       ├── calibration.rs        # Calibration logic
│   │       ├── camera.rs             # Camera configuration
│   │       ├── game.rs               # Game session management
│   │       ├── network.rs            # QuinnetServerPlugin networking
│   │       ├── path.rs               # Path/trajectory management
│   │       ├── projector.rs          # Projector output control
│   │       └── scene.rs              # Scene state management
│   ├── libs/                         # External libraries (HeliosLaserDAC.dll)
│   └── test/                         # Integration tests
│       ├── integrationtest.rs
│       ├── sync_test.rs
│       └── util.rs
├── terminal/                         # Terminal client application with UI
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                   # Terminal entry point
│   │   ├── util.rs                   # Utility functions (logging, etc.)
│   │   └── plugins/                  # Terminal-side plugins
│   │       ├── mod.rs
│   │       ├── basictarget.rs        # Basic target entities
│   │       ├── calibration.rs        # Calibration UI and logic
│   │       ├── camera.rs             # Camera view control
│   │       ├── config.rs             # Configuration management
│   │       ├── game.rs               # Game UI integration
│   │       ├── instructions.rs       # Help/instructions UI
│   │       ├── keyboard.rs           # Keyboard input handling
│   │       ├── mouse.rs              # Mouse input handling
│   │       ├── network.rs            # Client networking
│   │       ├── path.rs               # Path visualization
│   │       ├── projector.rs          # Projector preview/control
│   │       ├── scene.rs              # Scene editor
│   │       ├── settings.rs           # Settings UI
│   │       ├── target.rs             # Target management
│   │       ├── toolbar.rs            # Main toolbar UI
│   │       └── trigger.rs            # Trigger system
│   ├── assets/                       # Assets (fonts, etc.)
│   │   └── fonts/
│   └── bin/                          # Additional binaries
│       ├── combo.rs
│       └── test/
├── common/                           # Shared library for models, networking, and config
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                    # Module declarations
│   │   ├── actor.rs                  # Actor definitions and metadata
│   │   ├── config.rs                 # Scene, camera, projector configuration
│   │   ├── currency.rs               # In-game currency system
│   │   ├── game.rs                   # Game, GameSession, events
│   │   ├── network.rs                # NetworkMessage enum for client-server protocol
│   │   ├── path.rs                   # UniversalPath and trajectory types
│   │   ├── scene.rs                  # SceneSetup, SceneData, SceneEntity
│   │   ├── state.rs                  # ServerState, GameState, TerminalState
│   │   ├── target.rs                 # Target definitions (HunterTarget, etc.)
│   │   └── toolbar.rs                # Toolbar state/configuration
│   └── test/
│       └── scenetest.rs
├── minigames/                        # Modular game implementations
│   ├── hunter/                       # Hunter minigame
│   │   ├── Cargo.toml
│   │   ├── readme.md
│   │   └── src/
│   │       ├── lib.rs                # Module exports
│   │       ├── model.rs              # Hunter-specific data models
│   │       ├── common.rs             # HunterGamePlugin (shared logic)
│   │       ├── server.rs             # Server-side Hunter systems
│   │       └── terminal.rs           # Terminal-side Hunter UI
│   └── snake/                        # Snake minigame
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── model.rs              # Snake-specific data models
│           └── plugin.rs             # SnakeGamePlugin
├── swarm/                            # P2P swarm networking (experimental)
│   ├── Cargo.toml
│   ├── readme.md                     # SWARM architecture documentation
│   └── implementation_plan.md
└── target/                           # Compiled artifacts (shared across workspace)
```

## Key Dependencies

> **Important:** Keep this section up to date when adding, removing, or upgrading dependencies during development or refactoring.

### Workspace-Level Dependencies (defined in root Cargo.toml)
- **bevy** 0.17.1 - Core game engine
- **bevy_egui** 0.38.0 - Immediate mode GUI integration
- **bevy_quinnet** 0.19 - QUIC-based networking for Bevy
- **bevy_prototype_lyon** 0.15.0 - 2D shape rendering
- **egui** 0.33 - Immediate mode GUI
- **log** 0.4 - Logging facade
- **pretty_env_logger** 0.5.0 - Pretty log output
- **tokio** 1.0 - Async runtime (full features)
- **serde** 1.0 - Serialization framework
- **bincode** 1.3 - Binary serialization
- **lyon_path** 1.0.16 - Vector path manipulation

### Crate-Specific Dependencies
- **terminal**: `bevy_camera` 0.17.2, imports hunter and snake minigames
- **server**: `libloading` 0.8 (dynamic library loading), `lyon_tessellation`
- **common**: `lyon_geom`, `lyon_tessellation` (with serde features)

## Code Style and Conventions

### Rust Conventions
- Follow standard Rust formatting (use `cargo fmt`)
- Use descriptive, snake_case names for functions and variables
- Use PascalCase for types, traits, and enum variants
- Add documentation comments (`///`) for public APIs
- Prefer explicit types over type inference in public APIs
- Use Result types for fallible operations

### Bevy ECS Patterns
- **Components**: Pure data structures without behavior (derive `Component`). Can have methods based on their inner state (constructors, getters/setters, validation), but no game logic
- **Systems**: Functions that operate on Components via queries
- **Resources**: Global state shared across systems (derive `Resource`)
- **Plugins**: Group related systems, resources, and startup logic
- **Events/Messages**: Use Bevy 0.17's `Message` system (not `Event`)
- **SystemSets**: Use `SystemSet` trait for organizing execution order
- **States**: Application flow control using `States` and `SubStates` traits

### Plugin Organization
- Each major feature should be its own Plugin
- Use the plugin pattern: `pub struct MyPlugin;` + `impl Plugin for MyPlugin`
- Register systems in `.add_systems()` with appropriate schedule (Startup, Update, FixedUpdate)
- Use SystemSets to enforce execution order when needed
- Keep plugin files focused: one plugin per file in `plugins/` directory

### State Management
- Use Bevy States and SubStates for application flow control
- **Common States** (defined in `common/src/state.rs`):
  - `ServerState`: `Menu`, `InGame` - Server application flow
  - `GameState` (SubState of ServerState::InGame): `InGame`, `Paused`, `Finished` - Active game session flow
  - `TerminalState`: `Connecting`, `Connected` - Client connection status
- **Minigame States**: Each minigame can define its own states for game-specific flow
- Use `OnEnter`, `OnExit`, `OnTransition` run conditions for state-specific systems
- States must derive `States` or `SubStates` trait (Bevy 0.17)

## Network Protocol

### Communication Pattern
- Terminal connects to server via `bevy_quinnet` (QUIC protocol)
- Messages defined in `common/src/network.rs` as `NetworkMessage` enum
- Binary serialization via `bincode`

### Internal Message Pattern
Both server and terminal use an internal message-based architecture that abstracts network communication:

**Message Flow:**
1. **Source plugin** raises a Bevy message locally
2. **Network plugin** listens for that message and converts it to `NetworkMessage`
3. **Network layer** sends `NetworkMessage` over QUIC
4. **Remote network plugin** receives `NetworkMessage` and raises local Bevy message
5. **Target plugin** handles the message as if raised locally

**Result:** Plugins can think of themselves as directly raising messages on the remote side. The network layer is transparent.

**Example:**
```rust
// Terminal: Plugin raises message
commands.send_message(UpdateSceneConfig(config));

// Terminal: Network plugin sends over wire
// (NetworkPlugin listens and converts to NetworkMessage)

// Server: Network plugin raises message
commands.send_message(UpdateSceneConfig(config));

// Server: Plugin handles as if local
fn handle_scene_update(mut messages: MessageReader<UpdateSceneConfig>) {
    // Handle the update
}
```

This pattern maintains clean separation between business logic (plugins) and communication (network plugin).

### Key Message Types
- **Connection**: `Ping`, `Pong`
- **State Queries**: `QueryServerState`, `QueryGameState`, `QuerySceneConfig`, etc.
- **State Updates**: `ServerStateUpdate`, `GameStateUpdate`, `SceneConfigUpdate`, etc.
- **Game Session**: `InitGameSession`, `StartGameSession`, `PauseGameSession`, `ExitGameSession`, etc.
- **Actor Management**: `RegisterActor`, `UnregisterActor`, `QueryActor`, `ActorResponse`
- **Input Events**: `MouseButtonInput`, `KeyboardInput`, `UpdateMousePosition`

### Network System Design
- Server uses `NetworkingPlugin` with `QuinnetServerPlugin`
- **Server-side network logic** decides broadcast behavior:
  - **Unicast**: Query responses sent only to requesting client (e.g., `QueryServerState` → single client)
  - **Broadcast**: State updates sent to all connected clients (e.g., `SceneConfigUpdate` → all clients)
  - **Selective**: Some messages sent to specific clients based on game state or actor ownership
- Terminal uses `NetworkPlugin` with client connection logic
- Both sides handle `NetworkMessage` enum in dedicated systems
- Use Resources to cache synced state (configs, scene setup, etc.)

## Scene and Calibration System

### Scene Configuration
- **SceneConfiguration**: Defines the physical AR scene representation (dimensions, distance, transform)
- **CameraConfiguration**: Camera parameters (FOV, position, etc.)
- **ProjectorConfiguration**: Projector settings (resolution, calibration points)
- **SceneSetup**: Combined calculated resource that derives from all configs

### Calibration Flow
1. Define scene bounds in the terminal UI
2. Calibrate camera position and orientation
3. Map projector output to scene coordinates
4. Store calibration data in configuration resources
5. Synchronize configs between server and terminal

## Game Session Management

### Game Registry
- `GameRegistryPlugin` maintains a registry of available games
- Each minigame registers itself with a unique `game_id`
- Games can be initialized with `InitGameSessionEvent`

### Game Session Lifecycle
1. **Create**: `GameSessionCreated` message with unique `session_id` (UUID)
2. **Initialize**: `InitGameSession` sets up game-specific state
3. **Start**: `StartGameEvent` transitions to active gameplay
4. **Pause/Resume**: `PauseGameSession`, `ResumeGameSession`
5. **Finish**: `FinishGameEvent` ends the session
6. **Exit**: `ExitGameEvent` cleans up resources

### Minigame Development
- Create a new crate in `minigames/` directory
- Implement three modules:
  - `model.rs`: Game-specific data structures (serializable)
  - `common.rs`: Shared plugin with `GamePlugin` trait implementation
  - `server.rs`: Server-side game logic systems
  - `terminal.rs`: Terminal-side UI and rendering
- Add dependency to both `server/Cargo.toml` and `terminal/Cargo.toml`
- Register plugin in both applications

## Actor System

### Actor Concept
- Actors represent external entities (players, devices, etc.) connected to the game
- Each actor has a unique UUID and metadata
- Actors can register capabilities (e.g., "laser_pointer", "mobile_controller")
- Server manages actor lifecycle and state

### Actor Registration
1. Actor sends `RegisterActor(uuid, name, capabilities)` message
2. Server creates `ActorMetaData` and stores in resource
3. Server responds with `ActorResponse` confirmation
4. Actor can now participate in game sessions

## Hardware Integration

### Overview
- Helios Laser DAC integration for laser projector control (see [server.instructions.md](instructions/server.instructions.md))
- `ProjectorPlugin` manages projector output on both server and terminal
- Server sends actual laser commands to DAC hardware
- Terminal shows preview/visualization of projected content

## Path and Trajectory System

### UniversalPath
- Defined in `common/src/path.rs`
- Represents 2D/3D paths for laser drawing and actor movement
- Uses `lyon_path` for vector path operations
- Serializable for network transmission
- Used for bidirectional communication of projected path information between server and terminal

### Path Networking
- `PathNetworkPlugin` on server handles path synchronization
- Terminal can query and visualize paths
- Paths can be associated with game components (e.g., targets, actors) for dynamic behavior
## UI System (Terminal)

### Egui Integration
- Uses `bevy_egui` for immediate mode GUI
- Primary use: Settings plugin GUI interface
- Each panel implemented as a system in corresponding plugin


### Key UI Components
- **Toolbar**: Toolbar menu system (`toolbar.rs`)
- **Settings**: Configuration editor (`settings.rs`)
- **Calibration**: Interactive calibration UI (`calibration.rs`)
- **Instructions**: Help and documentation (`instructions.rs`)
- **Scene Setup**: Scene setup UI (`scene.rs`)

## Performance Optimization

### Build Profiles
- **dev**: Opt-level 1 for faster iteration, opt-level 3 for dependencies
- **dev-fast**: Opt-level 0 for fastest compilation (use for quick tests)
- **release**: Opt-level 3 with LTO enabled

### Development Workflow
- **Always use `--features bevy/dynamic_linking` during development** for significantly faster compile times
- This flag enables dynamic linking of Bevy, reducing incremental build times from minutes to seconds
- Examples:
  ```bash
  cargo run -p server --features bevy/dynamic_linking
  cargo run -p terminal --features bevy/dynamic_linking
  cargo build --features bevy/dynamic_linking
  ```
- Only build without this flag for release builds: `cargo build --release`

### Cross-Compilation for Raspberry Pi 4
The server can be cross-compiled for Raspberry Pi 4 Linux using `cross`:

```bash
# Install cross
cargo install cross

# Build for Raspberry Pi 4 (64-bit)
cross build -p server --target aarch64-unknown-linux-gnu --release

# Build for Raspberry Pi 4 (32-bit)
cross build -p server --target armv7-unknown-linux-gnueabihf --release
```

**Note:** Cross-compilation is only needed for the server (headless). Terminal requires a display and is typically run on desktop platforms.

### Runtime Performance
- Server runs at fixed 50 FPS (`FIXED_TIMESTEP = 1.0/50.0`)
- Use FixedUpdate schedule for deterministic game logic
- Terminal uses `AutoNoVsync` present mode for responsive UI
- Be mindful of query complexity in hot systems

## Testing

### Test Structure
- Unit tests in `#[cfg(test)]` modules within source files
- Integration tests in `test/` directories
- Example: `common/test/scenetest.rs`, `server/test/integrationtest.rs`

### Test Conventions
- Write tests for utility functions and data transformations
- Test serialization/deserialization of network messages
- Test configuration validation logic
- Use `cargo test` to run all tests in workspace

## Common Development Tasks

### Adding a New Feature
1. Decide if it belongs in server, terminal, or common
2. Create a new plugin file in the appropriate `plugins/` directory
3. Define Components, Resources, and Systems
4. Implement Plugin trait
5. Register plugin in `main.rs` (server or terminal)
6. Add network messages to `common/src/network.rs` if needed
7. Test locally with both server and terminal

### Adding a Network Message
1. Add new variant to `NetworkMessage` enum in `common/src/network.rs`
2. Implement message handling system in server's `plugins/network.rs`
3. Implement message handling system in terminal's `plugins/network.rs`
4. Ensure proper serialization with `serde`

### Creating a New Minigame
1. Create new crate in `minigames/` directory: `cargo new --lib minigames/mygame`
2. Add to workspace members in root `Cargo.toml`
3. Create `model.rs` with game-specific types
4. Create `common.rs` with `MyGamePlugin`
5. Create `server.rs` with server-side systems
6. Create `terminal.rs` with terminal-side UI
7. Add dependency to `server/Cargo.toml` and `terminal/Cargo.toml`
8. Register plugin in both `server/src/main.rs` and `terminal/src/main.rs`
9. Register game in `GameRegistryPlugin`

## Migration Notes (Bevy 0.17)

### Important Changes
- **Events → Messages**: Use `#[derive(Message)]` instead of `Event`
- **SystemSets**: Use `SystemSet` trait with `#[derive(SystemSet)]`
- **State API**: Uses `States` and `SubStates` traits
- **UI Components**: Separate transform types for UI (`UiTransform`)
- **Window Setup**: Split window configuration into multiple components

### Import Changes
Many Bevy types moved to new crates in 0.17:
- Camera types: `bevy::render::camera`
- Mesh types: `bevy::mesh`
- Image types: `bevy::image`
- Window types: `bevy::window`
- Math types: `bevy::math`

### Migration Strategy
- Update imports for moved types
- Replace `Event` with `Message` and `EventWriter` with `MessageWriter`
- Update SystemSet definitions to use `SystemSet` trait
- Refactor window setup code to use split components
- Test thoroughly after migration

## Troubleshooting
Path-Specific Instructions

Detailed instructions for specific areas of the codebase are in `.github/instructions/`:

| Working in... | Read... | Key Focus |
|---------------|---------|-----------|
| `server/` | [server.instructions.md](instructions/server.instructions.md) | Headless app, 50 FPS fixed, hardware control, authoritative state |
| `terminal/` | [terminal.instructions.md](instructions/terminal.instructions.md) | UI with egui, client networking, visualization only |
| `common/` | [common.instructions.md](instructions/common.instructions.md) | Data structures only, no logic, must serialize |
| `minigames/` | [minigames.instructions.md](instructions/minigames.instructions.md) | Plugin structure, model/common/server/terminal split |
| Network code | [network.instructions.md](instructions/network.instructions.md) | Message patterns, QUIC protocol, naming conventions |
| Calibration | [calibration.instructions.md](instructions/calibration.instructions.md) | Coordinate systems, transformations, UI/hardware sync |

## Resources and References

### Project Documentation
- [AGENTS.md](AGENTS.md): **Comprehensive guide for AI agents** - workflows, examples, troubleshooting
- `README.md`: Build and run instructions
- `WORKSPACE_README.md`: Detailed workspace structure
- `swarm/readme.md`: SWARM networking architecture
- Individual crate `Cargo.toml` files for dependency info

### Bevy Documentation
- [Bevy Book](https://bevyengine.org/learn/book/introduction/)
- [Bevy API Docs](https://docs.rs/bevy/)
- [Bevy Cheat Book](https://bevy-cheatbook.github.io/)

### External Libraries
- [bevy_egui](https://docs.rs/bevy_egui/)
- [bevy_quinnet](https://docs.rs/bevy_quinnet/)
- [lyon](https://docs.rs/lyon/)
- [egui](https://docs.rs/egui/)

## Project Conventions Summary

| Aspect | Convention |
|--------|-----------|
| **Module Structure** | Features spanning server/terminal/common use same name: `common/src/game.rs`, `server/src/plugins/game.rs`, `terminal/src/plugins/game.rs` |
| **Naming** | snake_case functions, PascalCase types |
| **Plugins** | One plugin per feature, in plugins/ directory |
| **Messages** | Query*, Update*, *Update, *Event patterns |
| **States** | ServerState, GameState (SubState), TerminalState |
| **Serialization** | All common types derive Serialize + Deserialize |
| **Resources** | Derive Resource, Default when possible |
| **Components** | Data with validation - constructors, getters/setters OK; no game logic |
| **Systems** | Small, focused, pure functions |
| **Errors** | Return Result, use ? operator, don't unwrap carelessly |
| **Documentation** | /// for public APIs, // for implementation notes |rs/bevy/)
- [Bevy Cheat Book](https://bevy-cheatbook.github.io/)

### Project-Specific Docs
- `README.md`: Build and run instructions
- `WORKSPACE_README.md`: Detailed workspace structure
- `swarm/readme.md`: SWARM networking architecture
- Individual crate `Cargo.toml` files for dependency info

### External Libraries
- [bevy_egui](https://docs.rs/bevy_egui/)
- [bevy_quinnet](https://docs.rs/bevy_quinnet/)
- [lyon](https://docs.rs/lyon/)
- [egui](https://docs.rs/egui/)



