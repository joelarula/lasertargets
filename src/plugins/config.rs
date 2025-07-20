use bevy::prelude::*;

/// Stores global configuration state for the application.
#[derive(Resource, Default)]
pub struct ConfigState {
    /// Controls whether the on-screen instructions are visible.
    pub instructions_visible: bool,
    /// Defines the grid spacing in modeled physical world in millimeters.
    pub grid_spacing: f32,
    // Defines the distance of a target detection plane in modeled physical world in millimeters.
    // pub target_projection_distance: f32 
    /// Defines the size of the thermal camera viewport in pixels.
    pub termocam_size: UVec2,
}
pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ConfigState{
            instructions_visible: true,
            grid_spacing: 250.0,
            termocam_size: UVec2::new(800, 600),
           // target_projection_distance: 25000.0
        });
}}