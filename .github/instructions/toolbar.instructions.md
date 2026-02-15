# Toolbar Instructions

**Applies to:** `terminal/src/plugins/toolbar.rs`, toolbar-related UI components, `common/src/toolbar.rs`

## Context
You are working with the LaserTargets terminal toolbar system. The toolbar provides the main control interface for the application using Bevy UI components (not egui). It's a declarative system where plugins spawn `ToolbarItem` entities to create buttons, and the toolbar plugin renders and manages them automatically.

## Toolbar Architecture

### Location
- **Implementation**: `terminal/src/plugins/toolbar.rs` (rendering and interaction)
- **Data Models**: `common/src/toolbar.rs` (ToolbarItem, ToolbarButton components)
- **Purpose**: Declarative UI control panel for terminal application
- **Framework**: Bevy native UI components (Node, Button, Text)

### Key Responsibilities
- Automatically render buttons from spawned `ToolbarItem` entities
- Handle button interactions (hover, pressed, none)
- Update button visual states based on `ItemState` changes
- Support multiple docking locations (Left, Right, Top, Bottom, StatusBar)
- Rebuild toolbar when items are added/changed/removed

## Data Models

### ToolbarItem (Component)
Defined in `common/src/toolbar.rs`. Spawn this as an entity to create a toolbar button:
```rust
pub struct ToolbarItem {
    pub name: String,           // Unique identifier
    pub order: u8,              // Sort order within docking location
    pub icon: Option<String>,   // Nerd font icon (e.g., "\u{f0eb}")
    pub text: Option<String>,   // Text label (unused in current impl)
    pub state: ItemState,       // Disabled, On, or Off
    pub docking: Docking,       // Where to place button
    pub button_size: f32,       // Button size in pixels (default 36.0)
    pub margin_before: f32,     // Margin before button (unused)
    pub margin_after: f32,      // Margin after button (unused)
}
```

### ItemState (Enum)
```rust
pub enum ItemState {
    Disabled,  // Button grayed out, not clickable
    On,        // Active/selected state (blue)
    Off,       // Inactive/default state (gray)
}
```

### Docking (Enum)
```rust
pub enum Docking {
    Left,       // Vertical column on left side
    Right,      // Vertical column on right side
    Top,        // Horizontal row at top
    Bottom,     // Horizontal row at bottom
    StatusBar,  // Bottom status bar (full width)
}
```

### ToolbarButton (Component)
Internal component added by toolbar plugin to rendered buttons. References the `ToolbarItem.name`.

## How It Works

### 1. Registration Pattern
Plugins spawn `ToolbarItem` entities to add buttons to the toolbar:

```rust
fn register_my_button(mut commands: Commands) {
    commands.spawn((
        ToolbarItem {
            name: "my_button".to_string(),
            order: 1,  // Controls sort order
            icon: Some("\u{f0eb}".to_string()),  // Nerd font icon
            state: ItemState::Off,
            docking: Docking::Right,
            button_size: 36.0,
            ..default()
        },
        MyButtonMarker,  // Your own marker component for queries
    ));
}
```

### 2. Automatic Rendering
The toolbar plugin automatically:
- Queries all `ToolbarItem` entities
- Groups by `docking` location
- Sorts by `order` within each group
- Spawns Bevy UI button entities with correct styles
- Watches for `Added<ToolbarItem>` or `Changed<ToolbarItem>` to rebuild

### 3. Button Interaction Handling
Each plugin handles clicks on its own buttons:

```rust
fn handle_button_click(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut my_state: ResMut<MyState>,
    mut item_query: Query<&mut ToolbarItem, With<MyButtonMarker>>,
) {
    for (interaction, toolbar_button) in &button_query {
        if toolbar_button.name == "my_button" && *interaction == Interaction::Pressed {
            // Toggle your feature
            my_state.enabled = !my_state.enabled;
            
            // Update button state
            if let Ok(mut item) = item_query.single_mut() {
                item.state = if my_state.enabled { 
                    ItemState::On 
                } else { 
                    ItemState::Off 
                };
            }
        }
    }
}
```

