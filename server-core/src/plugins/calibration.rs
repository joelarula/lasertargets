use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::MousePositionEvent;
use common::path::{PathSegment, UniversalPath};
use common::scene::SceneEntity;
use common::scene::{SceneSetup, SceneSystemSet};
use common::calibration::{log_calibration_status, nudge, CalibrationAction, CalibrationMode};
use common::state::CalibrationState;
use laserlogic::helios::HELIOS_MAX_COORD;
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

fn apply_calibration_actions(
    mut actions: MessageReader<CalibrationAction>,
    calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
    mut mode: ResMut<CalibrationMode>,
    mut scene_config: ResMut<SceneConfiguration>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut camera_config: ResMut<CameraConfiguration>,
) {
    // Track the state locally so several toggles in one frame resolve in order
    let mut calibration_on = match &*next_calibration_state {
        NextState::Pending(state) => *state == CalibrationState::On,
        _ => *calibration_state.get() == CalibrationState::On,
    };

    for action in actions.read() {
        // Built from the configs, which stay the source of truth (the SceneSetup resource lags a frame)
        let mut setup = SceneSetup::new(scene_config.clone(), camera_config.clone(), projector_config.clone());
        match *action {
            CalibrationAction::ToggleCalibration => {
                calibration_on = !calibration_on;
                let next = if calibration_on { CalibrationState::On } else { CalibrationState::Off };
                info!("[Calibration] {:?}", next);
                if calibration_on {
                    info!("[Calibration] Mode {}", mode.help());
                }
                next_calibration_state.set(next);
            }
            CalibrationAction::ToggleLaser => {
                projector_config.switched_on = !projector_config.switched_on;
                info!("[Calibration] Laser {}", if projector_config.switched_on { "ON" } else { "OFF" });
            }
            CalibrationAction::NextMode if calibration_on => {
                *mode = mode.next();
                info!("[Calibration] Mode {}", mode.help());
                log_calibration_status(*mode, &setup);
            }
            CalibrationAction::Nudge { dir, fine } if calibration_on => {
                if nudge(*mode, dir, fine, &mut setup) {
                    log_calibration_status(*mode, &setup);
                    scene_config.set_if_neq(setup.scene);
                    projector_config.set_if_neq(setup.projector);
                    camera_config.set_if_neq(setup.camera);
                } else {
                    info!("[Calibration] Scene at the red DAC perimeter, not moving further");
                }
            }
            _ => {}
        }
    }
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

/// Component to mark calibration-only paths (not broadcast to terminals).
/// They are drawn only while calibration is on and are not culled to the scene bounds.
#[derive(Component)]
pub struct CalibrationPath;

/// Path whose points are raw DAC coordinates (0..=HELIOS_MAX_COORD), sent as-is without any
/// world or projector transform.
#[derive(Component)]
pub struct DacSpacePath;

/// Component to mark the red DAC perimeter entity
#[derive(Component)]
pub struct DacPerimeter;

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CalibrationData>()
            .init_resource::<CalibrationGamepadCooldowns>()
            .init_resource::<CalibrationTargetMode>()
            .init_resource::<CalibrationMode>()
            .add_message::<CalibrationAction>()
            .add_systems(Update, apply_calibration_actions.before(SceneSystemSet))
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

/// Closed box through `corners` with a long dwell at every vertex for sharp 90 degree corners.
fn closed_box_segment(corners: [Vec2; 4], color: Color) -> PathSegment {
    let blank = Color::srgb(0.0, 0.0, 0.0);
    let mut seg = PathSegment::empty();
    seg.push_vec2(corners[0], blank, 4);
    for corner in corners.iter().chain(std::iter::once(&corners[0])) {
        seg.push_vec2(*corner, color, 14);
    }
    seg.push_vec2(corners[0], blank, 4);
    seg
}

/// Green box outlining the scene bounds (scene-local metres).
fn build_calibration_rectangle_path(scene_dimensions: Vec2) -> UniversalPath {
    let half = scene_dimensions / 2.0;
    let green = Color::srgb(0.0, 1.0, 0.0);
    let corners = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ];
    UniversalPath { segments: vec![closed_box_segment(corners, green)] }
}

/// Red box around the full DAC coordinate range with inward ticks at each side's midpoint
/// (DAC centre), in raw DAC units. Shows the X/Y proportions of the galvo signal on the wall.
fn build_dac_perimeter_path() -> UniversalPath {
    let max = HELIOS_MAX_COORD as f32;
    let mid = max / 2.0;
    let tick = max * 0.05;
    let red = Color::srgb(1.0, 0.0, 0.0);
    let blank = Color::srgb(0.0, 0.0, 0.0);
    let corners = [Vec2::ZERO, Vec2::new(max, 0.0), Vec2::new(max, max), Vec2::new(0.0, max)];

    let mut segments = vec![closed_box_segment(corners, red)];
    for (edge, inward) in [
        (Vec2::new(mid, 0.0), Vec2::Y),
        (Vec2::new(max, mid), -Vec2::X),
        (Vec2::new(mid, max), -Vec2::Y),
        (Vec2::new(0.0, mid), Vec2::X),
    ] {
        let mut seg = PathSegment::empty();
        let end = edge + inward * tick;
        seg.push_vec2(edge, blank, 4);
        seg.push_vec2(edge, red, 5);
        seg.push_vec2(end, red, 5);
        seg.push_vec2(end, blank, 4);
        segments.push(seg);
    }
    UniversalPath { segments }
}

/// Spawn a yellow crosshair at the scene center
fn spawn_center_crosshair(
    commands: &mut Commands,
    scene_entity: Entity,
) {
    let crosshair_size = 0.5;
    let yellow = Color::srgb(1.0, 1.0, 0.0);
    let crosshair_path = UniversalPath::crosshair(Vec2::ZERO, crosshair_size, yellow);
    let child_entity = commands.spawn((
        Transform::IDENTITY,
        Visibility::default(),
        CalibrationPath,
        CalibrationCenterCrosshair,
        crosshair_path,
        common::path::PathRenderable::default(),
    )).id();
    commands.entity(scene_entity).add_child(child_entity);
    info!("Spawned yellow center crosshair overlay parented to SceneEntity");
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
    info!("Spawned green scene box overlay parented to SceneEntity");
}

/// Spawns overlays parented to SceneEntity once SceneEntity is ready
fn spawn_calibration_overlays(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    perimeter_query: Query<Entity, With<DacPerimeter>>,
    scene_entity_query: Query<Entity, With<SceneEntity>>,
) {
    if perimeter_query.iter().next().is_none() {
        // DAC space: not parented to the scene, and no PathRenderable so terminals never get it
        commands.spawn((
            Transform::IDENTITY,
            Visibility::default(),
            CalibrationPath,
            DacSpacePath,
            DacPerimeter,
            build_dac_perimeter_path(),
        ));
        info!("Spawned red DAC perimeter overlay");
    }

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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dac_perimeter_spans_full_dac_range() {
        let max = HELIOS_MAX_COORD as f32;
        let points: Vec<Vec2> = build_dac_perimeter_path().flatten();
        let (min_p, max_p) = points.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        assert_eq!(min_p, Vec2::ZERO);
        assert_eq!(max_p, Vec2::splat(max));
    }
}
