use bevy::{
    asset::uuid::Uuid,
    ecs::resource::Resource,
    math::{Quat, UVec2, Vec3},
    transform::components::Transform,
};
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for ConfigTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct SceneConfiguration {
    /// Defines the distance of a target detection plane in modeled physical world in meters.
    pub target_projection_distance: f32,
    /// Defines the desired width of the scene in meters.
    pub scene_width: f32,
    /// Defines the position and orientation of the scene in world space.
    pub transform: ConfigTransform,
}

impl Default for SceneConfiguration {
    fn default() -> Self {
        let target_projection_distance = 25.0;
        let translation = Vec3::new(0.0, 0.0, -target_projection_distance);

        Self {
            target_projection_distance,
            scene_width: 10.0,
            transform: ConfigTransform {
                translation,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct ProjectorConfiguration {
    pub resolution: UVec2,
    // projection angle in degrees
    pub angle: f32,
    pub transform: ConfigTransform,
    pub enabled: bool,
    pub locked_to_scene: bool,
}

impl Default for ProjectorConfiguration {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(800, 800),
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
    pub resolution: UVec2,
    /// Defines the camera's position and orientation in world space.
    pub transform: ConfigTransform,
    // camera field of view angle in degrees
    pub angle: f32,
    // lock camera center to scene transform
    pub locked_to_scene: bool,
}

impl Default for CameraConfiguration {
    fn default() -> Self {
        Self {
            resolution: UVec2::new(256, 192),
            transform: ConfigTransform {
                translation: Vec3::new(0.0, 1.5, 5.0),
                rotation: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0))
                    .looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y)
                    .rotation,
                scale: Vec3::ONE,
            },
            angle: 45.0,
            locked_to_scene: false,
        }
    }
}