### 4. State Synchronization
Keep button state in sync with your plugin's state:

```rust
fn sync_button_with_state(
    my_state: Res<MyState>,
    mut item_query: Query<&mut ToolbarItem, With<MyButtonMarker>>,
) {
    if my_state.is_changed() {
        if let Ok(mut item) = item_query.single_mut() {
            item.state = match my_state.mode {
                Mode::Disabled => ItemState::Disabled,
                Mode::Active => ItemState::On,
                Mode::Inactive => ItemState::Off,
            };
        }
    }
}
```

## Visual System

### Button Colors
Defined in `button_colors` module in toolbar.rs:
- **PRESSED**: `Color::srgba(0.25, 0.35, 0.45, 0.95)` - Dark blue when clicked
- **ACTIVE_HOVERED**: `Color::srgba(0.35, 0.55, 0.65, 0.9)` - Bright blue when hovering over active
- **INACTIVE_HOVERED**: `Color::srgba(0.45, 0.50, 0.55, 0.85)` - Light gray when hovering over inactive
- **ACTIVE**: `Color::srgba(0.30, 0.48, 0.58, 0.85)` - Medium blue for active/on state
- **INACTIVE**: `Color::srgba(0.35, 0.40, 0.45, 0.7)` - Gray for inactive/off state

Colors automatically update based on `ItemState` and `Interaction`.

### Font
- Uses Nerd Font (FiraCodeNerdFont-Regular.ttf) for icons
- Loaded in Startup system and stored as `NerdFont` resource
- Falls back to default font if icon is None or font not loaded
- Font size: 24.0px

### Layout
- **Left/Right docking**: Vertical column with `row_gap: 10px`
- **Top/Bottom docking**: Horizontal row with `column_gap: 10px`
- **StatusBar**: Full-width bottom bar (28px height) with space-between justify
- Buttons have 6px margin, 6px border radius
- Toolbar container has z-index 1000 (always on top)

## System Architecture

### ToolbarPlugin Systems
1. **Startup**:
   - `load_nerd_font`: Load FiraCodeNerdFont-Regular.ttf
   
2. **PostStartup**:
   - `setup_toolbar`: Initial toolbar creation
   
3. **Update** (chained):
   - `update_button_states`: Update colors based on ItemState and Interaction
   - `update_button_text`: Update text when ToolbarItem changes
   - `rebuild_toolbar`: Despawn and recreate toolbar when items added/changed

### Rebuild Trigger
Toolbar automatically rebuilds when:
- `Added<ToolbarItem>` detected (new button added)
- `Changed<ToolbarItem>` detected (button properties modified)

This allows plugins to dynamically add/remove buttons at runtime.

## Common Patterns

### Example: Settings Button
From `terminal/src/plugins/settings.rs`:
```rust
// 1. Register button in Startup
commands.spawn((
    ToolbarItem {
        name: "settings".to_string(),
        order: 2,
        icon: Some("\u{f04fe}".to_string()),  // Settings icon
        state: ItemState::Off,
        docking: Docking::Right,
        button_size: 36.0,
        ..default()
    },
    SettingsButton,  // Marker component
));

// 2. Handle clicks in Update
fn handle_button(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut overlay_visible: ResMut<OverlayVisible>,
    mut item_query: Query<&mut ToolbarItem, With<SettingsButton>>,
) {
    for (interaction, button) in &button_query {
        if button.name == "settings" && *interaction == Interaction::Pressed {
            overlay_visible.0 = !overlay_visible.0;
            
            if let Ok(mut item) = item_query.single_mut() {
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

### Example: Projector Button
From `terminal/src/plugins/projector.rs`:
```rust
// Register with initial Disabled state
commands.spawn((
    ProjectorButton,
    ToolbarItem {
        name: "projector".to_string(),
        order: 1,
        icon: Some("\u{f0eb}".to_string()),  // Laser icon
        state: ItemState::Disabled,  // Disabled until connected
        docking: Docking::Right,
        ..default()
    },
));

