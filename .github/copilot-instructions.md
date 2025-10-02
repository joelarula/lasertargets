# GitHub Copilot Instructions for LaserTargets

## Project Overview
LaserTargets is a Rust application built with the Bevy game engine for augmented reality games and shows. The application is designed to be modular, extensible, and performant, leveraging Bevy's ECS architecture.

## Architecture
- **Main Framework**: Bevy 0.16.1 game engine
- **Language**: Rust (edition 2024)
- **Key Dependencies**: 
  - `log` for logging

## Code Style Guidelines
- Follow standard Rust conventions and formatting
- Use descriptive variable names and function names
- Add documentation comments for public APIs
- Prefer composition over inheritance where applicable
- Use Bevy's ECS (Entity Component System) patterns


## Project Structure
```
src/
├── main.rs              # Application entry point
├── plugins/             # Bevy plugins for different features
│   ├── calibration.rs   # Calibration functionality
│   ├── camera.rs        # Camera management
│   ├── config.rs        # Configuration management
│   ├── cursor.rs        # Cursor handling
│   ├── instructions.rs  # UI instructions
│   └── scene.rs         # Scene management
└── util/
    └── scale.rs         # Scaling calculations
```

## Key Patterns
- Each major feature should be implemented as a Bevy Plugin
- Use Bevy's Resource system for shared state
- Implement proper Component and System organization
- Use SystemSets for organizing system execution order

## Common Tasks
- When adding new features, create appropriate Components, Systems, and Resources
- Use Bevy's event system for communication between systems
- Follow the ECS pattern: avoid storing behavior in components
- Use proper error handling with Result types

## Performance Considerations
- Be mindful of system execution order and dependencies
- Use appropriate Bevy queries and filters
- Consider frame rate impact for real-time operations
- Optimize for both 2D and 3D display modes

## Testing
- Write unit tests for utility functions



