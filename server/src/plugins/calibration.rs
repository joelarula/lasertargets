/// Marker component for the projection area rectangle entity (for calibration overlay)
#[derive(Component, Debug, Clone, Copy)]
pub struct ProjectionAreaRectangle;
/// Component to mark persistent corner marker entities
#[derive(Component)]
pub struct CalibrationCornerMarker {
    pub index: usize, // 0=BL, 1=BR, 2=TR, 3=TL
}

/// System to spawn persistent corner markers (only if not present)
fn ensure_corner_markers_exist(mut commands: Commands, scene_setup: Res<SceneSetup>, query: Query<&CalibrationCornerMarker>) {
    let mut existing = [false; 4];
    for marker in query.iter() {
        if marker.index < 4 {
            existing[marker.index] = true;
        }
    }
    let scene_dimensions = scene_setup.scene.scene_dimension;
    let half_width = scene_dimensions.x / 2.0;
    let half_height = scene_dimensions.y / 2.0;
    let corners = [
        Vec2::new(-half_width, -half_height), // Bottom-left
        Vec2::new(half_width, -half_height),  // Bottom-right
        Vec2::new(half_width, half_height),   // Top-right
        Vec2::new(-half_width, half_height),  // Top-left
    ];
    let color = Color::srgb(0.0, 1.0, 0.0); // Green
    for (i, corner) in corners.iter().enumerate() {
        if !existing[i] {
            let mut segment = PathSegment::empty();
            segment.push(corner.x, corner.y, color, 0);
            let marker_path = UniversalPath { segments: vec![segment] };
            let origin = &scene_setup.scene.origin;
            let transform = Transform {
                translation: origin.translation,
                rotation: origin.rotation,
                scale: origin.scale,
            };
            commands.spawn((
                transform,
                GlobalTransform::from(transform),
                Visibility::default(),
                CalibrationPath,
                CalibrationCornerMarker { index: i },
                marker_path,
                common::path::PathRenderable::default(),
            ));
        }
    }
}

/// System to update corner marker positions on scene change
fn update_corner_marker_positions(scene_setup: Res<SceneSetup>, mut query: Query<(&CalibrationCornerMarker, &mut Transform, &mut UniversalPath)>) {
    let scene_dimensions = scene_setup.scene.scene_dimension;
    let half_width = scene_dimensions.x / 2.0;
    let half_height = scene_dimensions.y / 2.0;
    let corners = [
        Vec2::new(-half_width, -half_height), // Bottom-left
        Vec2::new(half_width, -half_height),  // Bottom-right
        Vec2::new(half_width, half_height),   // Top-right
        Vec2::new(-half_width, half_height),  // Top-left
    ];
    for (marker, mut transform, mut path) in query.iter_mut() {
        let corner = corners[marker.index];
        // Update path point
        if let Some(segment) = path.segments.get_mut(0) {
            if let Some(point) = segment.points.get_mut(0) {
                point.x = corner.x;
                point.y = corner.y;
            }
        }
        // Update transform (scene origin)
        let origin = &scene_setup.scene.origin;
        transform.translation = origin.translation;
        transform.rotation = origin.rotation;
        transform.scale = origin.scale;
    }
}
use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::path::{UniversalPath, PathSegment};
use common::scene::{SceneSetup, SceneSystemSet};
use common::state::CalibrationState;
use crate::plugins::network::MousePositionEvent;

pub struct CalibrationPlugin;

