# Snake Minigame Instructions

**Applies to:** `minigames/snake/**/*.rs`

## Context
You are working on the Snake minigame for LaserTargets. This is a multiplayer snake game where players control snakes that move around collecting "snaks" (food items) to grow and earn points. Snakes lose lives when colliding with boundaries or other snakes.

**Game ID**: TBD (assign unique ID in common/plugin)  
**Game Name**: TBD (assign in registration)

## Game Concept
- **Snakes**: Player-controlled entities that move along paths
- **Snaks**: Food items that snakes collect for points and growth
- **Lives**: Each snake has lives; game ends when lives reach 0
- **Scoring**: Snakes earn points by collecting snaks
- **Collision**: Snakes can collide with boundaries and each other

## Module Structure

Following the standard minigame pattern (see [.github/instructions/minigames.instructions.md](../../.github/instructions/minigames.instructions.md)):

```
minigames/snake/
├── src/
│   ├── lib.rs     # Module exports (model, plugin)
│   ├── model.rs   # Game-specific data models
│   └── plugin.rs  # Combined plugin (consider splitting to common/server/terminal)
```

**Note**: This minigame currently uses a single `plugin.rs` instead of the standard `common.rs`, `server.rs`, `terminal.rs` split. Consider refactoring to match the Hunter pattern for consistency.

## Data Models (model.rs)

### Snake
Represents a player-controlled snake:
- `uuid`: Unique identifier
- `actor`: Associated actor UUID
- `path`: Movement path (UniversalPath)
- `color`: Visual color of the snake
- `lives`: Remaining lives (u8)
- `score`: Current score (u32)

### Snak
Represents a collectible food item:
- `uuid`: Unique identifier
- `name`: Display name
- `actor`: Owner/controller actor UUID
- `path`: Movement path (can be static or moving)
- `reward`: Points awarded when collected (u32)

### SnakeGame
Game session state:
- `snakes`: List of active snakes
- `snaks`: List of active snaks (food items)

**Missing Fields** (consider adding):
- `game`: Game session UUID
- `controller`: Controlling actor UUID
- `boundaries`: Scene boundaries for collision detection

## Game State

**TODO**: Define game state enum similar to Hunter's `HunterGameState`:
```rust
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[source(ServerState = ServerState::InGame)]
pub enum SnakeGameState {
    #[default]
    Off,
    Playing,
    GameOver,
}
```

## Game Lifecycle

### 1. Registration
Define constants and register with `GameRegistry`:
```rust
pub const GAME_ID: u16 = 102; // Example - choose unique ID
pub const GAME_NAME: &str = "snakegame";
```

### 2. Initialization
When server receives `InitGameSession(session_id, GAME_ID, initial_state)`:
- Parse initial game state (snakes, snaks)
- Spawn snake entities with path components
- Spawn snak entities at positions
- Set up collision detection
- Initialize boundaries

### 3. Active Gameplay
**Server-side**:
- Update snake positions along paths
- Handle player input (direction changes)
- Detect snake-snak collisions (collection)
- Detect snake-snake collisions (life reduction)
- Detect snake-boundary collisions (life reduction)
- Spawn new snaks when collected
- Update scores
- Remove snakes with 0 lives
- Check win/lose conditions

**Terminal-side**:
- Render snakes with their colors
- Display snake scores and lives
- Render snaks (food items)
- Show visual effects for collisions and collections
- Display game boundaries

### 4. Completion
Game finishes when:
- Only one snake remains (winner)
- All snakes eliminated (no winner)
- Time limit reached (if implemented)
- Manual exit by controller

## Actor Integration

### Snake Control
Each snake is associated with an actor:
- Actor provides directional input (keyboard, controller, etc.)
- Input mapped to path updates or velocity changes
- Multiple actors can control multiple snakes

### Snak Generation
Snaks can be:
- Randomly spawned at intervals
- Placed at fixed positions
- Actor-controlled for special snaks (bonus items)

## Network Synchronization

### Server → Terminal
- Snake positions and states (lives, score, color)
- Snak positions and states
- Collision events
- Collection events
- Game over events

