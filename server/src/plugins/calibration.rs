use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::path::{UniversalPath, PathSegment};
use common::scene::SceneEntity;
use common::scene::{SceneSetup, SceneSystemSet};
use common::game::GameSession;
use common::state::{CalibrationState, GameState};
use crate::plugins::network::{MousePositionEvent};

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
            .add_systems(OnExit(CalibrationState::On), despawn_calibration_overlays.after(SceneSystemSet))
            .add_systems(Update, handle_mouse_position_updates)
            .add_systems(Update, (
                spawn_calibration_overlays,
                update_and_maintain_crosshair_positions,
                cleanup_disconnected_clients,
            ).after(SceneSystemSet))
            .add_systems(Update, update_projection_area_rectangle)
            .add_systems(Update, update_center_crosshair);
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
}fn build_hunter_reticle_path(selection: Option<&hunter::server::HunterTargetSelection>) -> UniversalPath {
    let crosshair_size = 0.4;
    let cyan = Color::srgb(0.0, 0.9, 1.0);
    let blank = Color::srgb(0.0, 0.0, 0.0);
    
    // 1. Crosshair (+) with 2 separate segments (Horizontal & Vertical)
    let crosshair_base = UniversalPath::crosshair(Vec2::ZERO, crosshair_size, cyan);
    let mut segments = crosshair_base.segments;

    // 2. Selected target visual preview outline centered at reticle cursor (only in Target Release modes 1-4)
    if let Some(sel) = selection {
        if let Some(target) = sel.get_target() {
            let (radius, color, is_balloon) = match target {
                common::target::HunterTarget::Basic(size, color) => (size, color, false),
                common::target::HunterTarget::Baloon(size, color) => (size, color, true),
            };

            let mut preview_seg = common::path::PathSegment::empty();
            if is_balloon {
                let num_pts = 16;
                preview_seg.push(0.0, radius, blank, 3);
                for i in 0..=num_pts {
                    let angle = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
                    let px = radius * angle.cos();
                    let py = radius * angle.sin();
                    preview_seg.push(px, py, color, 0);
                }
                // Knot
                preview_seg.push(0.0, -radius, color, 2);
                preview_seg.push(-0.04, -radius - 0.04, color, 2);
                preview_seg.push(0.04, -radius - 0.04, color, 2);
                preview_seg.push(0.0, -radius, color, 2);
                // String
                preview_seg.push(0.0, -radius - 0.12, color, 2);
                preview_seg.push(0.0, -radius - 0.12, blank, 3);
            } else {
                let num_pts = 24;
                let px_start = radius;
                let py_start = 0.0;

                // 1. Blank move + 4 blanked dwell points at start position
                preview_seg.push(px_start, py_start, blank, 4);

                // 2. 4 lit dwell points at start position
                preview_seg.push(px_start, py_start, color, 4);

                // 3. Circle perimeter
                for i in 1..=num_pts {
                    let angle = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
                    let px = radius * angle.cos();
                    let py = radius * angle.sin();
                    preview_seg.push(px, py, color, 0);
                }

                // 4. Extra 2 overlap points past 360° for 100% closed reticle circle!
                for i in 1..=2 {
                    let angle = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
                    let px = radius * angle.cos();
                    let py = radius * angle.sin();
                    preview_seg.push(px, py, color, 0);
                }

                let end_angle = (2.0 as f32 / num_pts as f32) * std::f32::consts::TAU;
                let px_end = radius * end_angle.cos();
                let py_end = radius * end_angle.sin();

                // 5. 4 lit dwell points at end position
                preview_seg.push(px_end, py_end, color, 4);

                // 6. 4 blanked dwell points at end position (laser OFF at end position before moving)
                preview_seg.push(px_end, py_end, blank, 4);
            }
            segments.push(preview_seg);
        }
    }

    UniversalPath { segments }
}

