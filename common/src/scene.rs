use crate::config::{
    CameraConfiguration, ProjectorConfiguration, SceneConfiguration,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::settings::ConfigAppExt;
pub struct SceneSetupPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneSystemSet;


#[derive(Resource, Debug, Default, Clone, Copy)] 
pub struct SceneData{
    pub mouse_world_pos: Option<Vec3>
}

/// Marker component for the scene entity: the billboard, the physical canvas on the wall the
/// laser draws on. Scene content (overlays, mouse crosshairs, game shapes) are its children, in
/// billboard x, y metres from its centre.
#[derive(Component)]
pub struct SceneEntity;

impl SceneData {
    pub fn new(mouse_pos: Option<Vec3>) -> Self {
        SceneData { mouse_world_pos: mouse_pos }
    }
}


impl Plugin for SceneSetupPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Startup, SceneSystemSet);
        app.configure_sets(Update, SceneSystemSet);
        app.init_config::<SceneConfiguration>();
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
        scene_configuration.clone(),
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
            scene_configuration.clone(),
            config.clone(),
            projection_config.clone(),
        );
    }
}


#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource, PartialEq)]
pub struct SceneSetup {
    /// Scene configuration including transform, dimensions, and distance.
    pub scene: SceneConfiguration,
    /// Configuration for the camera.
    pub camera: CameraConfiguration,
    /// Configuration for the projector.
    pub projector: ProjectorConfiguration,
}

impl SceneSetup {
    pub fn new(
        scene: SceneConfiguration,
        camera: CameraConfiguration,
        projector: ProjectorConfiguration,
    ) -> Self {
        SceneSetup {
            scene,
            camera,
            projector,
        }
    }
    /// Returns the width and height of the camera view at the scene distance in world units.
    /// The dimensions are calculated based on the angle and distance, ignoring the resolution aspect ratio.
    pub fn get_camera_view_dimensions(&self) -> Vec2 {
        let distance = self
            .camera
            .origin
            .translation
            .distance(self.scene.origin.translation);
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
            .origin
            .translation
            .distance(self.scene.origin.translation);
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
            .origin
            .translation
            .distance(self.scene.origin.translation);

        // Assuming camera looks down -Z
        let local_pos = Vec3::new(local_x, local_y, -distance);

        let t = &self.camera.origin;
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
            .origin
            .translation
            .distance(self.scene.origin.translation);

        let local_pos = Vec3::new(local_x, local_y, -distance);

        let t = &self.projector.origin;
        t.rotation * (local_pos * t.scale) + t.translation
    }

    /// Calculates the intersection point of the camera's forward ray (middle of view)
    /// with the plane defined by the scene's transform (assuming local Z is normal).
    /// Returns None if the ray is parallel to the plane.
    pub fn get_camera_center_on_scene_plane(&self) -> Option<Vec3> {
        let ray_origin = self.camera.origin.translation;
        let ray_dir = self.camera.origin.rotation * -Vec3::Z; // Camera looks down -Z
        let plane_origin = self.scene.origin.translation;
        let plane_normal = self.scene.origin.rotation * Vec3::Z; // Assuming scene plane is XY, so normal is Z

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
        let ray_origin = self.projector.origin.translation;
        let ray_dir = self.projector.origin.rotation * -Vec3::Z; // Projector looks down -Z

        let plane_origin = self.scene.origin.translation;
        let plane_normal = self.scene.origin.rotation * Vec3::Z; // Assuming scene plane is XY, so normal is Z

        let denominator = ray_dir.dot(plane_normal);
        if denominator.abs() < 1e-6 {
            return None;
        }

        let t = (plane_origin - ray_origin).dot(plane_normal) / denominator;
        Some(ray_origin + ray_dir * t)
    }

    /// Calculates the rotation required for the camera to look exactly at the scene's center.
    pub fn get_camera_look_at_scene_rotation(&self) -> Quat {
        let eye = self.camera.origin.translation;
        let target = self.scene.origin.translation;
        let up = Vec3::Y;
        Transform::from_translation(eye)
            .looking_at(target, up)
            .rotation
    }

