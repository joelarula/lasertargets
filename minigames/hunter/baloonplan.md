# Hunter Minigame

## Overview
The Hunter minigame is a laser target shooting game where players pop targets projected onto a physical scene using laser hardware. The terminal provides a UI for spawning and interacting with targets, while the server manages authoritative game state, collision detection, and laser projection.

## Features

### Basic Target
- Drag-and-drop from toolbar to place a static circle target at a specific position on the scene
- Click to pop (despawn) the target
- Scores points on hit

### Balloon Target (Planned)

#### Overview
A new target type that spawns below the scene and rises upward. The player must click (pop) the balloon before it exits the top of the scene. If the balloon escapes without being popped, it counts as a miss.

#### Behavior
1. **Spawn**: Clicking the balloon toolbar button spawns a balloon at a random X position, starting just below the visible scene area
2. **Rise**: The balloon moves upward at a configurable speed (e.g., 0.3 units/sec)
3. **Pop**: Clicking the balloon while it is inside the scene despawns it, scores points, and increments `targets_popped`
4. **Escape**: If the balloon passes the top edge of the scene without being clicked, it is despawned and `misses` is incremented

#### Implementation Plan

##### 1. Common Types (`common/src/target.rs`)
- Add `Balloon(f32, Color)` variant to `HunterTarget` enum (size, color)

##### 2. Balloon Path Shape (`common/src/path.rs`)
- Add `UniversalPath::balloon(center, radius, color)` method
- Start with a differently-colored circle;

##### 3. Hunter Model (`minigames/hunter/src/model.rs`)
- Add `BalloonTargetEntity` marker component
- Add `BalloonRiseSpeed` component (stores vertical speed, e.g., 0.3 units/sec)

##### 4. Server Logic (`minigames/hunter/src/server.rs`)

| System | Schedule | Description |
|--------|----------|-------------|
| `handle_spawn_balloon` | Update (InGame) | Receives `SpawnHunterTarget::Balloon` from network, picks random X within scene width, spawns entity at Y = scene bottom − radius, creates rising path |
| `update_balloon_positions` | FixedUpdate (InGame) | Moves all `BalloonTargetEntity` upward by `BalloonRiseSpeed × dt`, broadcasts `UpdatePathPosition` |
| `check_balloon_out_of_bounds` | Update (InGame) | If balloon Y > scene top + radius: increment `misses`, despawn entity, broadcast `DespawnPath` + stats update |

- Balloon entities also carry `HunterTargetEntity` so existing click/pop detection works unchanged
- Existing `handle_target_click` handles despawn + score for any `HunterTargetEntity`

**Spawn flow:**
1. Terminal sends `NetworkMessage::SpawnHunterTarget(HunterTarget::Balloon(..), Vec3::ZERO)` — position is ignored, server picks random X
2. Server computes random X ∈ `[−scene_half_width + margin, scene_half_width − margin]`
3. Server computes start Y = `−scene_half_height − balloon_radius` (below scene)
4. Spawns entity with `BalloonTargetEntity`, `BalloonRiseSpeed(0.3)`, `HunterTargetEntity`, path, transform
5. Broadcasts `SpawnPath` to terminal

**Movement flow (FixedUpdate):**
1. For each `BalloonTargetEntity`, translate Y += speed × dt
2. Broadcast `UpdatePathPosition` with new position

**Out-of-bounds flow:**
1. If balloon Y > `scene_half_height + balloon_radius`:
   - Increment `stats.misses`
   - Despawn entity
   - Broadcast `DespawnPath` + `HunterStatsUpdate`

##### 5. Terminal UI (`minigames/hunter/src/terminal.rs`)

| Item | Type | Description |
|------|------|-------------|
| `BalloonButton` | Component (marker) | Marker for the toolbar button entity |
| `SPAWN_BALLOON_BTN` | const | `"spawn_balloon_target"` |
| `spawn_balloon_toolbar_item` | System (OnEnter HunterGameState::On) | Spawns toolbar button with balloon icon |
| `despawn_balloon_toolbar_item` | System (OnExit HunterGameState::On) | Despawns toolbar button |
| `handle_balloon_button_click` | System (Update, InGame) | On click, sends `SpawnHunterTarget(Balloon(0.2, color))` to server |

- Simple click (not drag-and-drop) since position is random
- Terminal does **not** move the balloon locally — server broadcasts `UpdatePathPosition` each tick, and existing `handle_update_path_position_events` in `terminal/src/plugins/path.rs` handles rendering

##### 6. Network Layer
- No new network messages needed
- `SpawnHunterTarget` already supports new `HunterTarget` variants
- `UpdatePathPosition`, `DespawnPath`, `HunterStatsUpdate` already exist

#### File Change Summary

| File | Change |
|------|--------|
| `common/src/target.rs` | Add `Balloon(f32, Color)` variant to `HunterTarget` |
| `common/src/path.rs` | Add `UniversalPath::balloon()` shape method |
| `minigames/hunter/src/model.rs` | Add `BalloonTargetEntity`, `BalloonRiseSpeed` components |
| `minigames/hunter/src/server.rs` | Add `handle_spawn_balloon`, `update_balloon_positions`, `check_balloon_out_of_bounds` systems |
| `minigames/hunter/src/terminal.rs` | Add `BalloonButton` marker, toolbar spawn/despawn, click handler |

#### Implementation Order
1. `common/src/target.rs` — Add enum variant (everything depends on this)
2. `common/src/path.rs` — Add balloon shape generator
3. `minigames/hunter/src/model.rs` — Add components
4. `minigames/hunter/src/server.rs` — Server logic (spawn, rise, out-of-bounds)
5. `minigames/hunter/src/terminal.rs` — Toolbar button + click handler
6. Build & test

#### Open Questions
1. **Rise speed** — Fixed at 0.3 units/sec, or configurable per balloon?
2. **Balloon shape** — Start with colored circle, or full balloon with tail from day one?
3. **Random color** — Each balloon random color, or fixed?
4. **Multiple spawns** — One click = one balloon, or start a continuous spawn stream?

## Stats
The Hunter game tracks the following statistics in real-time:
- **Targets Spawned**: Total number of targets created
- **Targets Popped (Hits)**: Targets successfully clicked/destroyed
- **Misses**: Clicks inside the scene that miss all targets, plus balloon escapes
- **Score**: Points accumulated from hits

## Architecture
- **Server** (`server.rs`): Authoritative game logic, collision detection, target lifecycle
- **Terminal** (`terminal.rs`): UI toolbar buttons, drag-and-drop spawning, stats display, gizmo rendering
- **Model** (`model.rs`): Shared data structures (stats, components, events)
- **Common** (`common.rs`): Game registration, report generation, shared plugin