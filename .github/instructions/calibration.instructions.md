# Calibration System Instructions

**Applies to:** Files related to calibration, scene configuration, camera setup, or projector configuration

## Context
You are working with the LaserTargets calibration system. This handles mapping between camera space, world space, and projector/DAC space for accurate AR laser projection.

## Coordinate Systems

### Three Spaces
1. **World Space**: Physical 3D space (meters)
   - Origin: Scene center
   - Units: Meters
   - Coordinates: (x, y, z)

2. **Camera Space**: Camera view coordinates
   - Origin: Camera position
   - FOV-based projection
   - Configured in `CameraConfiguration`

3. **DAC/Projector Space**: Laser DAC coordinates
   - Range: Typically -32768 to 32767 (16-bit)
   - Origin: Projector calibration dependent
   - Configured in `ProjectorConfiguration`

## Configuration Types

### SceneConfiguration
In `common/src/config.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Resource, PartialEq)]
pub struct SceneConfiguration {
    /// Defines the dimensions (width, height) of the scene in pixels
    pub scene_dimension: UVec2,
    /// Defines the y-difference from the scene origin
    pub y_difference: f32,
    /// Defines the position and orientation of the scene in world space
    pub origin: ConfigTransform,
}
```

Defines the physical scene:
- `scene_dimension`: Width and height in pixels
- `y_difference`: Y-offset from scene origin
- `origin`: Position, rotation, and scale in world space

### CameraConfiguration
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Resource, PartialEq)]
pub struct CameraConfiguration {
    /// Defines the size of the thermal camera viewport in pixels
    pub resolution: UVec2,
    /// Defines the camera's position and orientation in world space
    pub origin: ConfigTransform,
    /// Camera field of view angle in degrees
    pub angle: f32,
    /// Lock camera center to scene transform
    pub locked_to_scene: bool,
}
```

Camera setup:
- `resolution`: Viewport size in pixels
- `origin`: Camera position, rotation, and scale (ConfigTransform)
- `angle`: Field of view in degrees
- `locked_to_scene`: Whether camera follows scene transform

### ProjectorConfiguration
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Resource, PartialEq)]
pub struct ProjectorConfiguration {
    pub resolution: UVec2,
    /// Projection angle in degrees
    pub angle: f32,
    pub origin: ConfigTransform,
    /// Enable or disable projector rendering
    pub switched_on: bool,
    pub connected: bool,
    pub locked_to_scene: bool,
}
```

Projector/DAC setup:
- `resolution`: DAC coordinate range (typically 4095x4095)
- `angle`: Projection angle in degrees
- `origin`: Projector position, rotation, and scale (ConfigTransform)
- `switched_on`: Enable/disable projector rendering
- `connected`: Hardware connection status
- `locked_to_scene`: Whether projector follows scene transform

### ConfigTransform
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Resource, PartialEq)]
pub struct ConfigTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

Shared transform type used by all configurations for consistent positioning.

## Calibration Flow

### Current Implementation
User calibrates projector and scene through the terminal application:

1. **Terminal UI (Settings + Keyboard Commands)**
   - User adjusts scene and projector configurations via settings UI (`terminal/src/plugins/settings.rs`)
   - Keyboard commands provide direct calibration control
   - Configuration changes are synced to server via network messages

2. **Terminal-Side Visualization (`terminal/src/plugins/calibration.rs`)**
   - Draws visual helpers in 3D camera view when `CalibrationVisible` is true
   - Shows scene bounds (billboard grid), projector frustum, calibration guides
   - Draws mouse crosshair, scene center crosshair
   - F3 key toggles calibration visibility
   - Provides real-time feedback during calibration

3. **Server-Side Projection (`server/src/plugins/calibration.rs`)**
   - Receives configuration updates from terminal
   - When `CalibrationVisible` is true, spawns calibration paths:
     - Rectangle path outlining the scene bounds
     - Crosshair path at scene center
   - Applies transforms to convert world coordinates to DAC coordinates
   - Sends actual laser output to Helios DAC for physical projection

### Calibration Workflow
1. Open terminal settings UI
2. Adjust `SceneConfiguration` (dimensions, position, orientation)
3. Adjust `CameraConfiguration` if needed
4. Adjust `ProjectorConfiguration` (position, angle, resolution)
5. Use keyboard commands for fine-tuning
6. Observe visual helpers in terminal 3D view
7. Server projects calibration patterns via laser hardware
8. Iterate until projection aligns with physical scene


## Coordinate Transformations

Coordinate transformations are handled in:
- `server/src/plugins/projector.rs` - World to DAC conversion
- `terminal/src/plugins/calibration.rs` - Camera and screen space calculations

## Network Synchronization

Configuration changes sync between terminal and server:
- Terminal sends `UpdateSceneConfig`, `UpdateCameraConfig`, `UpdateProjectorConfig`
- Server broadcasts updates to all connected clients
- Terminal queries initial state on connection

