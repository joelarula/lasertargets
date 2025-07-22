use bevy::prelude::*;

/// Stores global configuration state for the application.
#[derive(Resource, Default)]
pub struct ConfigState {
    /// Controls whether the on-screen instructions are visible.
    pub instructions_visible: bool,
    /// Defines the grid spacing in modeled physical world in meters.
    pub grid_spacing: f32,
    // Defines the distance of a target detection plane in modeled physical world in meters.
    pub target_projection_distance: f32,
    /// Defines the size of the thermal camera viewport in pixels.
    pub termocamera_size: UVec2,
    /// Defines the orign of the thermal camera viewport in world unit.
    pub termocamera_origin: Vec3,
    /// Defines the position where the thermal camera is looking at in world unit.
    pub termocamera_looking_at: Vec3,
}
pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ConfigState{
            instructions_visible: true,
            grid_spacing: 0.25,
            target_projection_distance: 25., 
            termocamera_origin: Vec3::new(0., 1.5,5.),
            termocamera_looking_at: Vec3::new(0., 1.5, 0.),
            termocamera_size: UVec2::new(256, 192),
        });
}} 