use crate::config::{
    CameraConfiguration, ConfigTransform, ProjectorConfiguration, SceneConfiguration,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
pub struct SceneSetupPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneSystemSet;

impl Plugin for SceneSetupPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Startup, SceneSystemSet);
        app.configure_sets(Update, SceneSystemSet);
        app.insert_resource(SceneConfiguration::default());
        app.init_resource::<SceneSetup>(); // Initialize SceneSetup as a resource
        app.add_systems(Startup, initialize_scene_setup_resource);
        app.add_systems(Update, update_scene_setup_resource);
    }
}


fn initialize_scene_setup_resource(
    mut scene_setup: ResMut<SceneSetup>,
    config: Res<CameraConfiguration>,
    projection_config: Res<ProjectorConfiguration>,
    scene_configuration: Res<SceneConfiguration>,
) {
    *scene_setup = SceneSetup::new(
        scene_configuration.transform.clone(),
        config.clone(),
        projection_config.clone(),
    );
}

fn update_scene_setup_resource(
    mut scene_setup: ResMut<SceneSetup>,
    config: Res<CameraConfiguration>,
    scene_configuration: Res<SceneConfiguration>,
    projection_config: Res<ProjectorConfiguration>,
) {
    if config.is_changed() || scene_configuration.is_changed() || projection_config.is_changed() {
        *scene_setup = SceneSetup::new(
            scene_configuration.transform.clone(),
            config.clone(),
            projection_config.clone(),
        );
    }
}


#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct SceneSetup {
    /// Transform of the scene in world space.
    pub transform: ConfigTransform,
    /// Configuration for the camera.
    pub camera: CameraConfiguration,
    /// Configuration for the projector.
    pub projector: ProjectorConfiguration,
}

impl SceneSetup {
    pub fn new(
        transform: ConfigTransform,
        camera: CameraConfiguration,
        projector: ProjectorConfiguration,
    ) -> Self {
        SceneSetup {
            transform,
            camera,
            projector,
        }
    }
    /// Returns the width and height of the camera view at the scene distance in world units.
    /// The dimensions are calculated based on the angle and distance, ignoring the resolution aspect ratio.
    pub fn get_camera_view_dimensions(&self) -> Vec2 {
        let distance = self
            .camera
            .transform
            .translation
            .distance(self.transform.translation);
        let half_angle_rad = self.camera.angle.to_radians() / 2.0;
        let width = 2.0 * distance * half_angle_rad.tan();
        let height = width;
        Vec2::new(width, height)
    }

    /// Returns the width and height of the projector view at the scene distance in world units.
    /// The dimensions are calculated based on the angle and distance, ignoring the resolution aspect ratio.
    pub fn get_projector_view_dimensions(&self) -> Vec2 {
        let distance = self
            .projector
            .transform
            .translation
            .distance(self.transform.translation);
        let half_angle_rad = self.projector.angle.to_radians() / 2.0;
        let width = 2.0 * distance * half_angle_rad.tan();
        let height = width;
        Vec2::new(width, height)
    }

    /// Translates a pixel coordinate in the camera's resolution to a world space coordinate on the target plane.
    /// Assumes pixel (0,0) is top-left.
    pub fn get_camera_pixel_to_world(&self, pixel_pos: Vec2) -> Vec3 {
        let view_dims = self.get_camera_view_dimensions();
        let resolution = self.camera.resolution.as_vec2();

        // Normalize coordinates to [-0.5, 0.5] range, centering (0,0).
        // Pixel coordinates: (0,0) top-left, +X right, +Y down.
        // Local coordinates: (0,0) center, +X right, +Y up.
        let norm_x = (pixel_pos.x / resolution.x) - 0.5;
        let norm_y = 0.5 - (pixel_pos.y / resolution.y);

        let local_x = norm_x * view_dims.x;
        let local_y = norm_y * view_dims.y;

        let distance = self
            .camera
            .transform
            .translation
            .distance(self.transform.translation);

        // Assuming camera looks down -Z
        let local_pos = Vec3::new(local_x, local_y, -distance);

        let t = &self.camera.transform;
        t.rotation * (local_pos * t.scale) + t.translation
    }

