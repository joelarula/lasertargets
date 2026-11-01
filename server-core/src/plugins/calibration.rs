use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::MousePositionEvent;
use common::path::{PathSegment, UniversalPath};
use common::scene::SceneEntity;
use common::scene::{SceneSetup, SceneSystemSet};
use common::state::CalibrationState;
use gamepad::{Btn, GamepadButtonEvent, GamepadState, PrevGamepadState, GAMEPAD_STICK_DEADZONE};

/// Target entity currently being calibrated by the gamepad in calibration mode.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationTargetMode {
    /// Mode A: Physical Rig Pose (Laser Projector & Camera at Point A looking at Billboard)
    #[default]
    RigPointA,
    /// Mode B: Physical Billboard Plane (Laser reflection canvas on the wall at Point B)
    BillboardPointB,
}

#[derive(Resource)]
pub struct CalibrationGamepadCooldowns {
    pub calibration_toggle: Timer,
    pub calibration_step: Timer,
    pub target_mode_toggle: Timer,
}

impl Default for CalibrationGamepadCooldowns {
    fn default() -> Self {
        Self {
            calibration_toggle: Timer::from_seconds(0.5, TimerMode::Once),
            calibration_step: Timer::from_seconds(0.15, TimerMode::Once),
            target_mode_toggle: Timer::from_seconds(0.3, TimerMode::Once),
        }
    }
}

pub struct CalibrationPlugin;

/// Resource to track calibration data (server singleton)
#[derive(Resource)]
pub struct CalibrationData {
    pub mouse_positions: std::collections::HashMap<u64, Vec3>, // Per-client mouse tracking
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            mouse_positions: std::collections::HashMap::new(),
        }
    }
}

/// Component to mark calibration crosshair entities
#[derive(Component)]
pub struct CalibrationCrosshair {
    pub client_id: u64, // Track which client this crosshair belongs to
}

/// Component to mark projection area rectangle entities  
#[derive(Component)]
pub struct ProjectionAreaRectangle;

/// Component to mark center crosshair entity
#[derive(Component)]
pub struct CalibrationCenterCrosshair;

/// Component to mark calibration-only paths (not broadcast to terminals)
#[derive(Component)]
pub struct CalibrationPath;

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CalibrationData>()
            .init_resource::<CalibrationGamepadCooldowns>()
            .init_resource::<CalibrationTargetMode>()
            .add_systems(OnExit(CalibrationState::On), despawn_calibration_overlays.after(SceneSystemSet))
            .add_systems(Update, handle_mouse_position_updates)
            .add_systems(Update, cleanup_disconnected_clients)
            .add_systems(Update, (
                spawn_calibration_overlays,
                update_and_maintain_crosshair_positions,
            ).after(SceneSystemSet).run_if(in_state(CalibrationState::On)))
            .add_systems(Update, update_projection_area_rectangle.run_if(in_state(CalibrationState::On)))
            .add_systems(Update, gamepad_toggle_calibration)
            .add_systems(Update, gamepad_calibration_controls.run_if(in_state(CalibrationState::On)));
    }
}


fn despawn_calibration_overlays(
    mut commands: Commands,
    path_query: Query<Entity, With<CalibrationPath>>,
) {
    info!("Exiting CalibrationState::On");
    for entity in path_query.iter() {
        if let Ok(mut entity_cmds) = commands.get_entity(entity) {
            entity_cmds.despawn();
        }
    }
}



fn handle_mouse_position_updates(
    mut mouse_events: MessageReader<MousePositionEvent>,
    mut calibration_data: ResMut<CalibrationData>,
) {
    // Always track mouse positions from all clients
    for event in mouse_events.read() {
        if let Some(world_pos) = event.position {
            calibration_data.mouse_positions.insert(event.client_id, world_pos);
        } else {
            calibration_data.mouse_positions.remove(&event.client_id);
        }
    }
}