### Terminal → Server
- Player directional input
- Game control commands (pause, resume, exit)

## Implementation Patterns

### Snake Movement (Server)
System updates snake positions:
```rust
fn update_snake_movement(
    time: Res<Time>,
    input: Res<PlayerInput>,
    mut query: Query<(&mut Snake, &mut UniversalPath)>,
) {
    // Update path based on input
    // Move snake along path
    // Handle wrapping or boundary collision
}
```

### Collision Detection (Server)
System checks for collisions:
```rust
fn detect_collisions(
    mut snake_query: Query<(&mut Snake, &Transform)>,
    snak_query: Query<(Entity, &Snak, &Transform)>,
    mut commands: Commands,
) {
    // Check snake-snak intersections
    // Check snake-snake intersections
    // Check snake-boundary intersections
}
```

### Collection Handling (Server)
When snake collects a snak:
```rust
fn handle_collection(
    snake: &mut Snake,
    snak: &Snak,
    commands: &mut Commands,
) {
    snake.score += snak.reward;
    // Grow snake (extend path)
    // Remove snak entity
    // Spawn new snak
}
```

### Rendering (Terminal)
Visual representation:
```rust
fn render_snakes(
    mut commands: Commands,
    query: Query<(&Snake, &UniversalPath)>,
) {
    // Render snake body along path
    // Use snake.color for visual
    // Draw head differently
}

fn render_snaks(
    mut commands: Commands,
    query: Query<(&Snak, &Transform)>,
) {
    // Render snak at position
    // Use consistent visual style
}
```

## Best Practices

### Server-Side
- ✅ Use FixedUpdate for deterministic gameplay
- ✅ Validate all player input
- ✅ Handle collisions authoritatively
- ✅ Broadcast all state changes
- ❌ Don't allow client to set score directly
- ❌ Don't trust client collision reports

### Terminal-Side
- ✅ Render based on server state
- ✅ Provide smooth interpolation between updates
- ✅ Show clear visual feedback
- ❌ Don't calculate collisions locally
- ❌ Don't store authoritative game state

### Data Models
- ✅ Keep all types serializable
- ✅ Use Uuid for entity references
- ✅ Validate data in constructors
- ❌ Don't put game logic in model.rs

## Suggested Improvements

### Refactoring
Consider splitting `plugin.rs` into:
- `common.rs`: Registration, shared state (SnakeGameState)
- `server.rs`: Movement, collision, scoring logic
- `terminal.rs`: Rendering, UI, visual effects

### Features to Implement
- **Growth Mechanics**: Snake grows longer when collecting snaks
- **Power-ups**: Special snaks with bonus effects
- **Obstacles**: Static obstacles on the map
- **Speed Control**: Variable snake speed based on size or power-ups
- **Leaderboard**: Track high scores across sessions

### Network Optimization
- Consider delta compression for snake paths (only send changes)
- Batch position updates for multiple snakes
- Use unreliable channel for high-frequency position updates

## Testing Considerations
- Test single-player and multiplayer scenarios
- Test collision detection accuracy
- Test boundary wrapping vs collision
- Test snake growth mechanics
- Test snak spawning and despawning
- Test score calculation
- Test state transitions

## Performance Notes
- Snake path updates should be efficient
- Collision detection can be expensive with many snakes
- Consider spatial hashing for collision optimization
- Terminal rendering should handle snake growth smoothly

## Debugging
- Enable snake game logging: `RUST_LOG=snake=debug`
- Verify snake path updates
- Monitor collision detection events
- Check snak spawning logic
- Use terminal visualization to verify positions

## References
- [Minigames General Instructions](../../.github/instructions/minigames.instructions.md)
- [Server Instructions](../../.github/instructions/server.instructions.md)
- [Terminal Instructions](../../.github/instructions/terminal.instructions.md)
- [Network Protocol](../../.github/instructions/network.instructions.md)
- [Hunter Minigame](../hunter/INSTRUCTIONS.md) - Reference implementation
