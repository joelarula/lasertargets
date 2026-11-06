//! The scene entity, kept at the scene's pose, and the scene point under the mouse cursor

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use common::scene::{SceneData, SceneEntity, SceneSetup, SceneSystemSet};

use super::camera::CameraTag;

/// The cursor counts as over the scene this far beyond its edges, in scene widths/heights
const CURSOR_REACH: f32 = 1.5;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneData>()
            .add_systems(Startup, spawn_scene.in_set(SceneSystemSet))
            .add_systems(
                Update,
                (track_cursor, follow_scene_setup.run_if(resource_changed::<SceneSetup>)).in_set(SceneSystemSet),
            );
    }
}

fn spawn_scene(mut commands: Commands, setup: Res<SceneSetup>) {
    commands.spawn((SceneEntity, setup.scene.origin.transform(), Visibility::default()));
}

fn follow_scene_setup(setup: Res<SceneSetup>, mut scene: Single<&mut Transform, With<SceneEntity>>) {
    **scene = setup.scene.origin.transform();
}

/// Casts the cursor onto the scene plane (facing the camera) and keeps the hit if near the scene
fn track_cursor(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<CameraTag>>,
    setup: Res<SceneSetup>,
    mut scene_data: ResMut<SceneData>,
) {
    let (camera, camera_transform) = *camera;
    let scene = GlobalTransform::from(setup.scene.origin.transform());
    let size = setup.scene.scene_dimension;

    scene_data.mouse_world_pos = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .and_then(|ray| {
            let normal = Dir3::new(camera_transform.translation() - scene.translation()).unwrap_or(Dir3::Z);
            let distance = ray.intersect_plane(scene.translation(), InfinitePlane3d::new(normal))?;
            Some(ray.get_point(distance))
        })
        .filter(|hit| {
            let local = scene.affine().inverse().transform_point(*hit);
            local.x.abs() <= size.x * CURSOR_REACH && local.y.abs() <= size.y * CURSOR_REACH
        });
}
