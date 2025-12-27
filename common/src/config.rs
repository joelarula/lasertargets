use bevy::{
    ecs::resource::Resource,
    math::{Quat, Vec3},
};
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
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

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ProjectorConfiguration {
    pub output_resolution: bevy::prelude::UVec2,
    // projection angle in degrees
    pub angle: f32,
    pub transform: ConfigTransform,
    pub enabled: bool,
    pub locked_to_scene: bool,
}

impl Default for ProjectorConfiguration {
    fn default() -> Self {
        Self {
            output_resolution: bevy::prelude::UVec2::new(800, 800),
            angle: 25.0,
            transform: ConfigTransform {
                translation: bevy::prelude::Vec3::new(0.0, 1.5, 5.0),
                rotation: bevy::prelude::Transform::from_translation(bevy::prelude::Vec3::new(
                    0.0, 1.5, 5.0,
                ))
                .looking_at(
                    bevy::prelude::Vec3::new(0.0, 1.5, 0.0),
                    bevy::prelude::Vec3::Y,
                )
                .rotation,
                scale: bevy::prelude::Vec3::ONE,
            },
            enabled: false,
            locked_to_scene: false,
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfiguration {
    /// Defines the size of the thermal camera viewport in pixels.
    pub input_resolution: bevy::prelude::UVec2,
    /// Defines the camera's position and orientation in world space.
    pub transform: ConfigTransform,
}

impl Default for CameraConfiguration {
    fn default() -> Self {
        Self {
            input_resolution: bevy::prelude::UVec2::new(256, 192),
            transform: ConfigTransform {
                translation: bevy::prelude::Vec3::new(0.0, 1.5, 5.0),
                rotation: bevy::prelude::Transform::from_translation(bevy::prelude::Vec3::new(
                    0.0, 1.5, 5.0,
                ))
                .looking_at(
                    bevy::prelude::Vec3::new(0.0, 1.5, 0.0),
                    bevy::prelude::Vec3::Y,
                )
                .rotation,
                scale: bevy::prelude::Vec3::ONE,
            },
        }
    }
}
