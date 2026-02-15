# Settings Instructions

**Applies to:** `terminal/src/plugins/settings.rs`, egui-based configuration UI

## Context
You are working with the LaserTargets settings UI system. This provides a graphical configuration editor using egui for adjusting scene, camera, and projector settings in the terminal application.

## Settings Architecture

### Location
- **Implementation**: `terminal/src/plugins/settings.rs`
- **Purpose**: Provide UI for editing SceneConfiguration, CameraConfiguration, and ProjectorConfiguration
- **Framework**: bevy_egui (immediate mode GUI)

### Key Responsibilities
- Display collapsible configuration panels
- Edit configuration values with sliders, drag values, and checkboxes
- Toggle settings overlay visibility via toolbar button
- Detect and propagate configuration changes to server
- Reset configurations to defaults

## Data Models

### OverlayVisible (Resource)
```rust
pub struct OverlayVisible(pub bool);
```
Controls whether the settings overlay window is visible. Toggled by settings toolbar button.

### SectionExpandedState (Resource)
```rust
pub struct SectionExpandedState {
    pub scene: bool,
    pub camera: bool,
    pub projector: bool,
}
```
Tracks which configuration sections are expanded in the UI.

## Plugin Systems

### Startup Systems
- `register_settings_button`: Spawns settings toolbar button with gear icon (`\u{f04fe}`)

### Update Systems
- `handle_settings_button`: Handles toolbar button clicks, toggles overlay visibility

### EguiPrimaryContextPass Systems
- `overlay_ui_system`: Main egui window rendering system

## Egui UI Patterns

### Window Creation
```rust
egui::Window::new("Configuration Inspector")
    .collapsible(false)
    .default_pos([100.0, 100.0])
    .resizable(true)
    .default_size([600.0, 500.0])
    .show(ctx, |ui| {
        // Window content
    });
```

### Collapsing Sections
```rust
let section_response = egui::CollapsingHeader::new("Section Title")
    .open(Some(section_expanded.scene))
    .show(ui, |ui| {
        // Section content
    });

if section_response.header_response.clicked() {
    section_expanded.scene = !section_expanded.scene;
}
```

### Property Row Layout
Use helper function for consistent two-column layout:
```rust
property_row(ui, "Label", |ui| {
    ui.add(egui::DragValue::new(&mut value)
        .range(0.0..=100.0)
        .speed(0.1)
        .suffix(" units"))
});
```

### Common Widgets
```rust
// Drag value (numeric input)
ui.add(egui::DragValue::new(&mut value)
    .range(min..=max)
    .speed(increment)
    .prefix("Label: ")
    .suffix(" units"));

// Slider
ui.add(egui::Slider::new(&mut value, min..=max)
    .text("Label"));

// Checkbox
ui.checkbox(&mut enabled, "Enable feature");

// ComboBox (dropdown)
egui::ComboBox::from_id_salt("unique_id")
    .selected_text("Current")
    .show_ui(ui, |ui| {
        ui.selectable_value(&mut mode, 0, "Option 1");
        ui.selectable_value(&mut mode, 1, "Option 2");
    });

// Reset button
if ui.button("Reset").clicked() {
    *config = ConfigType::default();
}
```

## Change Detection Pattern

### Problem
Egui directly mutates resources, which triggers Bevy's change detection even when values haven't actually changed. This causes unnecessary network messages.

### Solution
Use `bypass_change_detection()` and manual `set_changed()`:

```rust
fn overlay_ui_system(
    mut scene_configuration: ResMut<SceneConfiguration>,
    // ... other configs
) {
    // Clone originals
    let orig_scene_config = scene_configuration.clone();
    
    // Scope for bypassed references
    {
        let scene_config_ref = scene_configuration.bypass_change_detection();
        
        // Egui modifies scene_config_ref directly
        egui::Window::new("Settings").show(ctx, |ui| {
            ui.add(egui::DragValue::new(&mut scene_config_ref.field));
        });
    } // Bypassed references dropped
    
    // Only trigger change if actually changed
    if *scene_configuration != orig_scene_config {
        scene_configuration.set_changed();
    }
}
```

**Key Points:**
- Clone configurations before UI
- Use `bypass_change_detection()` for egui mutations
- Drop bypassed references before calling `set_changed()`
- Only call `set_changed()` when values actually differ

## Current Configuration Sections

### Scene Configuration
- **Target Distance**: Z position of scene (meters)
- **Scene Dimensions**: Width and height in pixels
- **Y-Difference from Origin**: Vertical offset
- **Scene Origin Y**: Y position of scene origin

### Camera Configuration
- **Display Mode**: 2D or 3D view mode
- **Field of View**: Camera FOV angle (10-120 degrees)
- **Camera Position**: Y position of camera

### Projector Configuration
- **Enabled**: Toggle projector on/off (checkbox)
- **Resolution**: Display width/height (read-only)
- **Projector Position**: Y position of projector

## Reset Functionality