fn build_calibration_crosshair_path() -> UniversalPath {
    let crosshair_size = 0.4;
    let cyan = Color::srgb(0.0, 0.9, 1.0);
    UniversalPath::crosshair(Vec2::ZERO, crosshair_size, cyan)
}

#[inline]
fn world_to_local(world_pos: Vec3, scene_inv_matrix: Option<&Mat4>) -> Vec3 {
    if let Some(inv_mat) = scene_inv_matrix {
        inv_mat.transform_point3(world_pos)
    } else {
        world_pos
    }
}

fn update_and_maintain_crosshair_positions(
    mut commands: Commands,
    calibration_data: Res<CalibrationData>,
    scene_query: Query<(Entity, &Transform), (With<SceneEntity>, Without<CalibrationCrosshair>)>,
    mut crosshair_query: Query<(Entity, &mut Transform, &CalibrationCrosshair)>,
) {
    let (scene_entity, scene_inv_matrix): (Option<Entity>, Option<Mat4>) = match scene_query.single() {
        Ok((entity, transform)) => (Some(entity), Some(transform.to_matrix().inverse())),
        Err(_) => (None, None),
    };

    let mut existing_clients: std::collections::HashMap<u64, Entity> = std::collections::HashMap::new();

    for (entity, mut transform, crosshair) in crosshair_query.iter_mut() {
        if let Some(&world_pos) = calibration_data.mouse_positions.get(&crosshair.client_id) {
            transform.translation = world_to_local(world_pos, scene_inv_matrix.as_ref());
            existing_clients.insert(crosshair.client_id, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Auto-respawn crosshair if client is actively tracking but entity was despawned
    for (&client_id, &world_pos) in calibration_data.mouse_positions.iter() {
        if !existing_clients.contains_key(&client_id) {
            spawn_crosshair_at_position(&mut commands, client_id, world_pos, scene_entity, scene_inv_matrix.as_ref());
            info!("✓ Auto-restored active crosshair for client {} at {:?}", client_id, world_pos);
        }
    }
}

fn cleanup_disconnected_clients(
    mut connection_lost_events: MessageReader<ConnectionLostEvent>,
    mut calibration_data: ResMut<CalibrationData>,
) {
    for connection_lost in connection_lost_events.read() {
        let client_id = connection_lost.id;
        calibration_data.mouse_positions.remove(&client_id);
        info!("Cleaned up mouse tracking for disconnected client {}", client_id);
    }
}

/// Spawn crosshair at specific position, parented to SceneEntity if available
fn spawn_crosshair_at_position(
    commands: &mut Commands,
    client_id: u64,
    world_position: Vec3,
    scene_entity: Option<Entity>,
    scene_inv_matrix: Option<&Mat4>,
) {
    let crosshair_universal_path = build_calibration_crosshair_path();
    let local_pos = world_to_local(world_position, scene_inv_matrix);

    let child_entity = commands.spawn((
        Transform::from_translation(local_pos),
        Visibility::default(),
        CalibrationPath,
        CalibrationCrosshair { client_id },
        crosshair_universal_path,
        common::path::PathRenderable::default(),
    )).id();

    if let Some(scene_e) = scene_entity {
        commands.entity(scene_e).add_child(child_entity);
    }
    
    info!("Spawned mouse crosshair for client {} in scene at local position {:?}", client_id, local_pos);
}

/// Build calibration universal path containing four L-shaped corner bracket markers with high corner dwells (dwell = 14) for 100% sharp 90-degree corners
fn build_calibration_rectangle_path(scene_dimensions: Vec2) -> UniversalPath {
    let half_w = scene_dimensions.x / 2.0;
    let half_h = scene_dimensions.y / 2.0;
    let green = Color::srgb(0.0, 1.0, 0.0);
    let blank = Color::srgb(0.0, 0.0, 0.0);
    let corner_arm = (scene_dimensions.x.min(scene_dimensions.y) * 0.10).clamp(0.25, 0.5);

    let mut segments = Vec::new();

    // Corner definitions: (Corner Position, Horizontal Arm Vector Direction, Vertical Arm Vector Direction)
    let corners = [
        (Vec2::new(-half_w, -half_h), Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)),  // BL: └
        (Vec2::new(half_w, -half_h),  Vec2::new(-1.0, 0.0), Vec2::new(0.0, 1.0)), // BR: ┘
        (Vec2::new(half_w, half_h),   Vec2::new(-1.0, 0.0), Vec2::new(0.0, -1.0)),// TR: ┐
        (Vec2::new(-half_w, half_h),  Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0)), // TL: ┌
    ];

    for (corner, h_dir, v_dir) in &corners {
        // Continuous L-bracket path segment:
        // 1. Arm end point (dwell = 4)
        // 2. Corner 90° vertex with high dwell = 14 so physical galvos come to complete stop for 100% sharp 90° corners!
        // 3. Arm end point (dwell = 4)
        let mut l_seg = PathSegment::empty();
        let h_end = *corner + *h_dir * corner_arm;
        let v_end = *corner + *v_dir * corner_arm;

        l_seg.push(h_end.x, h_end.y, blank, 4);
        l_seg.push(h_end.x, h_end.y, green, 5);
        l_seg.push(corner.x, corner.y, green, 14); // Dwell = 14 at vertex for 100% sharp 90° corners!
        l_seg.push(v_end.x, v_end.y, green, 5);
        l_seg.push(v_end.x, v_end.y, blank, 4);

        segments.push(l_seg);
    }

    UniversalPath { segments }
}

/// Spawn a green crosshair at the scene center matching the corner L-brackets
fn spawn_center_crosshair(
    commands: &mut Commands,
    scene_entity: Entity,
) {
    let crosshair_size = 0.5;
    let green = Color::srgb(0.0, 1.0, 0.0); // Matching green color as corner L-brackets
    let crosshair_path = UniversalPath::crosshair(Vec2::ZERO, crosshair_size, green);
    let child_entity = commands.spawn((
        Transform::IDENTITY,
        Visibility::default(),
        CalibrationPath,
        CalibrationCenterCrosshair,
        crosshair_path,
        common::path::PathRenderable::default(),
    )).id();
    commands.entity(scene_entity).add_child(child_entity);
    info!("Spawned green center crosshair overlay parented to SceneEntity");
}

/// Spawn projection area rectangle at scene center (projection surface)
fn spawn_projection_area_rectangle(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
    scene_entity: Entity,
) {
    let rectangle_path = build_calibration_rectangle_path(scene_setup.scene.scene_dimension);
    let child_entity = commands.spawn((
        Transform::IDENTITY,
        Visibility::default(),
        CalibrationPath,
        ProjectionAreaRectangle,
        rectangle_path,
        common::path::PathRenderable::default(),
    )).id();
    commands.entity(scene_entity).add_child(child_entity);
    info!("Spawned scene corner calibration overlay parented to SceneEntity");
}

/// Spawns overlays parented to SceneEntity once SceneEntity is ready
fn spawn_calibration_overlays(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    scene_entity_query: Query<Entity, With<SceneEntity>>,
) {
    let needs_rectangle = rectangle_query.iter().next().is_none();
    let needs_center = center_query.iter().next().is_none();

    if !needs_rectangle && !needs_center {
        return;
    }

    let Ok(scene_entity) = scene_entity_query.single() else {
        // Wait for SceneEntity to be spawned before creating calibration overlays
        return;
    };

    if needs_rectangle {
        spawn_projection_area_rectangle(&mut commands, &scene_setup, scene_entity);
    }
    if needs_center {
        spawn_center_crosshair(&mut commands, scene_entity);
    }
}

// --- Calibration overlay update systems ---
fn update_projection_area_rectangle(
    scene_setup: Res<SceneSetup>,
    mut query: Query<&mut UniversalPath, With<ProjectionAreaRectangle>>,
) {
    if !scene_setup.is_changed() {
        return;
    }
    let rectangle_path = build_calibration_rectangle_path(scene_setup.scene.scene_dimension);
    for mut path in query.iter_mut() {
        *path = rectangle_path.clone();
    }
}

fn gamepad_toggle_calibration(
    time: Res<Time>,
    mut cooldowns: ResMut<CalibrationGamepadCooldowns>,
    mut button_events: MessageReader<GamepadButtonEvent>,
    calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
) {
    cooldowns.calibration_toggle.tick(time.delta());
    if matches!(*next_calibration_state, NextState::Pending(_)) {
        return;
    }

    for event in button_events.read() {
        if event.button == Btn::North && event.pressed && cooldowns.calibration_toggle.is_finished() {
            cooldowns.calibration_toggle.reset();
            let next = match calibration_state.get() {
                CalibrationState::On => CalibrationState::Off,
                CalibrationState::Off => CalibrationState::On,
            };
            info!("Gamepad: Calibration mode toggled to {:?}", next);
            next_calibration_state.set(next);
        }
    }
}

fn gamepad_calibration_controls(
    time: Res<Time>,
    mut cooldowns: ResMut<CalibrationGamepadCooldowns>,
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut target_mode: ResMut<CalibrationTargetMode>,
    mut scene_config: ResMut<SceneConfiguration>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut camera_config: ResMut<CameraConfiguration>,
) {
    if !state.connected { return; }
    let dt = time.delta_secs();

    // ── 0. Target Mode Toggle (Select / Back Button) ──────────────────────────
    cooldowns.target_mode_toggle.tick(time.delta());
    if state.just_pressed(&prev, Btn::Select) && cooldowns.target_mode_toggle.is_finished() {
        cooldowns.target_mode_toggle.reset();
        *target_mode = match *target_mode {
            CalibrationTargetMode::RigPointA => CalibrationTargetMode::BillboardPointB,
            CalibrationTargetMode::BillboardPointB => CalibrationTargetMode::RigPointA,
        };
        match *target_mode {
            CalibrationTargetMode::RigPointA => {
                info!("🎯 [Gamepad Calibration] -> POINT A: PROJECTOR & CAMERA RIG (Pose & Optical FOV)");
            }
            CalibrationTargetMode::BillboardPointB => {
                info!("🎯 [Gamepad Calibration] -> POINT B: BILLBOARD SCENE (Size, Distance & Middle Position)");
            }
        }
    }

    // ── Speed Multipliers (Hover vs Boost vs Precision) ──────────────────────
    let speed_mult = if state.left_thumb {
        0.25 // Precision mode (0.25x)
    } else if state.south {
        2.5  // Boost / Fast Drag mode (2.5x)
    } else {
        1.0  // Normal mode (1.0x)
    };

    let move_speed = 1.8 * speed_mult;   // meters per second
    let scale_speed = 1.5 * speed_mult;  // meters per second
    let dist_speed = 2.0 * speed_mult;   // meters per second
    let angle_speed = 15.0 * speed_mult; // degrees per second

    cooldowns.calibration_step.tick(time.delta());

    // Modifier: Right Bumper / Right Trigger / Right Thumb pressed down for Scene Middle Position
    let is_middle_pos_held = state.right_bumper || state.right_trigger || state.right_thumb;

    match *target_mode {
        CalibrationTargetMode::BillboardPointB => {
            let distance = projector_config.origin.translation.distance(scene_config.origin.translation);
            let half_angle_rad = projector_config.angle.to_radians() / 2.0;
            let max_dim = 2.0 * distance * half_angle_rad.tan();

            if is_middle_pos_held {
                if state.left_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
                    scene_config.origin.translation.x += state.left_stick_x * move_speed * dt;
                }
                if state.left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
                    scene_config.origin.translation.y += state.left_stick_y * move_speed * dt;
                }
            } else {
                if state.left_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
                    scene_config.scene_dimension.x = (scene_config.scene_dimension.x + state.left_stick_x * scale_speed * dt)
                        .clamp(0.2, max_dim);
                }
                if state.left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
                    scene_config.scene_dimension.y = (scene_config.scene_dimension.y + state.left_stick_y * scale_speed * dt)
                        .clamp(0.2, max_dim);
                }
            }

            if state.right_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
                scene_config.origin.translation.z -= state.right_stick_y * dist_speed * dt;
            }
            if state.right_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
                scene_config.origin.translation.x += state.right_stick_x * move_speed * dt;
            }

            if cooldowns.calibration_step.is_finished() {
                let step = 0.1;
                let mut changed = false;

                if state.just_pressed(&prev, Btn::DPadUp) {
                    scene_config.scene_dimension.y = (scene_config.scene_dimension.y + step).min(max_dim);
                    info!("Gamepad Scene: Height -> {:.2}m", scene_config.scene_dimension.y);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadDown) {
                    scene_config.scene_dimension.y = (scene_config.scene_dimension.y - step).max(step);
                    info!("Gamepad Scene: Height -> {:.2}m", scene_config.scene_dimension.y);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadRight) {
                    scene_config.scene_dimension.x = (scene_config.scene_dimension.x + step).min(max_dim);
                    info!("Gamepad Scene: Width -> {:.2}m", scene_config.scene_dimension.x);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadLeft) {
                    scene_config.scene_dimension.x = (scene_config.scene_dimension.x - step).max(step);
                    info!("Gamepad Scene: Width -> {:.2}m", scene_config.scene_dimension.x);
                    changed = true;
                }

                if changed {
                    cooldowns.calibration_step.reset();
                }
            }
        }

        CalibrationTargetMode::RigPointA => {
            let mut rig_delta = Vec3::ZERO;

            if state.left_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
                rig_delta.x += state.left_stick_x * move_speed * dt;
            }
            if state.left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
                rig_delta.y += state.left_stick_y * move_speed * dt;
            }
            if state.right_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
                rig_delta.z -= state.right_stick_y * dist_speed * dt;
            }

            if state.right_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
                projector_config.angle = (projector_config.angle + state.right_stick_x * angle_speed * dt)
                    .clamp(15.0, 90.0);
            }

            if cooldowns.calibration_step.is_finished() {
                let step = 0.1;
                let mut changed = false;

                if state.just_pressed(&prev, Btn::DPadUp) {
                    rig_delta.y += step;
                    info!("Gamepad Rig: Height -> {:.2}m", projector_config.origin.translation.y + step);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadDown) {
                    rig_delta.y -= step;
                    info!("Gamepad Rig: Height -> {:.2}m", projector_config.origin.translation.y - step);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadRight) {
                    rig_delta.x += step;
                    info!("Gamepad Rig: Lateral X -> {:.2}m", projector_config.origin.translation.x + step);
                    changed = true;
                }
                if state.just_pressed(&prev, Btn::DPadLeft) {
                    rig_delta.x -= step;
                    info!("Gamepad Rig: Lateral X -> {:.2}m", projector_config.origin.translation.x - step);
                    changed = true;
                }

                if changed {
                    cooldowns.calibration_step.reset();
                }
            }

            if rig_delta != Vec3::ZERO {
                projector_config.origin.translation += rig_delta;
                camera_config.origin.translation += rig_delta;
            }

            let billboard_center = scene_config.origin.translation;
            projector_config.origin.rotation = Transform::from_translation(projector_config.origin.translation)
                .looking_at(billboard_center, Vec3::Y).rotation;
            camera_config.origin.rotation = Transform::from_translation(camera_config.origin.translation)
                .looking_at(billboard_center, Vec3::Y).rotation;
        }
    }
}