// Sync button state with config changes
fn sync_toolbar_with_config(
    projector_config: Res<ProjectorConfiguration>,
    mut item_query: Query<&mut ToolbarItem, With<ProjectorButton>>,
) {
    if projector_config.is_changed() {
        if let Ok(mut item) = item_query.single_mut() {
            item.state = if !projector_config.connected {
                ItemState::Disabled
            } else if projector_config.switched_on { 
                ItemState::On 
            } else { 
                ItemState::Off 
            };
        }
    }
}
```

## UI Guidelines

### Button Organization
- Use `order` field to control button sequence within docking location
- Lower order numbers appear first (left-to-right, top-to-bottom)
- Group related functionality with consecutive order numbers
- Leave gaps (e.g., 1, 2, 5, 10) to allow inserting buttons later

### Icon Selection
- Use Nerd Font icons for consistent visual style
- Common icons:
  - Settings: `\u{f04fe}`
  - Laser/Projector: `\u{f0eb}`
  - Play: `\u{f04b}`
  - Pause: `\u{f04c}`
  - Stop: `\u{f04d}`
- Find icons at: https://www.nerdfonts.com/cheat-sheet

### State Management
- Use `ItemState::Disabled` when action is unavailable (grayed out)
- Use `ItemState::On` when feature is active (blue highlight)
- Use `ItemState::Off` when feature is inactive (gray)
- Always update state when underlying resource changes



## Best Practices

### DO ✅
- Spawn `ToolbarItem` entities in your plugin's Startup systems
- Use unique names for each button (prevents conflicts)
- Add marker components to ToolbarItem entities for easy querying
- Update `ItemState` when your plugin's state changes
- Use `is_changed()` to avoid unnecessary state updates
- Check `toolbar_button.name` to identify which button was clicked
- Use appropriate `ItemState` values (Disabled when unavailable)
- Sort buttons logically with the `order` field

### DON'T ❌
- Don't put complex logic in button handlers (keep them simple)
- Don't store authoritative state in ToolbarItem (it's just UI state)
- Don't manually despawn/spawn toolbar UI (toolbar plugin handles this)
- Don't use same name for multiple buttons
- Don't forget to handle `Interaction::Pressed` specifically
- Don't update ItemState every frame (only when actually changed)
- Don't hardcode button positions (use docking and order)

## Debugging

### Common Issues
1. **Button not appearing**: 
   - Check `ToolbarItem` entity was spawned
   - Verify name is unique
   - Check if docking location is visible

2. **Click not working**:
   - Ensure you're checking `Interaction::Pressed`
   - Verify `toolbar_button.name` matches your button
   - Check if `ItemState::Disabled` (disabled buttons don't click)

3. **Button state not updating**:
   - Verify you're mutating the `ToolbarItem` component
   - Check query has correct marker component (With<YourMarker>)
   - Ensure single_mut() succeeds (entity exists)

4. **Icon not showing**:
   - Verify Nerd Font is loaded (assets/FiraCodeNerdFont-Regular.ttf)
   - Check icon unicode is correct
   - Font must be loaded before PostStartup

### Logging
```rust
RUST_LOG=toolbar=debug cargo run -p terminal --features bevy/dynamic_linking
```

## Testing
- Test button interactions (click, hover, disabled state)
- Verify state synchronization with plugin resources
- Test dynamic button addition/removal
- Test with multiple buttons in same docking location
- Verify order sorting works correctly
- Test all docking locations (Left, Right, Top, Bottom, StatusBar)

## References
- Implementation: [terminal/src/plugins/toolbar.rs](../../../terminal/src/plugins/toolbar.rs)
- Data models: [common/src/toolbar.rs](../../../common/src/toolbar.rs)
- Example usage: [terminal/src/plugins/settings.rs](../../../terminal/src/plugins/settings.rs)
- Example usage: [terminal/src/plugins/projector.rs](../../../terminal/src/plugins/projector.rs)
