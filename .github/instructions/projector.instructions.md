# Projector Instructions

**Applies to:** `server/src/plugins/projector.rs`, `terminal/src/plugins/projector.rs`, projector control and visualization

## Context
You are working with the LaserTargets projector system. The server side controls actual laser hardware output, while the terminal side provides preview and visualization of projected content.

## Architecture Split

### Server-Side (`server/src/plugins/projector.rs`)
- **Authoritative**: Controls actual laser projector hardware
- **Output**: Sends commands to Helios DAC for laser projection
- **Transforms**: Converts world coordinates to DAC coordinates
- **Calibration**: Applies projector calibration transforms
- **Fixed Timestep**: Runs in FixedUpdate schedule at 50 FPS

### Terminal-Side (`terminal/src/plugins/projector.rs`)
- **Visualization**: Shows preview of projected content
- **Non-authoritative**: Displays what server is projecting
- **UI Integration**: Provides visual feedback for calibration
- **No Hardware**: Does not control physical hardware

## Projector Configuration

### ProjectorConfiguration
Defined in `common/src/config.rs`:
- `resolution`: DAC coordinate range (typically 4095x4095)
- `angle`: Projection angle in degrees
- `origin`: Projector position, rotation, and scale (ConfigTransform)
- `switched_on`: Enable/disable projector rendering
- `connected`: Hardware connection status
- `locked_to_scene`: Whether projector follows scene transform

## Server-Side Responsibilities

### Coordinate Transformation
- Convert world space coordinates to DAC space
- Apply calibration transforms before output
- Handle coordinate system mapping (see calibration.instructions.md)

### Hardware Communication
- Interface with Helios DAC via `dac/helios.rs`
- Send laser point data for projection
- Handle hardware connection state
- Manage frame timing for smooth projection

### Path Rendering
- Process `UniversalPath` objects for projection
- Tessellate paths into laser points
- Apply color and intensity values
- Optimize point count for performance

## Terminal-Side Responsibilities

### Preview Rendering
- Visualize projected content in 3D view
- Show projector frustum during calibration
- Display projection area overlay
- Sync with `CalibrationVisible` state

### Configuration UI
- Expose projector settings in UI
- Provide controls for calibration adjustment
- Show connection status
- Toggle projector on/off

## Calibration Integration

### Calibration Patterns
When `CalibrationVisible` is true:
- Server projects rectangle path (scene bounds)
- Server projects crosshair path (scene center)
- Terminal shows corresponding visualization helpers
- Both use same configuration for alignment

### Transform Application
- Projector origin affects projection mapping
- Rotation affects projection direction
- Scale affects projection size
- All transforms applied before DAC output

## Network Synchronization

### Configuration Sync
- Terminal sends `UpdateProjectorConfig` on changes
- Server broadcasts `ProjectorConfigUpdate` to all clients
- Terminal queries initial config on connection

### State Updates
- Projector on/off state synced via network
- Connection status broadcast to clients
- Configuration changes applied immediately on both sides

## Best Practices
- ✅ Always apply calibration transforms server-side before DAC output
- ✅ Sync projector state between server and terminal
- ✅ Handle hardware connection failures gracefully
- ✅ Use FixedUpdate for server-side projection timing
- ✅ Validate DAC coordinates are within range
- ✅ Provide visual feedback for calibration
- ❌ Don't send DAC commands from terminal
- ❌ Don't bypass configuration system
- ❌ Don't assume hardware is always connected
- ❌ Don't skip coordinate validation before DAC output

## Common Pitfalls
- Forgetting to apply calibration transforms → misaligned projection
- Not validating DAC coordinate range → hardware errors
- Terminal trying to control hardware → architecture violation
- Missing network sync → desynchronized state
- Blocking on hardware I/O → frame drops
