# Minigames Instructions

**Applies to:** `minigames/**/*.rs`

## Context
You are working on a LaserTargets minigame plugin. Minigames are modular game implementations that run on both server and terminal.

## Minigame Structure

Each minigame crate should have:
```
minigames/mygame/
├── Cargo.toml
├── readme.md              # Game description and rules
└── src/
    ├── lib.rs             # Module exports
    ├── model.rs           # Game-specific data models (serializable)
    ├── common.rs          # Shared plugin (implements GamePlugin)
    ├── server.rs          # Server-side game logic
    └── terminal.rs        # Terminal-side UI and rendering
```

## Module Organization

### lib.rs
```rust
pub mod model;
pub mod common;
pub mod server;
pub mod terminal;
```

### model.rs
Game-specific data structures:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyGameState {
    pub score: u32,
    pub level: u32,
    // ... game-specific fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyGameTarget {
    pub id: u32,
    pub position: Vec3,
    // ... target-specific fields
}
```

### common.rs
Shared plugin for both server and terminal:
```rust
use bevy::prelude::*;

pub struct MyGamePlugin;

impl Plugin for MyGamePlugin {
    fn build(&self, app: &mut App) {
        // Register game in registry
        app.add_systems(Startup, register_game);
        
        // Add shared systems
        app.add_systems(Update, shared_system);
    }
}

fn register_game(mut registry: ResMut<GameRegistry>) {
    registry.register_game(Game {
        id: 101, // Unique game ID
        name: "My Game".to_string(),
    });
}
```

### server.rs
Server-side game logic:
```rust
use bevy::prelude::*;

pub struct MyGameServerPlugin;

impl Plugin for MyGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_game_init,
                handle_game_start,
                game_update_logic,
                handle_player_input,
            ).run_if(in_game_session(101)), // Filter by game ID
        );
    }
}

fn game_update_logic(
    time: Res<Time>,
    mut query: Query<&mut MyGameState>,
) {
    // Game logic runs on server
    // Update state based on time and input
    // Broadcast changes to clients
}
```

### terminal.rs
Terminal-side UI and rendering:
```rust
use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub struct MyGameTerminalPlugin;

impl Plugin for MyGameTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                render_game_ui,
                render_game_entities,
            ).run_if(in_game_session(101)),
        );
    }
}

fn render_game_ui(
    mut contexts: EguiContexts,
    game_state: Res<MyGameState>,
) {
    egui::Window::new("My Game")
        .show(contexts.ctx_mut(), |ui| {
            ui.label(format!("Score: {}", game_state.score));
            ui.label(format!("Level: {}", game_state.level));
        });
}
```

## Game Lifecycle

### Registration
1. Game registers with `GameRegistry` in common plugin
2. Assigns unique `game_id` (u16)
3. Provides game name

### Initialization
1. Server receives `InitGameSession(session_id, game_id, initial_state)`
2. Game plugin creates game-specific resources
3. Spawns initial entities (targets, etc.)

### Gameplay
1. Server runs game logic in FixedUpdate
2. Handles player input from clients
3. Updates game state
4. Broadcasts state changes

### Completion
1. Game detects win/loss condition
2. Sends `FinishGameEvent`
3. Cleans up game-specific resources

## State Management

### Game-Specific Resources
```rust
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MyGameData {
    pub session_id: Uuid,
    pub state: MyGameState,
}
```

### Filtering Systems
Use run conditions to only run during your game:
```rust
fn in_game_session(game_id: u16) -> impl Fn(Res<GameSession>) -> bool {
    move |session: Res<GameSession>| {
        session.game_id == game_id
    }
}
```

## Actor Integration

### Input Handling
Handle actor input on server:
```rust
fn handle_actor_input(
    mut messages: EventReader<ActorInputMessage>,
    mut query: Query<&mut GameState>,
) {
    for msg in messages.read() {
        // Validate and process input
        // Update game state
    }
}
```

## Testing

### Unit Tests
Test game logic in isolation:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_calculation() {
        let state = MyGameState::new();
        // Test logic
    }
}
```

### Integration Tests
Test game lifecycle:
- Create test app with game plugin
- Simulate game session
- Verify state transitions

## Dependencies

Add to minigame `Cargo.toml`:
```toml
[dependencies]
bevy = { workspace = true }
common = { path = "../../common" }
serde = { workspace = true }
```

Add to `server/Cargo.toml` and `terminal/Cargo.toml`:
```toml
[dependencies]
mygame = { path = "../minigames/mygame" }
```

Register in both apps:
```rust
// In main.rs
use mygame::common::MyGamePlugin;
use mygame::server::MyGameServerPlugin; // or terminal

app.add_plugins(MyGamePlugin);
app.add_plugins(MyGameServerPlugin); // or MyGameTerminalPlugin
```

## Best Practices
- ✅ Keep game logic on server
- ✅ Make all game data serializable
- ✅ Use common plugin for shared setup
- ✅ Filter systems by game_id
- ✅ Clean up resources on game exit
- ✅ Provide clear win/loss conditions
- ❌ Don't trust client input
- ❌ Don't store state only on client
- ❌ Don't block the game loop
