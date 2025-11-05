# GitHub Copilot Instructions for LaserTargets

## Project Overview
LaserTargets is a Rust application built with the Bevy game engine for augmented reality games and shows. The application is designed to be modular, extensible, and performant, leveraging Bevy's ECS architecture.

## Architecture
- **Main Framework**: Bevy 0.17.1 game engine
- **Language**: Rust (edition 2024)
- **Workspace Structure**: Cargo workspace with multiple crates
  - `crates/terminal` - Main terminal/UI application
  - `crates/common` - Server application
  - `crates/model` - Shared data models and configuration

- **Key Dependencies**: 
  - `bevy` 0.17.1
  - `bevy_egui` 0.32.0 for UI
  - `log` for logging
  - `pretty_env_logger`
  - `bevy_quinnet` for networking
  - `serde` and `serde_json` for serialization
  - `egui` for immediate mode GUI

## Code Style Guidelines
- Follow standard Rust conventions and formatting
- Use descriptive variable names and function names
- Add documentation comments for public APIs
- Prefer composition over inheritance where applicable
- Use Bevy's ECS (Entity Component System) patterns

## Project Structure
```
lasertargets/                      # Workspace root
├── Cargo.toml                     # Workspace manifest
├── target/                        # Shared build artifacts for all crates
├── .github/
│   └── copilot-instructions.md
└── crates/
    ├── terminal/                  # Main terminal/UI application
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── main.rs           # Application entry point
    │   │   ├── util.rs           # Utility functions
    │   │   └── plugins/          # Bevy plugins
    │   │       ├── mod.rs
    │   │       ├── calibration.rs
    │   │       ├── camera.rs
    │   │       ├── config.rs
    │   │       ├── cursor.rs
    │   │       ├── instructions.rs
    │   │       ├── projector.rs
    │   │       ├── scene.rs
    │   │       ├── settings.rs
    │   │       └── toolbar.rs
    │   └── assets/               # Assets for terminal app
    ├── server/                    # Server application
    │   ├── Cargo.toml
    │   └── src/
    │       └── main.rs
    └── common/                     # Shared models and config
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            └── config.rs         # SceneConfiguration and shared types
```



## Key Patterns
- Each major feature should be implemented as a Bevy Plugin
- Use Bevy's Resource system for shared state
- Implement proper Component and System organization
- Use SystemSets for organizing system execution order (Bevy 0.17: use *Systems suffix)

## Common Tasks
- When adding new features, create appropriate Components, Systems, and Resources
- Use Bevy's event/message system for communication between systems (Bevy 0.17 migration)
- Follow the ECS pattern: avoid storing behavior in components
- Use proper error handling with Result types

## Performance Considerations
- Be mindful of system execution order and dependencies
- Use appropriate Bevy queries and filters
- Consider frame rate impact for real-time operations
- Optimize for both 2D and 3D display modes

## Testing
- Write unit tests for utility functions

## Migration Notes (Bevy 0.17)
- Update all Bevy imports for types moved to new crates (camera, mesh, image, shader, light, sprite, ui_render, window, math, etc.).
- Use *Systems suffix for SystemSets.
- Refactor event/message usage (EventWriter → MessageWriter, etc.).
- Update UI code to use UiTransform/UiGlobalTransform for UI nodes.
- Refactor window setup to use split components (e.g., CursorOptions).
- If a file listed in the structure (e.g., util/scale.rs) does not exist, no migration is needed for it.



