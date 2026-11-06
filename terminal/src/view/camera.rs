//! The 3D camera: a perspective view from the configured camera position, or with [F2] a
//! straight-on orthographic view of the scene

use bevy::prelude::*;
use common::config::{CameraConfiguration, SceneConfiguration};
use common::scene::{SceneSetup, SceneSystemSet};

use crate::ui::InstructionsAppExt;

/// Margin around the scene in the 2D view, in metres
const ORTHO_PADDING: f32 = 50.0;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayMode {
    Mode2D,
    #[default]
    Mode3D,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CameraSystemSet;

#[derive(Component)]
pub struct CameraTag;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayMode>()
            .add_instructions(&["Press [F2] to toggle between 2d and 3d display mode"])
            .add_systems(Startup, spawn_camera.in_set(CameraSystemSet).after(SceneSystemSet))
            .add_systems(Update, (toggle_display_mode, aim_camera).chain().in_set(CameraSystemSet).after(SceneSystemSet));
    }
}

fn spawn_camera(
    mut commands: Commands,
    setup: Res<SceneSetup>,
    scene: Res<SceneConfiguration>,
    camera: Res<CameraConfiguration>,
    mode: Res<DisplayMode>,
) {
    commands.spawn((DirectionalLight::default(), Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        CameraTag,
        Camera3d::default(),
        Camera { order: 1, ..default() },
        projection(*mode, setup.camera.angle, &scene),
        Transform::from_translation(camera.origin.translation).looking_at(setup.scene.origin.translation, Vec3::Y),
    ));
}

fn toggle_display_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<DisplayMode>) {
    if keys.just_pressed(KeyCode::F2) {
        *mode = match *mode {
            DisplayMode::Mode2D => DisplayMode::Mode3D,
            DisplayMode::Mode3D => DisplayMode::Mode2D,
        };
    }
}

fn aim_camera(
    camera: Single<(&mut Projection, &mut Transform), With<CameraTag>>,
    setup: Res<SceneSetup>,
    scene: Res<SceneConfiguration>,
    config: Res<CameraConfiguration>,
    mode: Res<DisplayMode>,
) {
    let (mut camera_projection, mut transform) = camera.into_inner();
    let target = setup.scene.origin.translation;
    *transform = match *mode {
        // Straight down the Z axis at the scene centre, keeping the camera's distance
        DisplayMode::Mode2D => Transform::from_xyz(target.x, target.y, transform.translation.z),
        DisplayMode::Mode3D => Transform::from_translation(config.origin.translation),
    }
    .looking_at(target, Vec3::Y);

    if mode.is_changed() || config.is_changed() || setup.is_changed() || scene.is_changed() {
        *camera_projection = projection(*mode, setup.camera.angle, &scene);
    }
}

fn projection(mode: DisplayMode, fov_degrees: f32, scene: &SceneConfiguration) -> Projection {
    match mode {
        DisplayMode::Mode2D => Projection::from(OrthographicProjection {
            area: Rect::from_center_half_size(Vec2::ZERO, (scene.scene_dimension + ORTHO_PADDING) / 2.0),
            ..OrthographicProjection::default_3d()
        }),
        DisplayMode::Mode3D => Projection::from(PerspectiveProjection {
            fov: fov_degrees.to_radians(),
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
    }
}
