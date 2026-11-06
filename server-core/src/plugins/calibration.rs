use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::MousePositionEvent;
use common::path::{PathSegment, UniversalPath};
use common::scene::SceneEntity;
use common::scene::{SceneSetup, SceneSystemSet};
use common::calibration::{nudge, CalibrationAction, CalibrationMode, CalibrationOverlays};
use common::shape_v2::Shape;
use common::state::CalibrationState;
use laserlogic::helios::HELIOS_MAX_COORD;
use gamepad::{Btn, GamepadButtonEvent, GamepadState, PrevGamepadState, GAMEPAD_STICK_DEADZONE};
use crate::plugins::projector::LaserOptimizeConfig;
use laserlogic::OptimizeConfig;

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
    mut overlays: ResMut<CalibrationOverlays>,
    mut center_shape: ResMut<CenterShape>,
    center_shapes: Res<CenterShapes>,
    mut selected_timing: ResMut<SelectedTimingValue>,
    mut laser_config: Option<ResMut<LaserOptimizeConfig>>,
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
            CalibrationAction::CycleCalibration => {
                let current = if calibration_on { *overlays } else { CalibrationOverlays::Off };
                let next = current.next();
                *overlays = next;
                info!("[Calibration] Overlays: {}", next.label());
                if next.calibration_on() != calibration_on {
                    calibration_on = next.calibration_on();
                    next_calibration_state.set(if calibration_on { CalibrationState::On } else { CalibrationState::Off });
                }
                if next == CalibrationOverlays::All {
                    // All back on starts again from the crosshair
                    center_shape.set_if_neq(CenterShape(0));
                    info!("[Calibration] Mode {}", mode.help());
                }
            }
            CalibrationAction::NextShape if calibration_on => {
                center_shape.0 = (center_shape.0 + 1) % center_shapes.0.len();
            }
            CalibrationAction::ToggleLaser => {
                projector_config.switched_on = !projector_config.switched_on;
            }
            CalibrationAction::NextMode if calibration_on => {
                *mode = mode.next();
                info!("[Calibration] Mode {}", mode.help());
                if *mode == CalibrationMode::LaserTiming {
                    if let Some(laser) = laser_config.as_mut() {
                        log_all_timing(&mut laser.0);
                    }
                }
            }
            CalibrationAction::Nudge { dir, fine } if calibration_on && *mode == CalibrationMode::LaserTiming => {
                if let Some(laser) = laser_config.as_mut() {
                    nudge_timing(dir, fine, &mut selected_timing, &mut laser.0);
                }
            }
            CalibrationAction::Nudge { dir, fine } if calibration_on => {
                if nudge(*mode, dir, fine, &mut setup) {
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
    /// Per-client mouse position in billboard x, y (metres from the billboard centre). The mouse
    /// crosshair is a child of the billboard (scene entity), so it moves with it.
    pub mouse_positions: std::collections::HashMap<u64, Vec2>,
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

/// Shapes the shape key cycles through in place of the centre crosshair, as (name, path).
/// The first one is the calibration crosshair; the rest are the built-in v2 templates.
#[derive(Resource)]
pub struct CenterShapes(pub Vec<(String, UniversalPath)>);

impl CenterShapes {
    fn new() -> Self {
        let crosshair = ("crosshair".to_string(), crosshair_strokes(0.5, Color::srgb(1.0, 1.0, 0.0)));
        let templates = Shape::builtin()
            .into_iter()
            .filter(|shape| shape.name != "crosshair")
            .map(|shape| (shape.name.clone(), shape.to_universal_path()));
        Self(std::iter::once(crosshair).chain(templates).collect())
    }
}

/// One laser stage value the LASER TIMING mode can change live
pub struct TimingValue {
    pub name: &'static str,
    /// Label in the overview line
    pub short: &'static str,
    pub unit: &'static str,
    /// Change per Left/Right press; Shift uses a fifth of it
    pub step: f32,
    pub min: f32,
    pub max: f32,
    pub field: fn(&mut OptimizeConfig) -> &mut f32,
}

/// The values in the order Up/Down walks through them: the ones that decide whether strokes
/// start, end and turn where they should come first
pub const TIMING_VALUES: [TimingValue; 11] = [
    TimingValue { name: "laser_on_delay_ms", short: "ON", unit: "ms", step: 0.02, min: -2.0, max: 2.0, field: |c| &mut c.laser_on_delay_ms },
    TimingValue { name: "laser_off_delay_ms", short: "OFF", unit: "ms", step: 0.02, min: -2.0, max: 2.0, field: |c| &mut c.laser_off_delay_ms },
    TimingValue { name: "corner_hold_ms", short: "CH", unit: "ms", step: 0.05, min: 0.0, max: 3.0, field: |c| &mut c.corner_hold_ms },
    TimingValue { name: "stroke_start_hold_ms", short: "SH", unit: "ms", step: 0.05, min: 0.0, max: 3.0, field: |c| &mut c.stroke_start_hold_ms },
    TimingValue { name: "stroke_end_hold_ms", short: "EH", unit: "ms", step: 0.05, min: 0.0, max: 3.0, field: |c| &mut c.stroke_end_hold_ms },
    TimingValue { name: "corner_ramp_ms", short: "CR", unit: "ms", step: 0.05, min: 0.0, max: 2.0, field: |c| &mut c.corner_ramp_ms },
    TimingValue { name: "settle_ms", short: "ST", unit: "ms", step: 0.05, min: 0.0, max: 5.0, field: |c| &mut c.settle_ms },
    TimingValue { name: "depart_ms", short: "DP", unit: "ms", step: 0.05, min: 0.0, max: 5.0, field: |c| &mut c.depart_ms },
    TimingValue { name: "lit_speed_m_s", short: "LS", unit: "m/s", step: 500.0, min: 500.0, max: 50000.0, field: |c| &mut c.lit_speed_m_s },
    TimingValue { name: "max_lit_step_dac", short: "MLS", unit: "DAC", step: 10.0, min: 10.0, max: 1000.0, field: |c| &mut c.max_lit_step_dac },
    TimingValue { name: "blank_speed_m_s", short: "BS", unit: "m/s", step: 1000.0, min: 1000.0, max: 100000.0, field: |c| &mut c.blank_speed_m_s },
];

/// Index into [`TIMING_VALUES`] selected in the LASER TIMING mode
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedTimingValue(pub usize);

/// Applies one arrow press in the LASER TIMING mode: Up/Down select, Left/Right change
pub fn nudge_timing(dir: IVec2, fine: bool, selected: &mut SelectedTimingValue, config: &mut OptimizeConfig) {
    let count = TIMING_VALUES.len();
    if dir.y != 0 {
        // Up goes to the previous value, as in a list
        selected.0 = (selected.0 + count - (dir.y.signum() + count as i32) as usize % count) % count;
    }
    if dir.x != 0 {
        let value = &TIMING_VALUES[selected.0];
        let step = if fine { value.step / 5.0 } else { value.step };
        let field = (value.field)(config);
        // Round to the step grid so repeated presses do not accumulate float noise
        let next = *field + dir.x as f32 * step;
        *field = ((next / (value.step / 5.0)).round() * (value.step / 5.0)).clamp(value.min, value.max);
    }
}

/// Key to the abbreviations in the overview line
pub const OVERVIEW_LEGEND: &str = "[CAL] key: W/H scene width/height m, MID scene middle height m, DST rig-to-scene distance m, \
RIG rig height m, FOV projector field of view deg | ON/OFF light earlier (+) or later (-) at stroke start/end ms, \
CH corner hold ms, SH/EH stroke start/end hold ms, CR corner ramp ms, \
ST settle ms, DP depart ms, LS lit speed m/s, MLS max lit step DAC, BS blank speed m/s | LSR laser, DAC connected, \
OVL overlays, SHP centre shape | [..] = values of the current mode, > = selected timing value";

fn short_number(value: f32, unit: &str) -> String {
    match unit {
        "m/s" if value >= 1000.0 => {
            let k = value / 1000.0;
            if k.fract() == 0.0 { format!("{}k", k) } else { format!("{:.1}k", k) }
        }
        "DAC" | "m/s" => format!("{}", value.round()),
        _ => format!("{:.2}", value),
    }
}

/// One-line overview of the major settings; the current mode's values are in brackets
pub fn overview(
    mode: CalibrationMode,
    overlays: CalibrationOverlays,
    shape: &str,
    selected: SelectedTimingValue,
    setup: &SceneSetup,
    laser: &OptimizeConfig,
) -> String {
    let section = |of: CalibrationMode, text: String| if of == mode { format!("[{}]", text) } else { text };
    let (scene, projector) = (&setup.scene, &setup.projector);
    let mut laser = laser.clone();
    let timing: Vec<String> = TIMING_VALUES
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let marker = if mode == CalibrationMode::LaserTiming && i == selected.0 { ">" } else { "" };
            format!("{}{}{}", marker, value.short, short_number(*(value.field)(&mut laser), value.unit))
        })
        .collect();
    let overlays = match overlays {
        CalibrationOverlays::All => "all",
        CalibrationOverlays::NoPerimeter => "no-red",
        CalibrationOverlays::CrosshairOnly => "centre",
        CalibrationOverlays::Off => "off",
    };
    format!(
        "[CAL] {} | {} | {} | {} | LSR {} DAC {} | OVL {} | SHP {}",
        section(CalibrationMode::SceneSize, format!("W{:.2} H{:.2}", scene.scene_dimension.x, scene.scene_dimension.y)),
        section(
            CalibrationMode::ScenePosition,
            format!("MID{:.2} DST{:.2}", scene.origin.translation.y, projector.origin.translation.z - scene.origin.translation.z),
        ),
        section(CalibrationMode::Rig, format!("RIG{:.2} FOV{:.1}", projector.origin.translation.y, projector.angle)),
        section(CalibrationMode::LaserTiming, timing.join(" ")),
        if projector.switched_on { "on" } else { "off" },
        if projector.connected { "ok" } else { "--" },
        overlays,
        shape,
    )
}