    /// Translates a pixel coordinate in the projector's resolution to a world space coordinate on the target plane.
    /// Assumes pixel (0,0) is top-left.
    pub fn get_projector_pixel_to_world(&self, pixel_pos: Vec2) -> Vec3 {
        let view_dims = self.get_projector_view_dimensions();
        let resolution = self.projector.resolution.as_vec2();

        let norm_x = (pixel_pos.x / resolution.x) - 0.5;
        let norm_y = 0.5 - (pixel_pos.y / resolution.y);

        let local_x = norm_x * view_dims.x;
        let local_y = norm_y * view_dims.y;

        let distance = self
            .projector
            .transform
            .translation
            .distance(self.transform.translation);

        let local_pos = Vec3::new(local_x, local_y, -distance);

        let t = &self.projector.transform;
        t.rotation * (local_pos * t.scale) + t.translation
    }

    /// Calculates the intersection point of the camera's forward ray (middle of view)
    /// with the plane defined by the scene's transform (assuming local Z is normal).
    /// Returns None if the ray is parallel to the plane.
    pub fn get_camera_center_on_scene_plane(&self) -> Option<Vec3> {
        let ray_origin = self.camera.transform.translation;
        let ray_dir = self.camera.transform.rotation * -Vec3::Z; // Camera looks down -Z

        let plane_origin = self.transform.translation;
        let plane_normal = self.transform.rotation * Vec3::Z; // Assuming scene plane is XY, so normal is Z

        let denominator = ray_dir.dot(plane_normal);
        if denominator.abs() < 1e-6 {
            return None;
        }

        let t = (plane_origin - ray_origin).dot(plane_normal) / denominator;
        Some(ray_origin + ray_dir * t)
    }

    /// Calculates the intersection point of the projector's forward ray (middle of view)
    /// with the plane defined by the scene's transform (assuming local Z is normal).
    /// Returns None if the ray is parallel to the plane.
    pub fn get_projector_center_on_scene_plane(&self) -> Option<Vec3> {
        let ray_origin = self.projector.transform.translation;
        let ray_dir = self.projector.transform.rotation * -Vec3::Z; // Projector looks down -Z

        let plane_origin = self.transform.translation;
        let plane_normal = self.transform.rotation * Vec3::Z; // Assuming scene plane is XY, so normal is Z

        let denominator = ray_dir.dot(plane_normal);
        if denominator.abs() < 1e-6 {
            return None;
        }

        let t = (plane_origin - ray_origin).dot(plane_normal) / denominator;
        Some(ray_origin + ray_dir * t)
    }

    /// Calculates the rotation required for the camera to look exactly at the scene's center.
    pub fn get_camera_look_at_scene_rotation(&self) -> Quat {
        let eye = self.camera.transform.translation;
        let target = self.transform.translation;
        let up = Vec3::Y;
        Transform::from_translation(eye)
            .looking_at(target, up)
            .rotation
    }

    /// Calculates the rotation required for the projector to look exactly at the scene's center.
    pub fn get_projector_look_at_scene_rotation(&self) -> Quat {
        let eye = self.projector.transform.translation;
        let target = self.transform.translation;
        let up = Vec3::Y;
        Transform::from_translation(eye)
            .looking_at(target, up)
            .rotation
    }

    /// Calculates the shared viewport dimensions (width, height) and center point in world space
    /// where the camera and projector views overlap on the scene plane.
    pub fn get_common_viewport_stats(&self) -> Option<(f32, f32, Vec3)> {
        let camera_center = self.get_camera_center_on_scene_plane()?;
        let projector_center = self.get_projector_center_on_scene_plane()?;

        let transform = Transform {
            translation: self.transform.translation,
            rotation: self.transform.rotation,
            scale: self.transform.scale,
        };

        let to_local = transform.compute_affine().inverse();
        let camera_local = to_local.transform_point3(camera_center).xy();
        let projector_local = to_local.transform_point3(projector_center).xy();

        let camera_size = self.get_camera_view_dimensions();
        let projector_size = self.get_projector_view_dimensions();

        let c_min = camera_local - camera_size / 2.0;
        let c_max = camera_local + camera_size / 2.0;

        let p_min = projector_local - projector_size / 2.0;
        let p_max = projector_local + projector_size / 2.0;

        let min = c_min.max(p_min);
        let max = c_max.min(p_max);

        if min.x >= max.x || min.y >= max.y {
            return None;
        }

        let size = max - min;
        let center_local = min + size / 2.0;
        let center_world = transform.transform_point(center_local.extend(0.0));

        Some((size.x, size.y, center_world))
    }
}