fn update_and_maintain_crosshair_positions(
    mut commands: Commands,
    calibration_data: Res<CalibrationData>,
    game_sessions: Query<&GameSession>,
    hunter_selection: Option<Res<hunter::server::HunterTargetSelection>>,
    title_announcements: Query<Entity, With<hunter::server::HunterTitleAnnouncement>>,
    scene_query: Query<(Entity, &Transform), (With<SceneEntity>, Without<CalibrationCrosshair>)>,
    mut crosshair_query: Query<(Entity, &mut Transform, &mut GlobalTransform, Option<&ChildOf>, &mut UniversalPath, &CalibrationCrosshair)>,
) {
    // Reticle cursor is an exclusive feature of Hunter game ONLY
    let hunter_active = game_sessions
        .iter()
        .any(|s| s.game_id == 101 && s.state == GameState::InGame);

    // Suppress reticle cursor spawning while game intro vector title announcement ("HUNTER") is active/displayed
    let title_active = title_announcements.iter().next().is_some();

    if !hunter_active || title_active {
        // Despawn all cursor crosshairs while intro text is displayed or when not in Hunter game mode
        for (entity, _, _, _, _, _) in crosshair_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    let scene_transform = scene_query.single().ok().map(|(_, t)| t);
    let mut existing_clients: std::collections::HashMap<u64, Entity> = std::collections::HashMap::new();

    let selection_changed = hunter_selection.as_ref().map(|s| s.is_changed()).unwrap_or(false);

    for (entity, mut transform, mut global_transform, parent, _path, crosshair) in crosshair_query.iter_mut() {
        if selection_changed {
            commands.entity(entity).despawn();
            continue;
        }

        if let Some(&world_pos) = calibration_data.mouse_positions.get(&crosshair.client_id) {
            let local_pos = if parent.is_some() {
                if let Some(scene_t) = scene_transform {
                    let scene_matrix = Mat4::from_scale_rotation_translation(scene_t.scale, scene_t.rotation, scene_t.translation);
                    scene_matrix.inverse().transform_point3(world_pos)
                } else {
                    world_pos
                }
            } else {
                world_pos
            };
            transform.translation = local_pos;
            *global_transform = GlobalTransform::from(Transform::from_translation(world_pos));

            existing_clients.insert(crosshair.client_id, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Auto-respawn crosshair if entity was despawned during scene/state transition
    let scene_entity = scene_query.single().ok().map(|(e, _)| e);
    for (&client_id, &world_pos) in calibration_data.mouse_positions.iter() {
        if !existing_clients.contains_key(&client_id) {
            spawn_crosshair_at_position(&mut commands, client_id, world_pos, scene_entity, scene_transform, hunter_selection.as_deref());
            info!("✓ Auto-restored active crosshair for client {} at {:?}", client_id, world_pos);
        }
    }
}

fn cleanup_disconnected_clients(
    mut connection_lost_events: MessageReader<ConnectionLostEvent>,
    mut calibration_data: ResMut<CalibrationData>,
    mut commands: Commands,
    crosshair_query: Query<(Entity, &CalibrationCrosshair)>,
) {
    for connection_lost in connection_lost_events.read() {
        let client_id = connection_lost.id;
        
        // Remove from mouse positions tracking
        calibration_data.mouse_positions.remove(&client_id);
        
        for (entity, crosshair) in crosshair_query.iter() {
            if crosshair.client_id == client_id {
                commands.entity(entity).despawn();
            }
        }

        info!("Cleaned up mouse tracking and crosshair for disconnected client {}", client_id);
    }
}

/// Spawn crosshair at specific position, parented to SceneEntity if available
fn spawn_crosshair_at_position(
    commands: &mut Commands,
    client_id: u64,
    world_position: Vec3,
    scene_entity: Option<Entity>,
    scene_transform: Option<&Transform>,
    selection: Option<&hunter::server::HunterTargetSelection>,
) {
    let crosshair_universal_path = build_hunter_reticle_path(selection);
    
    let local_pos = if let Some(scene_t) = scene_transform {
        let scene_matrix = Mat4::from_scale_rotation_translation(scene_t.scale, scene_t.rotation, scene_t.translation);
        scene_matrix.inverse().transform_point3(world_position)
    } else {
        world_position
    };

    let transform = Transform::from_translation(local_pos);
    
    let child_entity = commands.spawn((
        transform,
        GlobalTransform::from(Transform::from_translation(world_position)),
        Visibility::default(),
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
    _scene_setup: &SceneSetup,
    scene_entity: Entity,
) {
    let crosshair_size = 0.5;
    let green = Color::srgb(0.0, 1.0, 0.0); // Matching green color as corner L-brackets
    let crosshair_path = UniversalPath::crosshair(Vec2::ZERO, crosshair_size, green);
    let child_entity = commands.spawn((
        Transform::IDENTITY,
        GlobalTransform::default(),
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
        GlobalTransform::default(),
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
    calibration_state: Res<State<CalibrationState>>,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    scene_entity_query: Query<Entity, With<SceneEntity>>,
) {
    if *calibration_state.get() != CalibrationState::On {
        return;
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
        spawn_center_crosshair(&mut commands, &scene_setup, scene_entity);
    }
}

// --- Calibration overlay update systems ---
fn update_projection_area_rectangle(
    scene_setup: Res<SceneSetup>,
    mut query: Query<(&mut Transform, &mut UniversalPath), With<ProjectionAreaRectangle>>,
) {
    if !scene_setup.is_changed() {
        return;
    }
    let rectangle_path = build_calibration_rectangle_path(scene_setup.scene.scene_dimension);
    for (mut transform, mut path) in query.iter_mut() {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::ONE;
        *path = rectangle_path.clone();
    }
}

fn update_center_crosshair(
    scene_setup: Res<SceneSetup>,
    mut query: Query<&mut Transform, With<CalibrationCenterCrosshair>>,
) {
    if !scene_setup.is_changed() {
        return;
    }
    for mut transform in query.iter_mut() {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::ONE;
    }
}