/// Shortest time between two overview lines while values keep changing (held keys)
const OVERVIEW_INTERVAL_SECS: f32 = 0.25;

#[derive(Default)]
struct OverviewThrottle {
    since_last: f32,
    pending: bool,
    started: bool,
}

/// Logs the overview line whenever a major setting changes, from any source (keys, terminal,
/// gamepad). Held keys print at most every OVERVIEW_INTERVAL_SECS; the final value always prints.
#[allow(clippy::too_many_arguments)]
fn log_overview_on_change(
    time: Res<Time>,
    mode: Res<CalibrationMode>,
    overlays: Res<CalibrationOverlays>,
    center_shape: Res<CenterShape>,
    center_shapes: Res<CenterShapes>,
    selected: Res<SelectedTimingValue>,
    scene: Res<SceneConfiguration>,
    camera: Res<CameraConfiguration>,
    projector: Res<ProjectorConfiguration>,
    laser: Option<Res<LaserOptimizeConfig>>,
    mut throttle: Local<OverviewThrottle>,
) {
    if !throttle.started {
        throttle.started = true;
        info!("{}", OVERVIEW_LEGEND);
    }
    let laser_changed = laser.as_ref().is_some_and(|l| l.is_changed());
    if mode.is_changed() || overlays.is_changed() || center_shape.is_changed() || selected.is_changed()
        || scene.is_changed() || camera.is_changed() || projector.is_changed() || laser_changed
    {
        throttle.pending = true;
    }
    throttle.since_last += time.delta_secs();
    if throttle.pending && throttle.since_last >= OVERVIEW_INTERVAL_SECS {
        throttle.pending = false;
        throttle.since_last = 0.0;
        let setup = SceneSetup::new(scene.clone(), camera.clone(), projector.clone());
        let laser = laser.map(|l| l.0.clone()).unwrap_or_default();
        let shape = &center_shapes.0[center_shape.0].0;
        info!("{}", overview(*mode, *overlays, shape, *selected, &setup, &laser));
    }
}

