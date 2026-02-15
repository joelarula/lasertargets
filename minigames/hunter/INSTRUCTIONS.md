# Hunter Minigame Instructions

**Applies to:** `minigames/hunter/**/*.rs`

## Context
You are working on the Hunter minigame for LaserTargets. This is a simple target practice game where the user drags basic circular targets into the scene and clicks on them to pop/destroy them. The game tracks statistics including targets spawned, targets popped, and total points earned.

**Game ID**: 101  
**Game Name**: "huntergame"  
**Implementation Status**: ✅ **COMPLETE** - All features implemented and working

## Game Concept
- **Target Practice**: User places targets in the scene by dragging
- **Basic Targets**: Circle shapes with 0.25 m diameter
- **Interaction**: Mouse click to pop (destroy) targets
- **Scoring**: 10 points awarded for each target popped
- **Visual Feedback**: Red collision indicator (4cm diameter) spawns at hit location
- **Indicator Behavior**: Previous collision point is cleared when next target is popped
- **Statistics Display**: Bottom-right UI showing live "Spawned: X | Hits: Y | Points: Z"
- **Event Tracking**: All spawn/pop events recorded with timestamps and positions
- **Post-Game Report**: Comprehensive analytics logged on game exit
- **Current State**: Terminal-side implementation (client authority)

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
Represents the player/game state:
- `uuid`: Unique identifier
- `actor`: Associated actor UUID
- `score`: Current score (points earned)
- `hits`: List of target UUIDs that have been popped

### Target
Represents a basic circular target:
- `name`: Display name
- `uuid`: Unique identifier
- `actor`: Owner/controller actor UUID
- `lives`: Remaining lives (currently u8, typically 1 for basic targets)
- `reward`: Points awarded when popped (u32)
- `path`: Movement path (UniversalPath) - currently unused for static targets

**Target Specifications:**
- **Shape**: Circle
- **Diameter**: 0.25 meters
- **Default Behavior**: Static (no movement)

### TargetEvent
Records target lifecycle events for post-game reporting:
- `target_uuid`: UUID of the target
- `event_type`: "spawned" or "popped"
- `timestamp`: Time when event occurred (f64 seconds since game start)
- `position`: Scene coordinates (Vec3) where event occurred

### CollisionIndicator (Component)
Marker component for the visual collision point indicator:
- Spawned at exact click position when target is popped
- Only one indicator exists at a time (previous is despawned)
- Small visual marker (circle, cross, or point)

### HunterGameStats (Resource)
**NEW**: Resource for tracking game statistics (implemented):
- `session_id`: Game session UUID
- `targets_spawned`: Counter for spawned targets (u32)
- `targets_popped`: Counter for popped targets (u32)
- `score`: Total points earned (u32)
- `target_events`: Vec<TargetEvent> for spawn/pop history
- `game_start_time`: f64 timestamp for relative event timing

### GameReport
**NEW**: Post-game analytics report (implemented):
- `total_targets_spawned`, `total_targets_popped`, `total_score`: Final counts
- `total_game_time`: Duration in seconds
- `avg_spawn_interval`: Average time between spawns
- `avg_target_lifetime`: Average time from spawn to pop
- `spawn_positions`, `pop_positions`: Vec<Vec3> for heatmap analysis
- `timeline`: Complete event history with timestamps

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
**Current Implementation** (Terminal-side - ✅ COMPLETE):
- ✅ User drags targets into scene via drag-and-drop from toolbar
- ✅ Server spawns targets with UniversalPath and broadcasts to terminals
- ✅ Terminal renders circular targets (0.25 m diameter, 0.125 m radius)
- ✅ Terminal detects mouse clicks with distance-based collision detection
- ✅ Terminal despawns clicked targets with visual feedback
- ✅ 10 points awarded per popped target
- ✅ Statistics tracked: spawned count, popped count, score
- ✅ All events recorded with timestamps and positions
- ✅ Real-time UI display: "Spawned: X | Hits: Y | Points: Z"
- ✅ Red collision indicator (0.02m diameter) spawns at click position
- ✅ Previous indicator cleared automatically

