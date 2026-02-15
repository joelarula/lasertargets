# Common Library Instructions

**Applies to:** `common/**/*.rs`

## Context
You are working on the shared `common` library. This contains data structures, configurations, messages, states, and network protocols used by both server and terminal. Common should be considered as model and business logic only, with implementations of systems which operate solely with common crate data without side effects.

## Key Responsibilities
- Shared data models
- Network message definitions
- Configuration structures
- State definitions
- Serializable game types

## Important Guidelines

### Component Design
Components should be data-focused but can include:
- ✅ Constructors with validation (`new()` that returns `Result`)
- ✅ Getters and setters with validation
- ✅ Methods that maintain type invariants
- ✅ Conversion methods (`From`, `Into` implementations)
- ❌ Game logic (scoring, AI, collision detection)
- ❌ Side effects (network calls, file I/O)
- ❌ Anything requiring queries of other entities

```rust
// ✅ Good: Validation in constructor
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    position: Vec3,
    radius: f32,
}

impl Target {
    pub fn new(position: Vec3, radius: f32) -> Result<Self, String> {
        if radius <= 0.0 {
            return Err("Radius must be positive".to_string());
        }
        Ok(Self { position, radius })
    }
    
    pub fn radius(&self) -> f32 { self.radius }
}
```

### Serialization
All types in common must be serializable:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyType {
    // fields
}
```

For Bevy types, use appropriate derives:
```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MyComponent { }

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MyResource { }
```

### No Business Logic
- Common is for **data structures only**
- No game logic or system implementations
- No side effects or I/O
- Keep it pure and simple

### Network Messages
All network messages in `network.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // Query messages (client → server)
    QuerySomething,
    
    // Update messages (bidirectional)
    UpdateSomething(SomeData),
    
    // Response messages (server → client)
    SomethingUpdate(SomeData),
}
```

**Naming convention:**
- `Query*` - Request for data
- `Update*` - Send new data
- `*Update` - Broadcast/response with data

### Configuration Types
All configs in `config.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct MyConfiguration {
    pub field: f32,
    // ...
}

impl Default for MyConfiguration {
    fn default() -> Self {
        Self {
            field: 1.0,
        }
    }
}
```

### State Definitions
States in `state.rs`:
```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MyState {
    #[default]
    Initial,
    Active,
}
```

For SubStates:
```rust
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ParentState = ParentState::SomeVariant)]
pub enum MySubState {
    #[default]
    Default,
}
```

### Events/Messages
Use Bevy 0.17 Message derive:
```rust
#[derive(Message, Debug, Clone)]
pub struct MyEvent {
    pub data: String,
}
```

### Game Types
Game-related types in `game.rs`:
- `Game` - Game definition
- `GameSession` - Active game instance
- Game events and messages

### Actor Types
Actor-related types in `actor.rs`:
- `Actor` - Actor entity data
- `ActorMetaData` - Actor registration info
- Actor-related messages

### Path Types
Path definitions in `path.rs`:
- `UniversalPath` - Vector path representation
- Path-related utilities
- Uses `lyon_path` with serde

### Target Types
Target definitions in `target.rs`:
- Game-specific target types
- Target states and properties
- Must be serializable

## Module Structure

### Naming Convention
When a feature spans server/terminal/common, use **consistent naming**:
- `common/src/feature.rs` - Data structures
- `server/src/plugins/feature.rs` - Server-side systems
- `terminal/src/plugins/feature.rs` - Terminal-side systems

Examples:
- `game.rs` in all three crates (game session management)
- `scene.rs` in all three crates (scene configuration)
- `network.rs` in server/terminal (message handling)
- `calibration.rs` in all three crates (calibration data/systems)

```rust
// lib.rs - Module declarations only
pub mod actor;
pub mod config;
pub mod currency;
pub mod game;
pub mod network;
pub mod path;
pub mod scene;
pub mod state;
pub mod target;
pub mod toolbar;
```

## Dependencies
- `bevy` - Core types and derives
- `serde` - Serialization
- `bincode` - Binary serialization
- `lyon_*` - Vector path types

## Testing
Tests in `common/test/`:
- Unit tests only - no I/O or network tests
- Test serialization/deserialization
- Test default values
- Test state transitions
- Test data validation logic
- test instanc methiods (e.g., `new()` validation)

## Common Pitfalls
- ❌ Don't add system implementations here
- ❌ Don't add side effects or I/O
- ❌ Don't import server or terminal code
- ❌ Don't use non-serializable types in public API
- ✅ Keep types simple and focused
- ✅ Derive Serialize, Deserialize on all public types
- ✅ Provide sensible defaults
- ✅ Document complex data structures