    /// Calculates the rotation required for the projector to look exactly at the scene's center.
    pub fn get_projector_look_at_scene_rotation(&self) -> Quat {
        let eye = self.projector.origin.translation;
        let target = self.scene.origin.translation;
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
            translation: self.scene.origin.translation,
            rotation: self.scene.origin.rotation,
            scale: self.scene.origin.scale,
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

/// Smallest scene width or height (m)
pub const MIN_SCENE_SIZE: f32 = 0.1;
/// Closest the scene may come to the projector along Z (m)
pub const MIN_SCENE_DISTANCE: f32 = 0.5;
/// Allowed projector field of view (deg)
pub const PROJECTOR_FOV_RANGE: (f32, f32) = (5.0, 90.0);

/// Projects a world point to the projector's normalised coordinates: (-1..1, -1..1) is the full
/// DAC range (the red perimeter), +Y down as on the DAC. `None` when behind the projector.
/// This is the projection the DAC output uses.
pub fn projector_ndc(world_pos: Vec3, projector: &ProjectorConfiguration) -> Option<Vec2> {
    let projector_transform = Mat4::from_scale_rotation_translation(
        projector.origin.scale,
        projector.origin.rotation,
        projector.origin.translation,
    );
    let local = projector_transform.inverse().transform_point3(world_pos);
    // The projector looks down its local -Z
    if local.z >= 0.0 {
        return None;
    }
    let distance = -local.z;
    let tan_half_fov = (projector.angle.to_radians() / 2.0).tan();
    Some(Vec2::new(local.x / distance / tan_half_fov, -(local.y / distance / tan_half_fov)))
}

/// Physical setup upkeep. Every operation keeps the setup valid (minimum scene size and
/// distance, projector FOV range) and the scene inside the red DAC perimeter. An operation that
/// would push the scene out is refused: it returns `false` and changes nothing. A scene that is
/// already out may still move back in.
impl SceneSetup {
    /// World point on the billboard for a position in billboard units: (0, 0) is its top-left
    /// corner, (1, 1) the bottom-right.
    pub fn billboard_point(&self, uv: Vec2) -> Vec3 {
        let size = self.scene.scene_dimension;
        let local = Vec3::new((uv.x - 0.5) * size.x, (0.5 - uv.y) * size.y, 0.0);
        let origin = &self.scene.origin;
        Mat4::from_scale_rotation_translation(origin.scale, origin.rotation, origin.translation).transform_point3(local)
    }

    /// Billboard x, y (metres from the billboard centre) of a world point, projected onto the
    /// billboard plane.
    pub fn world_to_billboard(&self, world_pos: Vec3) -> Vec2 {
        let origin = &self.scene.origin;
        Mat4::from_scale_rotation_translation(origin.scale, origin.rotation, origin.translation)
            .inverse()
            .transform_point3(world_pos)
            .truncate()
    }

    /// [`projector_ndc`] for this setup's projector.
    pub fn projector_ndc(&self, world_pos: Vec3) -> Option<Vec2> {
        projector_ndc(world_pos, &self.projector)
    }

    /// The inverse of [`Self::projector_ndc`]: where the projector ray through `ndc` meets the
    /// scene plane. `None` when the ray runs parallel to or away from the plane.
    pub fn projector_ndc_to_world(&self, ndc: Vec2) -> Option<Vec3> {
        let projector = &self.projector;
        let projector_transform = Mat4::from_scale_rotation_translation(
            projector.origin.scale,
            projector.origin.rotation,
            projector.origin.translation,
        );
        let tan_half_fov = (projector.angle.to_radians() / 2.0).tan();
        // A point one unit in front of the projector on the ray, in its local space (-Z forward, +Y up)
        let on_ray = projector_transform.transform_point3(Vec3::new(ndc.x * tan_half_fov, -ndc.y * tan_half_fov, -1.0));
        let start = projector.origin.translation;
        let direction = on_ray - start;

        let plane_point = self.scene.origin.translation;
        let plane_normal = self.scene.origin.rotation * Vec3::Z;
        let denominator = plane_normal.dot(direction);
        if denominator.abs() < f32::EPSILON {
            return None;
        }
        let t = plane_normal.dot(plane_point - start) / denominator;
        (t > 0.0).then(|| start + direction * t)
    }

    /// How far the scene reaches out in projector coordinates: the largest |x| or |y| of its four
    /// corners. At most 1.0 means the whole scene is inside the red DAC perimeter; infinite when a
    /// corner is behind the projector.
    pub fn scene_extent(&self) -> f32 {
        let mut projector = self.projector.clone();
        if projector.locked_to_scene {
            // The projector re-aims at the scene centre right after any change, so judge it aimed
            projector.origin.rotation = Transform::from_translation(projector.origin.translation)
                .looking_at(self.scene.origin.translation, Vec3::Y)
                .rotation;
        }
        let origin = &self.scene.origin;
        let scene_transform = Mat4::from_scale_rotation_translation(origin.scale, origin.rotation, origin.translation);
        let half = self.scene.scene_dimension / 2.0;
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .into_iter()
            .map(|(sx, sy)| scene_transform.transform_point3(Vec3::new(sx * half.x, sy * half.y, 0.0)))
            .map(|corner| projector_ndc(corner, &projector).map_or(f32::INFINITY, |ndc| ndc.abs().max_element()))
            .fold(0.0, f32::max)
    }

    /// Grows (positive) or shrinks the scene width and height by `delta` metres.
    pub fn resize_scene(&mut self, delta: Vec2) -> bool {
        let mut next = self.clone();
        next.scene.scene_dimension = (self.scene.scene_dimension + delta).max(Vec2::splat(MIN_SCENE_SIZE));
        self.accept(next)
    }

    /// Moves the scene middle `height` metres up and `distance` metres farther from the rig,
    /// which looks down -Z.
    pub fn move_scene(&mut self, height: f32, distance: f32) -> bool {
        let mut next = self.clone();
        next.scene.origin.translation.y += height;
        next.scene.origin.translation.z = (self.scene.origin.translation.z - distance)
            .min(self.projector.origin.translation.z - MIN_SCENE_DISTANCE);
        self.accept(next)
    }

    /// Moves the rig `height` metres up: camera and projector move together.
    pub fn move_rig(&mut self, height: f32) -> bool {
        let mut next = self.clone();
        next.projector.origin.translation.y += height;
        next.camera.origin.translation.y += height;
        self.accept(next)
    }

    /// Widens (positive) or narrows the projector field of view by `delta` degrees.
    pub fn change_projector_fov(&mut self, delta: f32) -> bool {
        let mut next = self.clone();
        next.projector.angle = (self.projector.angle + delta).clamp(PROJECTOR_FOV_RANGE.0, PROJECTOR_FOV_RANGE.1);
        self.accept(next)
    }

    fn accept(&mut self, next: SceneSetup) -> bool {
        let extent = next.scene_extent();
        if extent > 1.0 && extent > self.scene_extent() {
            return false;
        }
        *self = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_changes_scene_size_only() {
        let mut setup = SceneSetup::default();
        let before = setup.clone();
        assert!(setup.resize_scene(Vec2::new(0.05, -0.01)));
        assert!(setup.scene.scene_dimension.abs_diff_eq(before.scene.scene_dimension + Vec2::new(0.05, -0.01), 1e-5));
        assert_eq!(setup.scene.origin, before.scene.origin);
        assert_eq!((&setup.projector, &setup.camera), (&before.projector, &before.camera));
    }

    #[test]
    fn scene_size_never_collapses() {
        let mut setup = SceneSetup::default();
        for _ in 0..1000 {
            setup.resize_scene(Vec2::splat(-0.05));
        }
        assert_eq!(setup.scene.scene_dimension, Vec2::splat(MIN_SCENE_SIZE));
    }

    #[test]
    fn move_scene_changes_middle_height_and_distance() {
        let mut setup = SceneSetup::default();
        let before = setup.scene.origin.translation;
        assert!(setup.move_scene(0.05, 0.05));
        let after = setup.scene.origin.translation;
        assert!((after.y - (before.y + 0.05)).abs() < 1e-5);
        // Farther from the rig, which looks down -Z
        assert!((after.z - (before.z - 0.05)).abs() < 1e-5);
        assert_eq!(setup.projector, ProjectorConfiguration::default());
    }

    #[test]
    fn scene_cannot_move_through_the_rig() {
        let mut setup = SceneSetup::default();
        // Small and level with the projector, so the red perimeter does not stop it first
        setup.scene.scene_dimension = Vec2::splat(0.2);
        setup.scene.origin.translation.y = setup.projector.origin.translation.y;
        for _ in 0..1000 {
            setup.move_scene(0.0, -0.05);
        }
        let distance = setup.projector.origin.translation.z - setup.scene.origin.translation.z;
        assert!((distance - MIN_SCENE_DISTANCE).abs() < 1e-5, "distance {}", distance);
    }

    #[test]
    fn rig_moves_camera_and_projector_together() {
        let mut setup = SceneSetup::default();
        let before = setup.clone();
        assert!(setup.move_rig(-0.05));
        let down = Vec3::new(0.0, -0.05, 0.0);
        assert!((setup.projector.origin.translation - before.projector.origin.translation).abs_diff_eq(down, 1e-5));
        assert!((setup.camera.origin.translation - before.camera.origin.translation).abs_diff_eq(down, 1e-5));
        assert_eq!(setup.scene, before.scene);
    }

    #[test]
    fn projector_fov_stays_in_range() {
        let mut setup = SceneSetup::default();
        for _ in 0..1000 {
            setup.change_projector_fov(1.0);
        }
        assert_eq!(setup.projector.angle, PROJECTOR_FOV_RANGE.1);
    }

    #[test]
    fn scene_stops_growing_at_red_perimeter() {
        let mut setup = SceneSetup::default();
        assert!(setup.scene_extent() <= 1.0, "default scene starts inside");
        let mut steps = 0;
        while setup.resize_scene(Vec2::new(0.05, 0.0)) {
            steps += 1;
            assert!(steps < 10_000);
        }
        let extent = setup.scene_extent();
        assert!(extent <= 1.0 && extent > 0.98, "extent {}", extent);
        // A refused operation changes nothing; shrinking still works
        let before = setup.clone();
        assert!(!setup.resize_scene(Vec2::new(0.05, 0.0)));
        assert_eq!(setup, before);
        assert!(setup.resize_scene(Vec2::new(-0.05, 0.0)));
    }

    #[test]
    fn narrowing_fov_moving_closer_or_lowering_rig_cannot_push_scene_out() {
        let operations: [fn(&mut SceneSetup) -> bool; 3] = [
            |s| s.change_projector_fov(-0.5),
            |s| s.move_scene(0.0, -0.05),
            |s| s.move_rig(-0.05),
        ];
        for (i, operation) in operations.iter().enumerate() {
            let mut setup = SceneSetup::default();
            for _ in 0..2000 {
                operation(&mut setup);
            }
            assert!(setup.scene_extent() <= 1.0, "operation {} pushed the scene out", i);
        }
    }

    #[test]
    fn scene_already_outside_may_move_back_in() {
        let mut setup = SceneSetup::default();
        setup.scene.scene_dimension = Vec2::new(30.0, 6.0);
        assert!(setup.scene_extent() > 1.0);
        assert!(!setup.resize_scene(Vec2::new(0.05, 0.0)));
        assert!(setup.resize_scene(Vec2::new(-0.05, 0.0)));
    }

    #[test]
    fn scene_point_maps_box_corners() {
        let setup = SceneSetup::default();
        let center = setup.scene.origin.translation;
        let half = setup.scene.scene_dimension / 2.0;
        assert!(setup.billboard_point(Vec2::splat(0.5)).abs_diff_eq(center, 1e-5));
        assert!(setup.billboard_point(Vec2::ZERO).abs_diff_eq(center + Vec3::new(-half.x, half.y, 0.0), 1e-5));
        assert!(setup.billboard_point(Vec2::ONE).abs_diff_eq(center + Vec3::new(half.x, -half.y, 0.0), 1e-5));
    }

    #[test]
    fn world_to_scene_drops_distance_from_plane() {
        let setup = SceneSetup::default();
        let center = setup.scene.origin.translation;
        assert!(setup.world_to_billboard(center + Vec3::new(1.0, -0.5, 0.0)).abs_diff_eq(Vec2::new(1.0, -0.5), 1e-5));
        // A point in front of the plane lands on it
        assert!(setup.world_to_billboard(center + Vec3::new(1.0, -0.5, 3.0)).abs_diff_eq(Vec2::new(1.0, -0.5), 1e-5));
    }

    #[test]
    fn projector_ndc_edges() {
        let projector = ProjectorConfiguration {
            angle: 90.0,
            origin: crate::config::ConfigTransform::default(),
            ..Default::default()
        };
        // 90 deg FOV: the edge is at 45 deg, so x = distance
        assert!(projector_ndc(Vec3::new(10.0, 0.0, -10.0), &projector).unwrap().abs_diff_eq(Vec2::new(1.0, 0.0), 1e-5));
        // World up is DAC -Y
        assert!(projector_ndc(Vec3::new(0.0, 5.0, -10.0), &projector).unwrap().abs_diff_eq(Vec2::new(0.0, -0.5), 1e-5));
        assert_eq!(projector_ndc(Vec3::new(0.0, 0.0, 1.0), &projector), None);
    }

    #[test]
    fn projector_ndc_to_world_inverts_projector_ndc_on_the_scene_plane() {
        let setup = SceneSetup::default();
        for uv in [Vec2::splat(0.5), Vec2::ZERO, Vec2::ONE, Vec2::new(0.2, 0.7)] {
            let world = setup.billboard_point(uv);
            let ndc = setup.projector_ndc(world).unwrap();
            let back = setup.projector_ndc_to_world(ndc).unwrap();
            assert!(back.abs_diff_eq(world, 1e-3), "{uv}: {world} -> {ndc} -> {back}");
        }
    }
}