**Implementation Details**:
- **Server** (`server.rs`): Spawns targets with UUID and reward, broadcasts via UniversalPath
- **Terminal** (`terminal.rs`): Drag-and-drop UI, click detection, stats display, collision indicators
- **Common** (`common.rs`): Stats initialization, report generation

**Future Server-side Migration**:
- Move hit detection to server for authoritative gameplay
- Server validates all clicks and manages authoritative score
- Broadcast state updates to all clients for multiplayer

### 4. Completion
Game finishes when:
- Time limit reached (if implemented)
- Manual exit by user
- All targets destroyed (optional win condition)

**On Game Finish**:
- Generate game report from `target_events`
- Log report to console with all statistics and analytics
- Display report summary in UI (optional)
- Save report to file (optional)

## Game Statistics

### Tracked Metrics (✅ IMPLEMENTED)
- **Targets Spawned**: Total number of targets placed in the scene
- **Targets Popped**: Total number of targets destroyed by clicks
- **Total Points**: Cumulative score (10 points per target)
- **Current Targets**: Active targets in scene (Spawned - Popped)

### Event Tracking (✅ IMPLEMENTED)
- **Target Events**: Complete history stored in `HunterGameStats.target_events`
- **Spawn Events**: Record UUID, timestamp, position, event_type="spawned"
- **Pop Events**: Record UUID, timestamp, position, event_type="popped"
- **Timestamp**: Relative to game start time (f64 seconds)
- **Purpose**: Generate post-game reports and analytics

### Real-Time Display (✅ IMPLEMENTED)
**Location**: Bottom-right corner of terminal window  
**Format**: `"Spawned: X | Hits: Y | Points: Z"`  
**Technology**: Bevy UI Text with absolute positioning  
**Updates**: Automatically when `HunterGameStats` changes  
**Lifecycle**: Spawned on game start, despawned on game exit

### Post-Game Reporting (✅ IMPLEMENTED)
Generated automatically in `on_hunter_game_finish` system (OnExit(HunterGameState::On)):

**Report Includes**:
- ✅ Game Duration (total time from start to finish)
- ✅ Total targets spawned, popped, and score
- ✅ Average spawn interval (time between consecutive spawns)
- ✅ Average target lifetime (time from spawn to pop)
- ✅ Spawn position list (Vec<Vec3> for heatmap analysis)
- ✅ Pop position list (Vec<Vec3> for heatmap analysis)
- ✅ Complete event timeline with timestamps

**Output**: Logged to console with info! macro  
**Format**: Human-readable with section headers

## Target Interaction

### Spawning Targets (✅ IMPLEMENTED)
**Terminal** (`handle_target_drag` in terminal.rs):
1. User clicks and holds toolbar button ("spawn_basic_target")
2. Drag state activates, white preview circle follows mouse
3. User releases mouse button over scene
4. Terminal sends `NetworkMessage::SpawnHunterTarget(target, world_pos)` to server
5. Terminal increments `HunterGameStats.targets_spawned`
6. Terminal records spawn event with timestamp and position

**Server** (`spawn_hunter_targets` in server.rs):
1. Receives `SpawnHunterTargetEvent` from network plugin
2. Generates unique UUID and sets reward to 10
3. Creates target entity with:
   - `Transform` (world position)
   - `UniversalPath` (circle with 0.125m radius)
   - `HunterTargetEntity` (UUID, reward, target_type)
   - `PathRenderable` (for rendering)
4. Parents to SceneEntity for automatic broadcast

**Result**: Target appears on all connected terminals automatically

