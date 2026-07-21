use bevy::prelude::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::scene::{SceneEntity, SceneSetup, SceneSystemSet};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_scene_entity.in_set(SceneSystemSet))
            .add_systems(Update, (sync_scene_transform, log_scene_change_report).in_set(SceneSystemSet));
    }
}

fn setup_scene_entity(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
) {
    // Spawn the scene entity with its initial transform
    commands.spawn((
        SceneEntity,
        Transform::from_translation(scene_setup.scene.origin.translation)
            .with_rotation(scene_setup.scene.origin.rotation)
            .with_scale(scene_setup.scene.origin.scale),
        Visibility::default(),
    ));

    // Log initial scene report on startup
    log_readable_scene_report(&scene_setup);
}

fn sync_scene_transform(
    scene_setup: Res<SceneSetup>,
    mut scene_query: Query<&mut Transform, With<SceneEntity>>,
) {
    if scene_setup.is_changed() {
        if let Ok(mut transform) = scene_query.single_mut() {
            // Update the scene entity's transform to match SceneSetup
            transform.translation = scene_setup.scene.origin.translation;
            transform.rotation = scene_setup.scene.origin.rotation;
            transform.scale = scene_setup.scene.origin.scale;
        }
    }
}

/// System to log scene configuration changes automatically whenever any scene parameter changes
fn log_scene_change_report(
    scene_setup: Res<SceneSetup>,
    camera_config: Res<CameraConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
    scene_config: Res<SceneConfiguration>,
) {
    if scene_setup.is_changed() || camera_config.is_changed() || projector_config.is_changed() || scene_config.is_changed() {
        log_readable_scene_report(&scene_setup);
    }
}

/// Formats and logs a clear, human-readable report of scene dimensions, center position, 3D distances, and bounds.
pub fn log_readable_scene_report(setup: &SceneSetup) {
    let dim = setup.scene.scene_dimension;
    let half_w = dim.x / 2.0;
    let half_h = dim.y / 2.0;
    let origin = setup.scene.origin.translation;

    let min_x = origin.x - half_w;
    let max_x = origin.x + half_w;
    let min_y = origin.y - half_h;
    let max_y = origin.y + half_h;

    let area = dim.x * dim.y;

    let cam_pos = setup.camera.origin.translation;
    let prj_pos = setup.projector.origin.translation;

    let cam_dist = cam_pos.distance(origin);
    let prj_dist = prj_pos.distance(origin);

    info!("╔══════════════════════════════════════════════════════════════════════════════╗");
    info!("║                       SCENE CONFIGURATION CHANGE REPORT                      ║");
    info!("╠══════════════════════════════════════════════════════════════════════════════╣");
    info!("  • Dimensions         : {:.2} m × {:.2} m  (Area: {:.2} m²)", dim.x, dim.y, area);
    info!("  • Scene Center Pos   : (X: {:.2} m, Y: {:.2} m, Z: {:.2} m)", origin.x, origin.y, origin.z);
    info!("  • Horizontal Bounds  : X ∈ [{:+.2} m, {:+.2} m]", min_x, max_x);
    info!("  • Vertical Bounds    : Y ∈ [{:+.2} m, {:+.2} m]", min_y, max_y);
    info!("  • Y Difference       : {:.2} m", setup.scene.y_difference);
    info!("  • Camera Distance    : {:.2} m  (Cam Pos: ({:.2}, {:.2}, {:.2}), Angle: {:.1}°)", cam_dist, cam_pos.x, cam_pos.y, cam_pos.z, setup.camera.angle);
    info!("  • Projector Distance : {:.2} m  (Prj Pos: ({:.2}, {:.2}, {:.2}), Angle: {:.1}°)", prj_dist, prj_pos.x, prj_pos.y, prj_pos.z, setup.projector.angle);
    info!("╚══════════════════════════════════════════════════════════════════════════════╝");
}