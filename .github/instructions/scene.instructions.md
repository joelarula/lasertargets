# Scene Instructions

**Applies to:** `server/src/plugins/scene.rs`, `terminal/src/plugins/scene.rs`, `common/src/scene.rs`

## Context
You are working with the LaserTargets scene system. The scene defines the physical AR space where the laser game takes place, including dimensions, position, entities, and configuration.

## Architecture

### Common (`common/src/scene.rs`)
- **SceneConfiguration**: Physical scene bounds and transform
- **SceneData**: Runtime scene state and mouse interaction
- **SceneSetup**: Combined configuration resource
- **SceneEntity**: Entities within the scene
- All types must be serializable

### Server (`server/src/plugins/scene.rs`)
- **Authoritative**: Manages true scene state
- **Entity Management**: Spawns and updates scene entities
- **Network Sync**: Broadcasts scene changes to clients
- **State Validation**: Validates scene modifications

### Terminal (`terminal/src/plugins/scene.rs`)
- **Scene Editor**: UI for configuring scene parameters
- **Visualization**: Displays scene bounds and helpers
- **Mouse Interaction**: Tracks mouse position in scene space
- **Configuration UI**: Exposes scene settings to user

## Scene Configuration

### SceneConfiguration
Defined in `common/src/config.rs`:
- `scene_dimension`: Width and height in pixels (UVec2)
- `y_difference`: Y-offset from scene origin
- `origin`: Position, rotation, and scale in world space (ConfigTransform)

### SceneData
Runtime data in `common/src/scene.rs`:
- Current scene state
- Mouse position in world coordinates
- Interactive scene elements
- Calculated scene properties

### SceneSetup
Combined resource that derives from all configurations:
- Scene configuration
- Camera configuration  
- Projector configuration
- Provides calculated helper methods

## Scene Entities

### SceneEntity
Entities that exist within the scene:
- Targets, obstacles, game objects
- Position relative to scene origin
- Transform updates when scene configuration changes
- Network synchronized between server and terminal

## Scene Coordinate System

### World Space Alignment
- Scene origin defines the center of the AR space
- Scene rotation affects all entity transforms
- Scene scale affects dimensions and positions
- All scene entities positioned relative to scene origin

### Mouse Interaction
- Terminal converts screen coordinates to scene world coordinates
- Raycasting from camera through mouse position
- Intersection with scene plane for 3D positioning
- `SceneData.mouse_world_pos` stores current intersection

## Configuration Flow

### From Terminal to Server
1. User adjusts scene in terminal UI (settings or scene editor)
2. Terminal updates local `SceneConfiguration` resource
3. Terminal sends `UpdateSceneConfig` message
4. Server receives and updates its `SceneConfiguration`
5. Server broadcasts `SceneConfigUpdate` to all clients
6. Server re-calculates scene-dependent systems

### Scene Update Cascade
When scene configuration changes:
- Entity transforms may need recalculation
- Calibration visualization updates
- Mouse intersection plane updates
- Camera updates if locked to scene
- Projector updates if locked to scene

## Scene Editor (Terminal)

### Interactive Editing
Scene editor provides UI for:
- Adjusting scene dimensions
- Positioning scene origin
- Rotating scene plane
- Previewing changes in 3D view

### Visual Feedback
- Billboard grid showing scene bounds
- Coordinate axes at scene origin
- Measurement helpers
- Real-time preview of changes

## Network Synchronization

### Queries
- Terminal queries scene config on connection: `QuerySceneConfig`
- Server responds with current state: `SceneConfigUpdate`

### Updates
- Terminal sends modifications: `UpdateSceneConfig`
- Server validates and applies changes
- Server broadcasts to all clients: `SceneConfigUpdate`
- All clients update local scene state

## Best Practices
- ✅ Always validate scene configuration before applying
- ✅ Broadcast scene changes to all connected clients
- ✅ Update dependent systems when scene changes
- ✅ Use scene-relative coordinates for entities
- ✅ Provide visual feedback in terminal for scene bounds
- ✅ Keep scene configuration synchronized across network
- ❌ Don't store scene state in multiple places
- ❌ Don't bypass configuration system for scene changes
- ❌ Don't forget to update SceneSetup when configs change
- ❌ Don't apply scene changes without validation (server)
- ❌ Don't assume terminal scene state is authoritative

## Integration Points
- **Calibration**: Scene bounds define calibration target area
- **Camera**: Camera can be locked to scene transform
- **Projector**: Projector can be locked to scene transform
- **Entities**: All game entities positioned relative to scene
- **Mouse**: Mouse intersection calculated against scene plane
- **Settings**: Scene parameters exposed in settings UI