#[derive(Resource, Debug, Clone)]
pub struct CalibrationSceneSnapshot {
    pub scene_dimension: Vec2,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for CalibrationSceneSnapshot {
    fn default() -> Self {
        Self {
            scene_dimension: Vec2::ZERO,
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

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
            .init_resource::<CalibrationSceneSnapshot>()
            .add_systems(PostStartup, initialize_calibration)
            .add_systems(OnEnter(CalibrationState::On), spawn_calibration_on_enable)
            .add_systems(OnExit(CalibrationState::On), cleanup_calibration_on_disable)
            .add_systems(Update, (
                handle_mouse_position_updates,
                spawn_crosshairs_for_new_clients,
            ).chain())
            .add_systems(Update, (
                update_crosshair_positions,
                cleanup_disconnected_clients,
                refresh_calibration_visuals_on_scene_change,
            ).after(SceneSystemSet))
            .add_systems(OnEnter(CalibrationState::On), ensure_corner_markers_exist)
            .add_systems(Update, update_corner_marker_positions);
    }
}

fn spawn_calibration_on_enable(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
) {
    info!("Entering CalibrationState::On");
    // Corner markers are now persistent and managed by ensure_corner_markers_exist system
    // Always spawn red center crosshair
    spawn_center_crosshair(&mut commands, &scene_setup);
    // Always spawn green point at center (client_id=0)
    let center = scene_setup.scene.origin.translation;
    spawn_crosshair_at_position(&mut commands, 0, center, &scene_setup);

/// Spawn visible markers at each corner of the scene for calibration
fn spawn_corner_markers(commands: &mut Commands, scene_setup: &SceneSetup) {
    let scene_dimensions = scene_setup.scene.scene_dimension;
    let half_width = scene_dimensions.x / 2.0;
    let half_height = scene_dimensions.y / 2.0;
    let corners = [
        Vec2::new(-half_width, -half_height), // Bottom-left
        Vec2::new(half_width, -half_height),  // Bottom-right
        Vec2::new(half_width, half_height),   // Top-right
        Vec2::new(-half_width, half_height),  // Top-left
    ];
    let color = Color::srgb(1.0, 0.0, 0.0); // Red for visibility
    for corner in corners.iter() {
        let mut segment = PathSegment::empty();
        segment.push(corner.x, corner.y, color, 0);
        // Optionally, add a small cross or dot at each corner
        let marker_path = UniversalPath { segments: vec![segment] };
        let origin = &scene_setup.scene.origin;
        let transform = Transform {
            translation: origin.translation,
            rotation: origin.rotation,
            scale: origin.scale,
        };
        commands.spawn((
            transform,
            GlobalTransform::from(transform),
            Visibility::default(),
            CalibrationPath,
            marker_path,
            common::path::PathRenderable::default(),
        ));
    }
}
}

fn cleanup_calibration_on_disable(
    mut commands: Commands,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    crosshair_query: Query<Entity, With<CalibrationCrosshair>>,
    path_query: Query<Entity, With<CalibrationPath>>,
) {
    info!("Exiting CalibrationState::On");
    for entity in rectangle_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in center_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in crosshair_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in path_query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Initialize calibration on startup - spawn projection area rectangle
fn initialize_calibration(
    mut commands: Commands,
    calibration_state: Res<State<CalibrationState>>,
    scene_setup: Res<SceneSetup>,
) {
    info!("Initializing calibration system. State: {:?}", calibration_state.get());
    
    // Spawn projection area rectangle if calibration is enabled
    if *calibration_state.get() == CalibrationState::Off {
        info!("Calibration disabled, skipping projection area rectangle spawn");
        return;
    }
    
    info!("Calibration visuals: only corner markers will be shown");
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

fn update_crosshair_positions(
    calibration_state: Res<State<CalibrationState>>,
    calibration_data: Res<CalibrationData>,
    mut crosshair_query: Query<(&mut Transform, &mut GlobalTransform, &CalibrationCrosshair)>,
) {
    // Only update if calibration is enabled
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    for (mut transform, mut global_transform, crosshair) in crosshair_query.iter_mut() {
        if let Some(world_pos) = calibration_data.mouse_positions.get(&crosshair.client_id) {
            transform.translation = *world_pos;
            *global_transform = GlobalTransform::from(Transform::from_translation(*world_pos));
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

fn spawn_calibration_elements(
    commands: &mut Commands,
    world_position: Vec3,
    scene_setup: &SceneSetup,
    calibration_data: &CalibrationData,
) {
    // Convert world position to local position relative to scene
    let origin = &scene_setup.scene.origin;
    let scene_matrix = Mat4::from_scale_rotation_translation(
        origin.scale,
        origin.rotation,
        origin.translation,
    );
    let local_position = scene_matrix.inverse().transform_point3(world_position);

    // Spawn crosshairs for all currently connected clients
    for &client_id in calibration_data.mouse_positions.keys() {
        spawn_crosshair_at_position(commands, client_id, local_position, scene_setup);
    }

    // Do not spawn rectangle
}

/// Spawn crosshair at specific position
fn spawn_crosshair_at_position(
    commands: &mut Commands,
    client_id: u64,
    world_position: Vec3,
    scene_setup: &SceneSetup,
) {
    // Make crosshair size dynamic: 10% of the smaller scene dimension, min 0.2m, max 2.0m
    let scene_dim = scene_setup.scene.scene_dimension;
    let min_dim = scene_dim.x.min(scene_dim.y);
    let crosshair_size = (min_dim * 0.1).clamp(0.2, 2.0);
    let half_size = crosshair_size / 2.0;
    let blue = Color::srgb(0.0, 0.0, 0.5); // Blue color to distinguish from red center

    // Horizontal line segment
    let mut h_segment = common::path::PathSegment::empty();
    h_segment.push(-half_size, 0.0, blue, 0);
    h_segment.push(0.0, 0.0, blue, 0);
    h_segment.push(half_size, 0.0, blue, 0);

    // Vertical line segment
    let mut v_segment = common::path::PathSegment::empty();
    v_segment.push(0.0, -half_size, blue, 0);
    v_segment.push(0.0, 0.0, blue, 0);
    v_segment.push(0.0, half_size, blue, 0);

    let crosshair_universal_path = UniversalPath {
        segments: vec![h_segment, v_segment],
    };

    // Spawn at world position in XY plane (flat, not rotated)
    let transform = Transform::from_translation(world_position);

    commands.spawn((
        transform,
        GlobalTransform::from(transform),
        Visibility::default(),
        CalibrationPath,
        CalibrationCrosshair { client_id },
        crosshair_universal_path,
        common::path::PathRenderable::default(),
    ));

    info!("Spawned mouse crosshair for client {} at world position {:?}, size {}m", client_id, world_position, crosshair_size);
}

/// Spawn a red crosshair at the scene center (projection surface)
fn spawn_center_crosshair(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
) {
    // Make crosshair size dynamic: 10% of the smaller scene dimension, min 0.2m, max 2.0m
    let scene_dim = scene_setup.scene.scene_dimension;
    let min_dim = scene_dim.x.min(scene_dim.y);
    let crosshair_size = (min_dim * 0.1).clamp(0.2, 2.0);
    let half_size = crosshair_size / 2.0;

    // Position at scene center - this is the billboard/projection surface
    let center_world_pos = scene_setup.scene.origin.translation;

    info!("Spawning center crosshair at scene center (projection surface) {:?}, size {}m", center_world_pos, crosshair_size);

    // Create two segments: horizontal line and vertical line
    let red = Color::srgb(0.5, 0.0, 0.0); // Reduced red intensity
    let mut h_segment = common::path::PathSegment::empty();

    // Horizontal line: left through center to right
    h_segment.push(-half_size, 0.0, red, 0);
    h_segment.push(0.0, 0.0, red, 0);
    h_segment.push(half_size, 0.0, red, 0);

    let mut v_segment = common::path::PathSegment::empty();

    // Vertical line: bottom through center to top
    v_segment.push(0.0, -half_size, red, 0);
    v_segment.push(0.0, 0.0, red, 0);
    v_segment.push(0.0, half_size, red, 0);

    let crosshair_universal_path = UniversalPath {
        segments: vec![h_segment, v_segment],
    };

    // Spawn at world position in XY plane (flat, not rotated)
    let transform = Transform::from_translation(center_world_pos);

    commands.spawn((
        transform,
        GlobalTransform::from(transform),
        Visibility::default(),
        CalibrationPath,
        CalibrationCenterCrosshair,
        crosshair_universal_path,
        common::path::PathRenderable::default(),
    ));

    info!("Spawned red crosshair ({}m length) with 1 segment", crosshair_size);
}

/// Spawn projection area rectangle at scene center (projection surface)

/// Spawn crosshairs for new clients when they first send mouse events
fn spawn_crosshairs_for_new_clients(
    mut mouse_events: MessageReader<MousePositionEvent>,
    calibration_state: Res<State<CalibrationState>>,
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
    crosshair_query: Query<&CalibrationCrosshair>,
) {
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    // Get existing crosshair client IDs
    let mut existing_clients: std::collections::HashSet<u64> = crosshair_query
        .iter()
        .map(|crosshair| crosshair.client_id)
        .collect();
    
    // Check for new clients in mouse events
    for event in mouse_events.read() {
        let client_id = event.client_id;
        
        if !existing_clients.contains(&client_id) {
            info!("New client {} detected, spawning crosshair at {:?}", 
                  client_id, event.position);
            
            // Spawn crosshair at the mouse position (or scene center if None)
            let world_position = event.position
                .unwrap_or(scene_setup.scene.origin.translation);
            
            spawn_crosshair_at_position(&mut commands, client_id, world_position, &scene_setup);
            existing_clients.insert(client_id);
        }
    }
}

fn refresh_calibration_visuals_on_scene_change(
    mut commands: Commands,
    calibration_state: Res<State<CalibrationState>>,
    scene_setup: Res<SceneSetup>,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_crosshair_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    mut snapshot: ResMut<CalibrationSceneSnapshot>,
) {
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }

    let scene_config = &scene_setup.scene;
    let should_update = snapshot.scene_dimension != scene_config.scene_dimension
        || snapshot.translation != scene_config.origin.translation
        || snapshot.rotation != scene_config.origin.rotation
        || snapshot.scale != scene_config.origin.scale;

    if !should_update {
        return;
    }

    for entity in rectangle_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in center_crosshair_query.iter() {
        commands.entity(entity).despawn();
    }
    // Corner markers are now persistent and updated by update_corner_marker_positions system
    // Always spawn red center crosshair
    spawn_center_crosshair(&mut commands, &scene_setup);
    // Always spawn green point at center (client_id=0)
    let center = scene_setup.scene.origin.translation;
    spawn_crosshair_at_position(&mut commands, 0, center, &scene_setup);
    snapshot.scene_dimension = scene_config.scene_dimension;
    snapshot.translation = scene_config.origin.translation;
    snapshot.rotation = scene_config.origin.rotation;
    snapshot.scale = scene_config.origin.scale;
}