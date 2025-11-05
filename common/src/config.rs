use bevy::ecs::resource::Resource;
use bevy::prelude::States;



/// Stores global configuration state for the application.
#[derive(Resource)]
pub struct SceneConfiguration {
    /// Defines the distance of a target detection plane in modeled physical world in meters.
    pub target_projection_distance: f32,
    /// Defines the width of the scene in meters.
    pub scene_width: f32,
}

impl Default for SceneConfiguration {
    fn default() -> Self {
        Self {
            target_projection_distance: 25.0,
            scene_width: 10.0,
        }
    }
}