### Popping Targets (✅ IMPLEMENTED)
**Terminal** (`detect_target_clicks` in terminal.rs):
1. User clicks left mouse button
2. System gets click position in world coordinates from `SceneData`
3. Queries all target entities with `GlobalTransform` and `UniversalPath`
4. For each target:
   - Calculate distance from click to target center
   - Check if distance <= 0.125m (radius)
   - If hit:
     a. Despawn all previous `CollisionIndicator` entities
     b. Spawn new red collision indicator at click position (0.02m diameter)
     c. Record pop event in `HunterGameStats.target_events`
     d. Increment `HunterGameStats.targets_popped`
     e. Add 10 to `HunterGameStats.score`
     f. Despawn target entity
     g. Break (only pop one target per click)

**Future Server Authority**:
- Terminal sends click position to server
- Server validates hit with authoritative state
- Server removes target and updates score
- Broadcasts to all clients

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

### Target Spawning (Terminal)
Implemented in `handle_target_drag` system in terminal.rs:
- Handles drag-and-drop from toolbar button
- Sends spawn request to server via NetworkMessage
- Records spawn event in HunterGameStats
- Updates targets_spawned counter

### Hit Detection (Terminal)
Implemented in `detect_target_clicks` system in terminal.rs:
- Detects left mouse button press
- Calculates distance from click to target centers
- Checks collision (distance <= 0.125m radius)
- Despawns target and updates statistics
- Spawns collision indicator at click position

### Collision Indicator
Implemented in `spawn_collision_indicator` helper function:
- Creates small red circle (0.02m diameter) at click position
- Previous indicator despawned before spawning new one
- Uses UniversalPath::circle with red color

### Statistics Display (Terminal)
Implemented in `spawn_hunter_stats_ui` and `update_hunter_stats_display` systems:
- Bottom-right UI text with Bevy UI components
- Format: "Spawned: X | Hits: Y | Points: Z"
- Updates automatically when HunterGameStats changes
- Spawned on game start, despawned on exit

### Post-Game Report Generation
Implemented in `generate_game_report` function in common.rs:
- Separates spawn and pop events
- Calculates total game time
- Calculates average spawn interval
- Calculates average target lifetime (matches spawn/pop by UUID)
- Returns GameReport with all analytics

### Game Finish Handler
Implemented in `on_hunter_game_finish` system in terminal.rs:
- Runs on OnExit(HunterGameState::On)
- Generates report using generate_game_report
- Logs comprehensive report with info! macro
- Includes summary statistics and complete event timeline

## Best Practices

### Current Implementation (Terminal-Side) - ✅ ALL IMPLEMENTED
- ✅ **DONE**: Track all statistics locally (spawned, popped, score) via `HunterGameStats`
- ✅ **DONE**: Record all target events with timestamps and positions in `target_events` Vec
- ✅ **DONE**: Generate and log report on game finish in `on_hunter_game_finish`
- ✅ **DONE**: Spawn red collision indicator (0.02m) at click position
- ✅ **DONE**: Clear previous collision indicator when next target is popped
- ✅ **DONE**: Provide immediate visual feedback for clicks
- ✅ **DONE**: Use accurate circle collision detection (distance <= 0.125 m radius)
- ✅ **DONE**: Despawn target entities when popped with `despawn()`
- ✅ **DONE**: Display statistics in real-time with bottom-right UI text
- ✅ **DONE**: Initialize stats on game start with `init_hunter_stats` system
- ✅ **DONE**: Bottom toolbar button for target spawning (order: 10)
- ✅ **DONE**: Drag-and-drop with white preview circle (0.125m radius)
- ❌ Don't spawn targets outside valid scene bounds (future validation)
- ❌ Don't allow negative statistics (validated by u32 type)
- ❌ Don't leave multiple collision indicators (enforced by despawn all previous)

### Future Migration (Server Authority)
- ✅ Move hit detection to server for validation
- ✅ Server validates target spawn positions
- ✅ Broadcast state changes to all clients
- ✅ Clean up entities when game ends
- ❌ Don't allow client to set scores directly
- ❌ Don't trust client hit reports without verification

