use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::path::{UniversalPath, PathSegment};
use common::scene::{SceneEntity, SceneSetup};
use common::state::CalibrationState;
use crate::plugins::network::{MousePositionEvent, KeyboardInputEvent};


pub struct CalibrationPlugin;

#[derive(Resource, Debug, Clone)]
pub struct CalibrationSceneSnapshot {
    pub scene_dimension: UVec2,
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for CalibrationSceneSnapshot {
    fn default() -> Self {
        Self {
            scene_dimension: UVec2::ZERO,
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
            ));
    }
}

fn spawn_calibration_on_enable(
    mut commands: Commands,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
) {
    if let Ok((scene_entity, scene_transform)) = scene_query.single() {
        spawn_projection_area_rectangle(&mut commands, &scene_setup, scene_entity, scene_transform);
        spawn_center_crosshair(&mut commands, &scene_setup);
    }
}

fn cleanup_calibration_on_disable(
    mut commands: Commands,
    rectangle_query: Query<Entity, With<ProjectionAreaRectangle>>,
    center_query: Query<Entity, With<CalibrationCenterCrosshair>>,
    crosshair_query: Query<Entity, With<CalibrationCrosshair>>,
    path_query: Query<Entity, With<CalibrationPath>>,
) {
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
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
) {
    info!("Initializing calibration system. State: {:?}", calibration_state.get());
    
    // Spawn projection area rectangle if calibration is enabled
    if *calibration_state.get() == CalibrationState::Off {
        info!("Calibration disabled, skipping projection area rectangle spawn");
        return;
    }
    
    if let Some((scene_entity, scene_transform)) = scene_query.iter().next() {
        info!("Found scene entity, spawning projection area rectangle and center crosshair");
        spawn_projection_area_rectangle(&mut commands, &scene_setup, scene_entity, scene_transform);
        spawn_center_crosshair(&mut commands, &scene_setup);
    } else {
        warn!("No scene entity found during calibration initialization");
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
    scene_setup: Res<SceneSetup>,
    mut crosshair_query: Query<(&mut Transform, &mut GlobalTransform, &CalibrationCrosshair)>,
) {
    // Only update if calibration is enabled
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    let scene_y = scene_setup.scene.origin.translation.y;
    
    for (mut transform, mut global_transform, crosshair) in crosshair_query.iter_mut() {
        if let Some(world_pos) = calibration_data.mouse_positions.get(&crosshair.client_id) {
            // Invert Y axis around the scene center Y position
            let corrected_pos = Vec3::new(world_pos.x, 2.0 * scene_y - world_pos.y, world_pos.z);
            transform.translation = corrected_pos;
            *global_transform = GlobalTransform::from(Transform::from_translation(corrected_pos));
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
    scene_query: &Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: &SceneSetup,
    calibration_data: &CalibrationData,
) {
    // Find the scene entity to parent the calibration elements to
    if let Ok((scene_entity, scene_transform)) = scene_query.single() {
        // Convert world position to local position relative to scene
        let scene_matrix = Mat4::from_scale_rotation_translation(
            scene_transform.scale, 
            scene_transform.rotation, 
            scene_transform.translation
        );
        let local_position = scene_matrix.inverse().transform_point3(world_position);
        
        // Spawn crosshairs for all currently connected clients
        for &client_id in calibration_data.mouse_positions.keys() {
            spawn_crosshair_at_position(commands, client_id, local_position);
        }
        
        // Spawn single projection area rectangle (shared for all clients)
        spawn_projection_area_rectangle(commands, &scene_setup, scene_entity, scene_transform);
    } else {
        warn!("No scene entity found to parent calibration elements to");
        
        // Fallback: spawn without parenting
        for &client_id in calibration_data.mouse_positions.keys() {
            spawn_crosshair_at_position(commands, client_id, world_position);
        }
        spawn_projection_area_rectangle(commands, &scene_setup, Entity::PLACEHOLDER, &Transform::IDENTITY);
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
    let blank = Color::srgb(0.0, 0.0, 0.0); // Black for blanking
    
    // Create single segment with crosshair points
    let mut segment = common::path::PathSegment::empty();
    
    // Draw full horizontal line: left through center to right
    segment.push(-half_size, 0.0, blue, 3);
    segment.push(0.0, 0.0, blue, 2); // Center point
    segment.push(half_size, 0.0, blue, 3);
    segment.push(half_size, 0.0, blank, 3);
    
    segment.push(0.0, -half_size, blank, 3); // Blank transition
    
    // Draw full vertical line: bottom through center to top
    segment.push(0.0, -half_size, blue, 3);
    segment.push(0.0, 0.0, blue, 2); // Center point
    segment.push(0.0, half_size, blue, 3);
    
    let crosshair_universal_path = UniversalPath {
        segments: vec![segment],
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
                                                   
    // Create single segment with crosshair points
    let red = Color::srgb(0.5, 0.0, 0.0); // Reduced red intensity
    let blank = Color::srgb(0.0, 0.0, 0.0); // Black for blanking
    let mut segment = common::path::PathSegment::empty();
    
    // Draw full horizontal line: left through center to right
    segment.push(-half_size, 0.0, red, 5);
    segment.push(0.0, 0.0, red, 3); // Center point
    segment.push(half_size, 0.0, red, 5);
    segment.push(half_size, 0.0, blank, 3);

    segment.push(0.0, -half_size, blank, 5); // Blank at start of vertical
    
    // Draw full vertical line: bottom through center to top
    segment.push(0.0, -half_size, red, 5);
    segment.push(0.0, 0.0, red, 3); // Center point
    segment.push(0.0, half_size, red, 5);
    segment.push(half_size, 0.0, blank, 3);
    
    let crosshair_universal_path = UniversalPath {
        segments: vec![segment],
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
    _parent_entity: Entity,
    scene_transform: &Transform,
) {
    let scene_dimensions = scene_setup.scene.scene_dimension.as_vec2();
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
    let blank = Color::srgb(0.0, 0.0, 0.0);
    let corner_dwell = 12;

    let mut segments = Vec::new();
    for corner in &corners {
        let mut segment = PathSegment::empty();
        segment.push(corner.x, corner.y, blank, corner_dwell);
        segment.push(corner.x, corner.y, green, corner_dwell * 2);
        segment.push(corner.x, corner.y, blank, corner_dwell);
        segments.push(segment);
    }

    let rectangle_universal_path = UniversalPath {
        segments,
    };

    let transform = Transform {
        translation: scene_transform.translation,
        rotation: scene_transform.rotation,
        scale: scene_transform.scale,
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
        scene_transform.translation,
        rectangle_universal_path.segments.len()
    );
}

/// Spawn crosshairs for new clients when they first send mouse events
fn spawn_crosshairs_for_new_clients(
    mut mouse_events: MessageReader<MousePositionEvent>,
    calibration_state: Res<State<CalibrationState>>,
    mut commands: Commands,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
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
            let world_position = event.position.unwrap_or_else(|| {
                scene_query.single()
                    .map(|(_, transform)| transform.translation)
                    .unwrap_or(Vec3::ZERO)
            });
            
            spawn_crosshair_at_position(&mut commands, client_id, world_position);
            existing_clients.insert(client_id);
        }
    }
}

fn refresh_calibration_visuals_on_scene_change(
    mut commands: Commands,
    calibration_state: Res<State<CalibrationState>>,
    scene_setup: Res<SceneSetup>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
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

    if let Some((scene_entity, scene_transform)) = scene_query.iter().next() {
        spawn_projection_area_rectangle(&mut commands, &scene_setup, scene_entity, scene_transform);
        spawn_center_crosshair(&mut commands, &scene_setup);
    }

    snapshot.scene_dimension = scene_config.scene_dimension;
    snapshot.translation = scene_config.origin.translation;
    snapshot.rotation = scene_config.origin.rotation;
    snapshot.scale = scene_config.origin.scale;
}