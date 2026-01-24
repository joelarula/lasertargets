use bevy::ecs::component::Component;

pub struct ToolbarPlugin;


#[derive(Component)]
pub struct ToolabarButton {
    pub name: String,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Disabled,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Docking {
    Left,
    Right,
    Top,
    Bottom,
    StatusBar
}

/// Component that defines a toolbar button item.
/// Spawn this as an entity in your plugin to create a toolbar button.
#[derive(Component, Clone)]
pub struct ToolbarItem {
    pub name: String,
    pub order: u8,
    pub icon: Option<String>,
    pub text: Option<String>,
    pub state: ItemState,
    pub docking: Docking,
    pub button_size: f32,
    pub margin_before: f32,
    pub margin_after: f32,
}

impl Default for ToolbarItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            order: 0,
            icon: None,
            text: None,
            state: ItemState::Off,
            docking: Docking::Left,
            button_size: 36.0,
            margin_before: 0.0,
            margin_after: 0.0,
        }
    }
}