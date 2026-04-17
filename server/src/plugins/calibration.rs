use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::path::{UniversalPath, PathSegment};
use common::scene::{SceneSetup, SceneSystemSet};
use common::state::CalibrationState;
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
            .add_systems(OnEnter(CalibrationState::On), spawn_calibration_overlays)
            .add_systems(OnExit(CalibrationState::On), despawn_calibration_overlays)
            .add_systems(Update, (
                handle_mouse_position_updates,
                spawn_crosshairs_for_new_clients,
            ).chain())
            .add_systems(Update, (
                update_crosshair_positions,
                cleanup_disconnected_clients,
            ).after(SceneSystemSet))
            .add_systems(Update, update_projection_area_rectangle)
            .add_systems(Update, update_center_crosshair);
    }
}


fn despawn_calibration_overlays(
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



/// Spawn crosshair at specific position
fn spawn_crosshair_at_position(
    commands: &mut Commands,
    client_id: u64,
    world_position: Vec3,
) {
    let crosshair_size = 0.5; // 0.5m crosshair (same as center)
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
    
    info!("Spawned mouse crosshair for client {} at world position {:?}", client_id, world_position);
}

/// Spawn a red crosshair at the scene center (projection surface)
fn spawn_center_crosshair(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
) {
    let crosshair_size = 0.5; // 0.5m crosshair (0.25m in each direction)
    let half_size = crosshair_size / 2.0;
    
    // Position at scene center - this is the billboard/projection surface
    let center_world_pos = scene_setup.scene.origin.translation;
    
    info!("Spawning center crosshair at scene center (projection surface) {:?}", center_world_pos);
                                                   
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
fn spawn_projection_area_rectangle(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
) {
    let scene_dimensions = scene_setup.scene.scene_dimension;
    let half_width = scene_dimensions.x / 2.0;
    let half_height = scene_dimensions.y / 2.0;

    info!(
        "Spawning scene corner markers: width={:.2}, height={:.2}",
        scene_dimensions.x, scene_dimensions.y
    );

    let corners = [
        Vec2::new(-half_width, -half_height), // Bottom-left
        Vec2::new(half_width, -half_height),  // Bottom-right
        Vec2::new(half_width, half_height),   // Top-right
        Vec2::new(-half_width, half_height),  // Top-left
    ];

    let green = Color::srgb(0.0, 1.0, 0.0);

    let mut segments = Vec::new();
    for corner in &corners {
        let mut segment = PathSegment::empty();
        segment.push(corner.x, corner.y, green, 0);
        segments.push(segment);
    }

    let rectangle_universal_path = UniversalPath {
        segments,
    };

    let origin = &scene_setup.scene.origin;
    let transform = Transform {
        translation: origin.translation,
        rotation: origin.rotation,
        scale: origin.scale,
    };

    let _rectangle_entity = commands.spawn((
        transform,
        GlobalTransform::from(transform),
        Visibility::default(),
        CalibrationPath,
        ProjectionAreaRectangle,
        rectangle_universal_path.clone(),
        common::path::PathRenderable::default(),
    )).id();

    info!(
        "Spawned scene corner markers at {:?}, {} segments",
        origin.translation,
        rectangle_universal_path.segments.len()
    );
}

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
            
            spawn_crosshair_at_position(&mut commands, client_id, world_position);
            existing_clients.insert(client_id);
        }
    }
}


/// Spawns overlays only if not already present (called on entering calibration state)
fn spawn_calibration_overlays(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
) {
    info!("Entering CalibrationState::On");
    if rectangle_query.iter().next().is_none() {
        spawn_projection_area_rectangle(&mut commands, &scene_setup);
    }
    if center_query.iter().next().is_none() {
        spawn_center_crosshair(&mut commands, &scene_setup);
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
    let scene_dimensions = scene_setup.scene.scene_dimension;
    let half_width = scene_dimensions.x / 2.0;
    let half_height = scene_dimensions.y / 2.0;
    let corners = [
        Vec2::new(-half_width, -half_height),
        Vec2::new(half_width, -half_height),
        Vec2::new(half_width, half_height),
        Vec2::new(-half_width, half_height),
    ];
    for (mut transform, mut path) in query.iter_mut() {
        let green = Color::srgb(0.0, 1.0, 0.0);
        let mut segments = Vec::new();
        for corner in &corners {
            let mut segment = PathSegment::empty();
            segment.push(corner.x, corner.y, green, 0);
            segments.push(segment);
        }
        path.segments = segments;
        let origin = &scene_setup.scene.origin;
        transform.translation = origin.translation;
        transform.rotation = origin.rotation;
        transform.scale = origin.scale;
    }
}

fn update_center_crosshair(
    scene_setup: Res<SceneSetup>,
    mut query: Query<(&mut Transform, &mut UniversalPath), With<CalibrationCenterCrosshair>>,
) {
    if !scene_setup.is_changed() {
        return;
    }
    let crosshair_size = 0.5;
    let half_size = crosshair_size / 2.0;
    let center_world_pos = scene_setup.scene.origin.translation;
    for (mut transform, mut path) in query.iter_mut() {
        let red = Color::srgb(0.5, 0.0, 0.0);
        let mut h_segment = PathSegment::empty();
        h_segment.push(-half_size, 0.0, red, 0);
        h_segment.push(0.0, 0.0, red, 0);
        h_segment.push(half_size, 0.0, red, 0);
        let mut v_segment = PathSegment::empty();
        v_segment.push(0.0, -half_size, red, 0);
        v_segment.push(0.0, 0.0, red, 0);
        v_segment.push(0.0, half_size, red, 0);
        path.segments = vec![h_segment, v_segment];
        transform.translation = center_world_pos;
    }
}