/// All timing values as server.yaml lines, so tuned values can be copied into the config file
fn log_all_timing(config: &mut OptimizeConfig) {
    info!("[Laser timing] Current values (projector.optimize in server.yaml):");
    for value in &TIMING_VALUES {
        info!("[Laser timing]   {}: {}", value.name, *(value.field)(config));
    }
}

/// Index into [`CenterShapes`] of the shape drawn at the scene centre
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CenterShape(pub usize);

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CalibrationData>()
            .init_resource::<CalibrationGamepadCooldowns>()
            .init_resource::<CalibrationTargetMode>()
            .init_resource::<CalibrationMode>()
            .init_resource::<CalibrationOverlays>()
            .init_resource::<CenterShape>()
            .init_resource::<SelectedTimingValue>()
            .insert_resource(CenterShapes::new())
            .add_message::<CalibrationAction>()
            // Calibration switched on or off from elsewhere (terminal, gamepad): overlays follow
            .add_systems(OnEnter(CalibrationState::On), |mut overlays: ResMut<CalibrationOverlays>| {
                if *overlays == CalibrationOverlays::Off {
                    *overlays = CalibrationOverlays::All;
                }
            })
            .add_systems(OnEnter(CalibrationState::Off), |mut overlays: ResMut<CalibrationOverlays>| {
                overlays.set_if_neq(CalibrationOverlays::Off);
            })
            .add_systems(Update, apply_calibration_actions.before(SceneSystemSet))
            .add_systems(Update, log_overview_on_change.after(apply_calibration_actions))
            .add_systems(OnExit(CalibrationState::On), despawn_calibration_overlays.after(SceneSystemSet))
            .add_systems(Update, handle_mouse_position_updates)
            .add_systems(Update, cleanup_disconnected_clients)
            .add_systems(Update, (
                spawn_calibration_overlays,
                update_and_maintain_crosshair_positions,
            ).after(SceneSystemSet).run_if(in_state(CalibrationState::On)))
            .add_systems(Update, update_projection_area_rectangle.run_if(in_state(CalibrationState::On)))
            .add_systems(Update, (apply_overlay_visibility, update_center_shape)
                .after(spawn_calibration_overlays)
                .run_if(in_state(CalibrationState::On)))
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
    scene_config: Res<SceneConfiguration>,
) {
    // Always track mouse positions from all clients, as x, y on the billboard
    let setup = SceneSetup { scene: scene_config.clone(), ..Default::default() };
    for event in mouse_events.read() {
        if let Some(world_pos) = event.position {
            calibration_data.mouse_positions.insert(event.client_id, setup.world_to_billboard(world_pos));
        } else {
            calibration_data.mouse_positions.remove(&event.client_id);
        }
    }
}

