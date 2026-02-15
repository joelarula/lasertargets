# Camera Instructions

**Applies to:** `terminal/src/plugins/camera.rs`, camera configuration and visualization

## Context
You are working with the LaserTargets camera system in the terminal application. This manages the 3D view camera for visualizing the AR scene, calibration helpers, and game state.

## Key Responsibilities
- Provide 3D view of the scene for calibration and visualization
- Support multiple display modes (2D/3D views)
- Handle camera positioning and orientation
- Integrate with calibration system for proper scene alignment
- Respond to camera configuration changes

## Camera Modes

### Display Modes
The camera supports different visualization modes:
- **3D Mode**: Full 3D perspective view for scene visualization and calibration
- **2D Mode**: Top-down or orthographic view for specific use cases

Camera mode is typically controlled via `DisplayMode` resource.

## Camera Configuration

### CameraConfiguration
Defined in `common/src/config.rs`:
- `resolution`: Viewport size in pixels
- `origin`: Camera position, rotation, and scale (ConfigTransform)
- `angle`: Field of view in degrees
- `locked_to_scene`: Whether camera follows scene transform

The camera plugin reacts to changes in `CameraConfiguration` resource.

## Important Patterns

### Camera Updates
- Camera transform updates when configuration changes
- Use `is_changed()` to detect configuration updates
- Apply transforms in systems scheduled after configuration updates

### Scene Integration
- Camera position relative to scene center
- When `locked_to_scene` is true, camera follows scene transforms
- Coordinate with calibration system for proper alignment

### System Ordering
Use `CameraSystemSet` to ensure proper execution order:
- Camera updates should run before rendering systems
- Calibration visualization runs after camera updates

## Common Operations

### Adjusting Camera View
Camera view can be adjusted through:
- `CameraConfiguration` resource changes
- Direct transform manipulation for user controls (pan, zoom, rotate)
- Keyboard/mouse input handling for interactive camera control

### Viewport Management
- Handle window resize events
- Update camera projection parameters
- Maintain aspect ratio for proper scene visualization

## Best Practices
- ✅ Use `CameraSystemSet` for proper system ordering
- ✅ React to configuration changes, don't poll
- ✅ Coordinate with scene and calibration systems
- ✅ Handle viewport changes gracefully
- ✅ Provide smooth camera transitions when mode switching
- ❌ Don't store duplicate camera state
- ❌ Don't implement camera logic in multiple places
- ❌ Don't bypass configuration system for camera changes
- ❌ Don't forget to sync camera updates with rendering

## Integration Points
- **Calibration**: Camera positioning affects calibration visualization
- **Mouse**: Screen coordinates depend on camera projection
- **Scene**: Camera view centers on scene for proper visualization
- **Settings UI**: Camera parameters exposed for user adjustment
