//! The calibration overlay, shown while the server's calibration is on: the scene billboard with
//! its grid, the projector's reach, a floor grid in 3D, and labels with the scene size and the
//! cursor's scene position. The cursor crosshair is always drawn.
//!
//! The calibration keys ([F3], [Space], arrows) are forwarded to the server like every key; the
//! server applies them and broadcasts the result.

use bevy::color::palettes::css::{DARK_GREY, ORANGE, SILVER, YELLOW};
use bevy::prelude::*;
use common::config::{ProjectorConfiguration, SceneConfiguration};
use common::scene::{SceneData, SceneSetup};
use common::state::CalibrationState;
use std::f32::consts::FRAC_PI_2;

use super::camera::{CameraSystemSet, CameraTag, DisplayMode};
use crate::ui::InstructionsAppExt;

const GRID_SPACING: f32 = 0.25;
/// [Mouse Back/Forward] moves the scene this far, in metres
const DISTANCE_STEP: f32 = 0.25;
const LABEL_FONT_SIZE: f32 = 14.0;
/// Rough glyph width, to right-align a label without measuring it
const AVG_CHAR_WIDTH: f32 = 0.6 * LABEL_FONT_SIZE;
/// Labels sit this far inside the scene's bottom corners, in metres
const LABEL_INSET: f32 = 0.25;

/// Whether the grid lines are drawn ([F4]); the frames always are
#[derive(Resource, Deref, DerefMut)]
struct GridVisible(bool);

impl Default for GridVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// Bottom-left label: the cursor's scene position
#[derive(Component)]
struct CursorLabel;

/// Bottom-right label: the scene size and distance
#[derive(Component)]
struct SceneLabel;

pub struct CalibrationPlugin;

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GridVisible>()
            .add_instructions(&[
                "Calibration modes: [Space] size -> position -> rig",
                "Press [F3] calibration on/off, [Space] next mode, [Arrows] adjust ([Shift] fine)",
                "Press [F4] to toggle background grid lines visibility",
                "Press [Mouse Back/Forward] to move scene distance further/closer",
            ])
            .add_systems(OnEnter(CalibrationState::On), spawn_labels)
            .add_systems(Update, (toggle_grid, draw_crosshair.after(CameraSystemSet)))
            .add_systems(
                Update,
                (
                    adjust_scene_distance,
                    (draw_floor_grid, draw_scene_billboard, draw_projector_billboard, place_labels).after(CameraSystemSet),
                )
                    .run_if(in_state(CalibrationState::On)),
            );
    }
}

fn toggle_grid(keys: Res<ButtonInput<KeyCode>>, mut grid: ResMut<GridVisible>) {
    if keys.just_pressed(KeyCode::F4) {
        **grid = !**grid;
    }
}

fn adjust_scene_distance(buttons: Res<ButtonInput<MouseButton>>, mut scene: ResMut<SceneConfiguration>) {
    let further = buttons.just_pressed(MouseButton::Back) as i32 as f32;
    let closer = buttons.just_pressed(MouseButton::Forward) as i32 as f32;
    let delta = (closer - further) * DISTANCE_STEP;
    if delta != 0.0 {
        // The scene lies along -Z; moving it further means more negative
        let z = scene.origin.translation.z;
        scene.origin.translation.z = z + if z <= 0.0 { delta } else { -delta };
    }
}

fn draw_floor_grid(mut gizmos: Gizmos, scene: Res<SceneConfiguration>, mode: Res<DisplayMode>, grid: Res<GridVisible>) {
    if **grid && *mode == DisplayMode::Mode3D {
        let cells = (Vec2::new(scene.scene_dimension.x, scene.origin.translation.z.abs()) / GRID_SPACING).as_uvec2();
        gizmos.grid(Quat::from_rotation_x(FRAC_PI_2), cells, Vec2::splat(GRID_SPACING), DARK_GREY);
    }
}

fn draw_scene_billboard(mut gizmos: Gizmos, scene: Res<SceneConfiguration>, grid: Res<GridVisible>) {
    let grid_color = Srgba::new(0.663, 0.663, 0.663, 0.3);
    let grid = grid.then_some((grid_color.into(), GRID_SPACING));
    draw_billboard(&mut gizmos, scene.origin.rotation, scene.origin.translation, scene.scene_dimension, SILVER.into(), grid);
}

/// The projector's reach on the scene, turned to face the camera
fn draw_projector_billboard(
    mut gizmos: Gizmos,
    setup: Res<SceneSetup>,
    projector: Res<ProjectorConfiguration>,
    camera: Query<&GlobalTransform, With<CameraTag>>,
) {
    if !projector.switched_on {
        return;
    }
    let position = setup.scene.origin.translation;
    let rotation = match camera.single() {
        Ok(camera) => {
            let level_camera = camera.translation().with_y(position.y);
            Transform::from_translation(position).looking_at(level_camera, Vec3::Y).rotation
        }
        Err(_) => projector.origin.rotation,
    };
    draw_billboard(&mut gizmos, rotation, position, setup.get_projector_view_dimensions(), ORANGE.into(), None);
}