fn build_calibration_crosshair_path() -> UniversalPath {
    crosshair_strokes(0.4, Color::srgb(0.0, 0.9, 1.0))
}

/// Calibration crosshair as two plain lit strokes (v2 style): no dwell overrides and no blank
/// lead points, so its stroke ends get the laser stage's holds and delays and follow live tuning
fn crosshair_strokes(size: f32, color: Color) -> UniversalPath {
    let half = size / 2.0;
    let stroke = |from: Vec2, to: Vec2| {
        let mut segment = PathSegment::empty();
        segment.push_vec2(from, color, 0);
        segment.push_vec2(to, color, 0);
        segment
    };
    UniversalPath {
        segments: vec![
            stroke(Vec2::new(-half, 0.0), Vec2::new(half, 0.0)),
            stroke(Vec2::new(0.0, -half), Vec2::new(0.0, half)),
        ],
    }
}


fn update_and_maintain_crosshair_positions(
    mut commands: Commands,
    calibration_data: Res<CalibrationData>,
    scene_query: Query<Entity, With<SceneEntity>>,
    mut crosshair_query: Query<(Entity, &mut Transform, &CalibrationCrosshair)>,
) {
    let scene_entity = scene_query.single().ok();
    let mut existing_clients: std::collections::HashMap<u64, Entity> = std::collections::HashMap::new();

    for (entity, mut transform, crosshair) in crosshair_query.iter_mut() {
        if let Some(&local) = calibration_data.mouse_positions.get(&crosshair.client_id) {
            transform.translation = local.extend(0.0);
            existing_clients.insert(crosshair.client_id, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Auto-respawn crosshair if client is actively tracking but entity was despawned
    for (&client_id, &local) in calibration_data.mouse_positions.iter() {
        if !existing_clients.contains_key(&client_id) {
            spawn_crosshair_at_position(&mut commands, client_id, local, scene_entity);
            info!("✓ Auto-restored active crosshair for client {} at scene {:?}", client_id, local);
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

/// Spawn a mouse crosshair at scene-local x, y as a child of the scene entity
fn spawn_crosshair_at_position(
    commands: &mut Commands,
    client_id: u64,
    local: Vec2,
    scene_entity: Option<Entity>,
) {
    let child_entity = commands.spawn((
        Transform::from_translation(local.extend(0.0)),
        Visibility::default(),
        CalibrationPath,
        CalibrationCrosshair { client_id },
        build_calibration_crosshair_path(),
        common::path::PathRenderable::default(),
    )).id();

    if let Some(scene_e) = scene_entity {
        commands.entity(scene_e).add_child(child_entity);
    }

    info!("Spawned mouse crosshair for client {} at scene {:?}", client_id, local);
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
    let crosshair_path = crosshair_strokes(crosshair_size, yellow);
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
/// Hides the red perimeter and green scene box according to [`CalibrationOverlays`]
fn apply_overlay_visibility(
    overlays: Res<CalibrationOverlays>,
    mut query: Query<(&mut Visibility, Has<DacPerimeter>), Or<(With<DacPerimeter>, With<ProjectionAreaRectangle>)>>,
) {
    for (mut visibility, is_perimeter) in &mut query {
        let shown = if is_perimeter { overlays.shows_perimeter() } else { overlays.shows_scene_box() };
        visibility.set_if_neq(if shown { Visibility::Inherited } else { Visibility::Hidden });
    }
}

/// Draws the selected [`CenterShape`] on the centre crosshair entity
fn update_center_shape(
    center_shape: Res<CenterShape>,
    center_shapes: Res<CenterShapes>,
    mut query: Query<(&mut UniversalPath, Ref<CalibrationCenterCrosshair>)>,
) {
    for (mut path, marker) in &mut query {
        if center_shape.is_changed() || marker.is_added() {
            *path = center_shapes.0[center_shape.0].1.clone();
        }
    }
}

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

/// Uses only [`gamepad::CALIBRATION_BUTTONS`] (and the sticks); games keep the other buttons
/// while calibrating, so a button added here must be added there too.
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
    fn overview_is_one_line_and_brackets_the_current_mode() {
        let setup = SceneSetup::default();
        let laser = OptimizeConfig::default();
        let line = overview(CalibrationMode::SceneSize, CalibrationOverlays::All, "crosshair", SelectedTimingValue(2), &setup, &laser);
        assert!(!line.contains('\n'));
        assert!(line.contains("[W10.00 H6.00]"), "{}", line);
        assert!(line.contains("MID3.00 DST10.00 |"), "{}", line);
        assert!(line.contains("LS10k") && line.contains("BS20k") && line.contains("MLS150"), "{}", line);
        assert!(!line.contains('>'), "no selection marker outside LASER TIMING: {}", line);
        assert!(line.ends_with("OVL all | SHP crosshair"), "{}", line);

        let line = overview(CalibrationMode::LaserTiming, CalibrationOverlays::Off, "box", SelectedTimingValue(2), &setup, &laser);
        assert!(line.contains("[ON0.00 OFF0.00 >CH0.20 SH0.20 EH0.20 CR0.10"), "{}", line);
        assert!(line.contains("W10.00 H6.00 |") && !line.contains("[W"), "{}", line);
    }

    #[test]
    fn timing_mode_selects_with_up_down_and_changes_with_left_right() {
        let mut config = OptimizeConfig::default();
        let mut selected = SelectedTimingValue::default();
        assert_eq!(TIMING_VALUES[0].name, "laser_on_delay_ms");

        nudge_timing(IVec2::X, false, &mut selected, &mut config);
        assert!((config.laser_on_delay_ms - 0.02).abs() < 1e-6);
        nudge_timing(IVec2::X, true, &mut selected, &mut config);
        assert!((config.laser_on_delay_ms - 0.024).abs() < 1e-6, "{}", config.laser_on_delay_ms);
        // Negative is allowed (light later), down to the minimum
        for _ in 0..200 {
            nudge_timing(IVec2::NEG_X, false, &mut selected, &mut config);
        }
        assert_eq!(config.laser_on_delay_ms, -2.0);

        // Down moves to the next value, Up back, wrapping at both ends
        nudge_timing(IVec2::NEG_Y, false, &mut selected, &mut config);
        assert_eq!(TIMING_VALUES[selected.0].name, "laser_off_delay_ms");
        nudge_timing(IVec2::Y, false, &mut selected, &mut config);
        nudge_timing(IVec2::Y, false, &mut selected, &mut config);
        assert_eq!(selected.0, TIMING_VALUES.len() - 1);

        // Each entry edits its own field
        let hold = config.corner_hold_ms;
        selected.0 = 2;
        nudge_timing(IVec2::X, false, &mut selected, &mut config);
        assert!((config.corner_hold_ms - (hold + 0.05)).abs() < 1e-6);
    }

    #[test]
    fn dac_perimeter_spans_full_dac_range() {
        let max = HELIOS_MAX_COORD as f32;
        let points: Vec<Vec2> = build_dac_perimeter_path().flatten();
        let (min_p, max_p) = points.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        assert_eq!(min_p, Vec2::ZERO);
        assert_eq!(max_p, Vec2::splat(max));
    }
}
