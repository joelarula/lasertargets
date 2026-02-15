# Hunter Minigame Instructions

**Applies to:** `minigames/hunter/**/*.rs`

## Context
You are working on the Hunter minigame for LaserTargets. This is a target shooting game where hunters (players with laser pointers) shoot at moving targets to earn points. The game tracks individual hunter scores and target states.

**Game ID**: 101  
**Game Name**: "huntergame"

## Game Concept
- **Hunters**: Players with laser pointers who shoot at targets
- **Targets**: Moving entities with lives, rewards, and paths
- **Scoring**: Hunters earn points by hitting and destroying targets
- **Lives**: Each target has lives; destroyed when lives reach 0

## Module Structure

Following the standard minigame pattern (see [.github/instructions/minigames.instructions.md](../../.github/instructions/minigames.instructions.md)):

```
minigames/hunter/
├── src/
│   ├── lib.rs        # Module exports
│   ├── model.rs      # Game-specific data models
│   ├── common.rs     # Shared plugin (HunterGamePlugin)
│   ├── server.rs     # Server-side game logic
│   └── terminal.rs   # Terminal-side UI and rendering
```

## Data Models (model.rs)

### Hunter
Represents a player in the game:
- `uuid`: Unique identifier
- `actor`: Associated actor UUID
- `score`: Current score
- `hits`: List of target UUIDs that have been hit

### Target
Represents a shootable target:
- `name`: Display name
- `uuid`: Unique identifier
- `actor`: Owner/controller actor UUID
- `lives`: Remaining lives (u8)
- `reward`: Points awarded when destroyed
- `path`: Movement path (UniversalPath)

### HunterGame
Game session state:
- `game`: Game session UUID
- `controller`: Controlling actor UUID
- `hunters`: List of active hunters
- `targets`: List of active targets

## Game State (common.rs)

### HunterGameState
SubState of `ServerState::InGame`:
- `Off`: Game not active (default)
- `On`: Game running

**Important**: State transitions are handled automatically:
- Enters `On` when `GameSessionCreated` with `game_id == 101`
- Returns to `Off` when entering `ServerState::Menu`

## Game Lifecycle

### 1. Registration
In `common.rs`, the game registers itself with the `GameRegistry`:
```rust
pub const GAME_ID: u16 = 101;
pub const GAME_NAME: &str = "huntergame";
```

### 2. Initialization
When server receives `InitGameSession(session_id, 101, initial_state)`:
- Parse initial game state (hunters, targets)
- Spawn target entities with components
- Set up hunter tracking resources

### 3. Active Gameplay
**Server-side** (in `server.rs`):
- Detect laser pointer hits on targets
- Reduce target lives on hit
- Award points to hunter
- Remove destroyed targets
- Update target positions along paths
- Broadcast state updates to terminal

**Terminal-side** (in `terminal.rs`):
- Render targets at current positions
- Display hunter scores
- Show visual feedback for hits
- Render target paths

### 4. Completion
Game finishes when:
- All targets destroyed (win condition)
- Time limit reached (if implemented)
- Manual exit by controller

## Actor Integration

### Hunter Registration
Hunters must be registered actors with capabilities:
- `"laser_pointer"` capability for input
- Associated with a specific actor UUID

### Target Control
Targets can be:
- Static (no path movement)
- Dynamic (following UniversalPath)
- Actor-controlled (associated with an actor for special behavior)

## Network Synchronization

### Server → Terminal
- Target positions and states
- Hunter scores
- Hit events
- Target destruction events

### Terminal → Server
- Laser pointer input (position, button press)
- Game control commands (pause, resume, exit)

## Implementation Patterns

### Hit Detection (Server)
System filters by `HunterGameState::On`:
```rust
fn detect_hits(
    query: Query<(&Target, &Transform)>,
    input: Res<LaserInput>,
) {
    // Check if laser intersects with target hitbox
    // Reduce target lives
    // Award points to hunter
}
```

### Target Movement (Server)
System updates target positions along paths:
```rust
fn update_target_positions(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &UniversalPath)>,
) {
    // Move targets along their paths
    // Handle path completion/looping
}
```

### Score Display (Terminal)
UI system shows current scores:
```rust
fn render_scores_ui(
    mut contexts: EguiContexts,
    game_state: Res<HunterGame>,
) {
    // Display hunter scores in UI panel
}
```

### Target Rendering (Terminal)
Visual representation of targets:
```rust
fn render_targets(
    mut commands: Commands,
    query: Query<(&Target, &Transform)>,
) {
    // Spawn visual entities for targets
    // Use lyon for shape rendering
}
```

## Best Practices

### Server-Side
- ✅ Validate all hit detections (don't trust client)
- ✅ Use FixedUpdate for game logic
- ✅ Broadcast state changes after validation
- ✅ Clean up entities when game ends
- ❌ Don't allow client to set scores directly
- ❌ Don't trust client hit reports without verification

### Terminal-Side
- ✅ Render based on server state only
- ✅ Provide immediate visual feedback for player actions
- ✅ Cache target visuals for performance
- ❌ Don't calculate scores locally
- ❌ Don't store authoritative game state

### Data Models
- ✅ Keep all types serializable
- ✅ Use Uuid for all entity references
- ✅ Validate constructor parameters
- ❌ Don't put game logic in model.rs

## Testing Considerations
- Test hit detection with various target sizes and positions
- Test target path following
- Test score calculation
- Test multi-hunter scenarios
- Test target destruction and cleanup
- Test state transitions (Off ↔ On)

## Performance Notes
- Targets use UniversalPath for efficient movement
- Hit detection runs in FixedUpdate (50 FPS)
- Terminal rendering can be throttled independently
- Consider spatial partitioning for many targets

## Debugging
- Enable hunter game logging: `RUST_LOG=hunter=debug`
- Check `HunterGameState` transitions
- Verify target entity spawning/despawning
- Monitor network messages for state sync
- Use terminal visualization to verify target positions

## References
- [Minigames General Instructions](../../.github/instructions/minigames.instructions.md)
- [Server Instructions](../../.github/instructions/server.instructions.md)
- [Terminal Instructions](../../.github/instructions/terminal.instructions.md)
- [Network Protocol](../../.github/instructions/network.instructions.md)
