use bevy::ecs::component::Component;

pub struct ToolbarPlugin;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Docking {
    Left,
    Right,
    Top,
    Bottom,
}

/// Component that defines a toolbar button item.
/// Spawn this as an entity in your plugin to create a toolbar button.
#[derive(Component, Clone)]
pub struct ToolbarItem {
    pub name: String,
    pub icon: Option<String>,
    pub text: Option<String>,
    pub is_active: bool,
    pub docking: Docking,
    pub button_size: f32,
}