fn draw_crosshair(mut gizmos: Gizmos, scene_data: Res<SceneData>, scene: Res<SceneConfiguration>) {
    let Some(at) = scene_data.mouse_world_pos else { return };
    let half = GRID_SPACING * 0.5;
    for axis in [Vec3::X, Vec3::Y] {
        let along = scene.origin.rotation * axis * half;
        gizmos.line(at - along, at + along, YELLOW);
    }
}

/// A rectangle of `size` centred on `position`, with grid lines every `grid.1` if given
fn draw_billboard(gizmos: &mut Gizmos, rotation: Quat, position: Vec3, size: Vec2, frame: Color, grid: Option<(Color, f32)>) {
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let half = size / 2.0;
    let corner = |x: f32, y: f32| position + right * x + up * y;

    gizmos.linestrip(
        [corner(-half.x, half.y), corner(half.x, half.y), corner(half.x, -half.y), corner(-half.x, -half.y), corner(-half.x, half.y)],
        frame,
    );

    let Some((color, spacing)) = grid else { return };
    for i in 0..=(size.x / spacing) as usize {
        let x = i as f32 * spacing - half.x;
        gizmos.line(corner(x, -half.y), corner(x, half.y), color);
    }
    for i in 0..=(size.y / spacing) as usize {
        let y = i as f32 * spacing - half.y;
        gizmos.line(corner(-half.x, y), corner(half.x, y), color);
    }
}

fn spawn_labels(mut commands: Commands) {
    let label = |text: &str| {
        (
            Text::new(text),
            TextFont { font_size: LABEL_FONT_SIZE, ..default() },
            TextColor(Color::WHITE),
            Node { position_type: PositionType::Absolute, ..default() },
            ZIndex(1000),
            Visibility::Hidden,
            DespawnOnExit(CalibrationState::On),
        )
    };
    commands.spawn((CursorLabel, label("Mouse: --")));
    commands.spawn((SceneLabel, label("Scene: --")));
}

/// Pins the labels to the scene's bottom corners on screen; hides them when a corner is off screen
fn place_labels(
    setup: Res<SceneSetup>,
    scene_data: Res<SceneData>,
    camera: Single<(&Camera, &GlobalTransform), With<CameraTag>>,
    mut cursor_label: Single<(&mut Text, &mut Node, &mut Visibility), (With<CursorLabel>, Without<SceneLabel>)>,
    mut scene_label: Single<(&mut Text, &mut Node, &mut Visibility), (With<SceneLabel>, Without<CursorLabel>)>,
) {
    let (camera, camera_transform) = *camera;
    let origin = &setup.scene.origin;
    let size = setup.scene.scene_dimension;
    let on_screen = |x: f32| {
        let corner = Vec3::new(x, -size.y / 2.0 + LABEL_INSET, 0.01);
        camera.world_to_viewport(camera_transform, origin.translation + origin.rotation * corner).ok()
    };

    let cursor_text = match scene_data.mouse_world_pos {
        Some(world) => {
            let local = origin.transform().to_matrix().inverse().transform_point3(world);
            format!("Mouse: ({:.2}, {:.2})", local.x, -local.y)
        }
        None => "Mouse: --".to_string(),
    };
    let scene_text = format!(
        "Scene: {:.2}m x {:.2}m | Dist: {:.2}m",
        size.x,
        size.y,
        origin.translation.z.abs()
    );
    let scene_text_width = scene_text.chars().count() as f32 * AVG_CHAR_WIDTH;

    let left = on_screen(-size.x / 2.0 + LABEL_INSET).map(|p| p + Vec2::new(0.0, -LABEL_FONT_SIZE));
    let right = on_screen(size.x / 2.0 - LABEL_INSET).map(|p| p - Vec2::new(scene_text_width + 6.0, LABEL_FONT_SIZE));
    place(&mut cursor_label, cursor_text, left);
    place(&mut scene_label, scene_text, right);
}

fn place(label: &mut (Mut<Text>, Mut<Node>, Mut<Visibility>), text: String, at: Option<Vec2>) {
    let (label_text, node, visibility) = label;
    label_text.0 = text;
    match at {
        Some(at) => {
            node.left = Val::Px(at.x);
            node.top = Val::Px(at.y);
            **visibility = Visibility::Visible;
        }
        None => **visibility = Visibility::Hidden,
    }
}