### Individual Section Reset
Each section has a "Reset" button that resets only that configuration:
```rust
if ui.button("Reset").clicked() {
    *scene_config_ref = SceneConfiguration::default();
}
```

### Reset All
Top-level "Reset All to Defaults" button:
```rust
if ui.button("🔄 Reset All to Defaults").clicked() {
    *scene_config_ref = SceneConfiguration::default();
    *camera_config_ref = CameraConfiguration::default();
    *projector_config_ref = ProjectorConfiguration::default();
    *display_mode = DisplayMode::Mode2D;
}
```

## Toolbar Integration

### Settings Button
- **Icon**: `\u{f04fe}` (gear icon, Nerd Font)
- **Docking**: Right
- **Order**: 2
- **State**: On when overlay visible, Off when hidden

### Button Click Handler
```rust
fn handle_settings_button(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut overlay_visible: ResMut<OverlayVisible>,
    mut settings_button_query: Query<&mut ToolbarItem, With<SettingsButton>>,
) {
    for (interaction, button) in &button_query {
        if button.name == "settings" && *interaction == Interaction::Pressed {
            overlay_visible.0 = !overlay_visible.0;
            
            if let Ok(mut item) = settings_button_query.single_mut() {
                item.state = if overlay_visible.0 { 
                    ItemState::On 
                } else { 
                    ItemState::Off 
                };
            }
        }
    }
}
```

## Performance Considerations

### Early Return
Check overlay visibility first to skip all UI work when hidden:
```rust
fn overlay_ui_system(
    overlay_visible: Res<OverlayVisible>,
    // ...
) {
    if !overlay_visible.0 {
        return;  // Skip all egui work
    }
    
    // UI rendering...
}
```

### ScrollArea for Long Content
Use `egui::ScrollArea` when sections may exceed window height:
```rust
egui::ScrollArea::vertical().show(ui, |ui| {
    // Long content sections
});
```

## Best Practices

### DO ✅
- Use `bypass_change_detection()` to prevent spurious change triggers
- Clone configs before UI to compare after
- Only call `set_changed()` when values actually differ
- Early return when overlay not visible
- Use `property_row` helper for consistent layout
- Provide reset buttons for user convenience
- Use semantic ranges for drag values (e.g., 10-120 for FOV)
- Add suffix/prefix to units for clarity
- Use appropriate widget types (DragValue for precise, Slider for ranges)

### DON'T ❌
- Don't let egui directly modify resources without change control
- Don't forget to update toolbar button state
- Don't block with heavy computation in immediate mode UI
- Don't forget `.after(CalibrationSystemSet)` for system ordering
- Don't make UI too cluttered (use collapsing sections)
- Don't use fixed sizes (make window resizable)
- Don't forget to handle early return when overlay hidden

## Adding New Configuration Sections

### Step 1: Add to Settings UI
```rust
// In overlay_ui_system, after existing sections
let new_response = egui::CollapsingHeader::new("New Section")
    .open(Some(section_expanded.new_section))
    .show(ui, |ui| {
        property_row(ui, "Property Name", |ui| {
            ui.add(egui::DragValue::new(&mut config.field))
        });
    });

if new_response.header_response.clicked() {
    section_expanded.new_section = !section_expanded.new_section;
}
```

### Step 2: Add to SectionExpandedState
```rust
pub struct SectionExpandedState {
    pub scene: bool,
    pub camera: bool,
    pub projector: bool,
    pub new_section: bool,  // Add new field
}

impl Default for SectionExpandedState {
    fn default() -> Self {
        Self {
            // ...
            new_section: true,
        }
    }
}
```

### Step 3: Add Change Detection
```rust
// Clone before UI
let orig_new_config = new_config.clone();

// Use bypassed reference in UI
let new_config_ref = new_config.bypass_change_detection();

// After UI scope
if *new_config != orig_new_config {
    new_config.set_changed();
}
```

## Debugging

### Common Issues
1. **Changes not propagating**: Check `set_changed()` is called after comparing
2. **Spurious network messages**: Ensure using `bypass_change_detection()`
3. **UI not updating**: Verify overlay_visible is true
4. **Button state wrong**: Check toolbar button state is synced with overlay visibility

### Logging
```rust
debug!("Settings UI: Config changed, triggering change detection");
```

Enable with:
```bash
RUST_LOG=settings=debug cargo run -p terminal --features bevy/dynamic_linking
```

## Testing
- Test all widget interactions (drag, click, checkbox)
- Verify reset buttons work correctly
- Test change detection only triggers on actual changes
- Verify toolbar button state syncs with overlay
- Test with various configuration values
- Ensure UI doesn't slow down when hidden

## References
- Implementation: [terminal/src/plugins/settings.rs](../../../terminal/src/plugins/settings.rs)
- Toolbar integration: [.github/instructions/toolbar.instructions.md](toolbar.instructions.md)
- Scene config: [common/src/config.rs](../../../common/src/config.rs)
- Egui documentation: https://docs.rs/egui/
