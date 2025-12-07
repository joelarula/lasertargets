# LaserTargets Monorepo

A Rust workspace monorepo for the LaserTargets augmented reality games and shows application.

## Project Structure

This monorepo uses Cargo workspaces to organize the LaserTargets application into multiple crates:

```
lasertargets/
├── Cargo.toml          # Workspace configuration
├── crates/
│   ├── client/         # Client application (Bevy-based UI)
│   └── server/         # Server application (Game engine & AR processing)
├── examples/           # Shared examples and demos
└── README.md          # This file
```

## Workspace Members

### Terminal (`terminal`)
- **Purpose**: Client-side application for LaserTargets
- **Framework**: Bevy 0.16.1 game engine
- **Features**: 
  - UI interface
  - Client-side rendering
  - User input handling

### Server (`server`)
- **Purpose**: Server application for LaserTargets augmented reality system
- **Framework**: Bevy 0.16.1 game engine
- **Features**: 
  - AR processing and calibration
  - Scene management
  - Camera controls
  - Toolbar system with Nerd Font icons

## Development

### Building the workspace
```bash
# Build all workspace members
cargo build

# Build specific crate
cargo build -p client
cargo build -p server
```

### Running applications
```bash
# Run client
cargo run -p client

# Run server with dynamic linking (faster development builds)
cargo run -p server --features bevy/dynamic_linking
```

### Workspace Dependencies

Common dependencies are defined in the workspace `Cargo.toml` and shared across members:
- `bevy = "0.16.1"` - Game engine framework
- `log = "0.4"` - Logging
- `pretty_env_logger = "0.5.0"` - Pretty logging output
- `tokio = "1.0"` - Async runtime
- `serde = "1.0"` - Serialization
- `serde_json = "1.0"` - JSON support

## Architecture

The monorepo follows a clean separation between client and server concerns:

- **Client**: Focuses on user interface and client-side rendering
- **Server**: Handles the core AR game engine, calibration, and processing

Both applications use the Bevy game engine but serve different roles in the overall LaserTargets system.