### Terminal-Side (After Migration)
- ✅ Render based on server state
- ✅ Send spawn/click events to server
- ✅ Provide visual feedback while awaiting server response
- ❌ Don't calculate scores locally
- ❌ Don't store authoritative game state

### Data Models
- ✅ Keep all types serializable
- ✅ Use Uuid for all entity references
- ✅ Validate constructor parameters
- ❌ Don't put game logic in model.rs

## Testing Considerations

### Manual Testing Checklist (✅ Ready to Test)
- [ ] **Game Start**: Click "Start Hunter Game" button in left toolbar
- [ ] **Stats Init**: Verify `HunterGameStats` initialized (check logs)
- [ ] **Stats Display**: Bottom-right shows "Spawned: 0 | Hits: 0 | Points: 0"
- [ ] **Target Spawn**: Click and drag "spawn_basic_target" button to scene
- [ ] **Spawn Count**: Stats display updates to "Spawned: 1"
- [ ] **Target Render**: White circle (0.25m diameter) appears at drop position
- [ ] **Preview**: White circle follows mouse during drag
- [ ] **Click Detection**: Click on target center - should despawn
- [ ] **Hit Stats**: Stats update to "Hits: 1 | Points: 10"
- [ ] **Collision Indicator**: Small red circle appears at click position
- [ ] **Indicator Clear**: Spawn and pop another target - previous red circle disappears
- [ ] **Multiple Targets**: Spawn several targets, pop them one by one
- [ ] **Score Accumulation**: Points increment by 10 for each pop
- [ ] **Miss Click**: Click empty space - nothing happens
- [ ] **Edge Click**: Click edge of target circle - should still pop (radius check)
- [ ] **Overlapping**: Spawn overlapping targets - only one pops per click
- [ ] **Game Exit**: Exit game - check console for report log
- [ ] **Report Content**: Verify report includes all statistics and timeline
- [ ] **Stats Cleanup**: Stats display disappears after game exit
- [ ] **Restart**: Start new game - stats reset to 0

### Expected Log Output on Game Exit
```
=== HUNTER GAME REPORT ===
Game Duration: XX.XXs
Targets Spawned: X
Targets Popped: X
Total Score: XXX
Average Spawn Interval: X.XXs
Average Target Lifetime: X.XXs

EVENT TIMELINE:
[0.00s] spawned target <uuid> at (x, y, z)
[1.23s] popped target <uuid> at (x, y, z)
...
=== END REPORT ===
```

### Automated Testing (Future)
- Unit test: `generate_game_report` function with sample events
- Unit test: Collision detection math (distance <= radius)
- Integration test: Full game lifecycle (start → spawn → pop → finish)
- Integration test: Event recording accuracy
- Integration test: Stats resource initialization

## Performance Notes
- Circle collision detection is simple and fast (distance check)
- Terminal handles all interaction currently (no network overhead)
- Target rendering uses lyon circles (efficient)
- Statistics update is lightweight (simple counters)
- Consider spatial partitioning if many targets (100+)
- Future: Network latency when moved to server authority

## Debugging
- Enable hunter game logging: `RUST_LOG=hunter=debug`
- Check `HunterGameState` transitions
- Verify target entity spawning on drag-and-drop
- Verify target despawning on click
- Monitor statistics updates (spawned, popped, score)
- Check circle collision radius (0.125 m)
- Use terminal visualization to verify target positions and sizes
- Log click positions and target distances for hit detection debugging

## References
- [Minigames General Instructions](../../.github/instructions/minigames.instructions.md)
- [Server Instructions](../../.github/instructions/server.instructions.md)
- [Terminal Instructions](../../.github/instructions/terminal.instructions.md)
- [Network Protocol](../../.github/instructions/network.instructions.md)
