use bevy::prelude::*;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
pub enum AppState {
    #[default]
    Calibration,
}

/// Stores global configuration state for the application.
#[derive(Resource, Default)]
pub struct ConfigState {
    /// Controls whether the on-screen instructions are visible.
    pub instructions_visible: bool,

    /// Defines the size of the thermal camera viewport in pixels.
    pub camera_input_size: UVec2,
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
            termocamera_origin: Vec3::new(0., 1.5,5.),
            termocamera_looking_at: Vec3::new(0., 1.5, 0.),
            camera_input_size: UVec2::new(256, 192)
        });
}} 