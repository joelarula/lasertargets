use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::path::{UniversalPath, PathSegment};
use common::scene::SceneSetup;
use crate::plugins::network::{MousePositionEvent, KeyboardInputEvent};
use crate::plugins::scene::SceneEntity;

pub struct CalibrationPlugin;

/// Resource to track calibration state (server singleton)
#[derive(Resource)]
pub struct CalibrationState {
    pub enabled: bool, // Global calibration state
    pub mouse_positions: std::collections::HashMap<u64, Vec3>, // Per-client mouse tracking
}

impl Default for CalibrationState {
    fn default() -> Self {
        Self {
            enabled: true, // Calibration enabled by default
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

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<CalibrationState>()
            .add_systems(PostStartup, initialize_calibration)
            .add_systems(Update, (
                handle_calibration_toggle,
                handle_mouse_position_updates,
                spawn_crosshairs_for_new_clients,
            ))
            .add_systems(Update, (
                update_crosshair_positions,
                cleanup_disconnected_clients,
            ));
    }
}

/// Initialize calibration on startup - spawn projection area rectangle
fn initialize_calibration(
    mut commands: Commands,
    calibration_state: Res<CalibrationState>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
) {
    info!("Initializing calibration system. Enabled: {}", calibration_state.enabled);
    
    // Spawn projection area rectangle if calibration is enabled
    if !calibration_state.enabled {
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

fn handle_calibration_toggle(
    mut keyboard_events: MessageReader<KeyboardInputEvent>,
    mut calibration_state: ResMut<CalibrationState>,
    mut commands: Commands,
    crosshair_query: Query<Entity, With<CalibrationCrosshair>>,
    projection_area_query: Query<Entity, With<ProjectionAreaRectangle>>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
) {
    for event in keyboard_events.read() {
        info!("Received keyboard event: key='{}' pressed={} client_id={}", event.key, event.pressed, event.client_id);
        
        if event.key == "F3" && event.pressed {
            info!("F3 pressed by client {}, current calibration enabled: {}", event.client_id, calibration_state.enabled);
            
            if calibration_state.enabled {
                // Deactivate calibration
                calibration_state.enabled = false;
                
                // Despawn all calibration entities
                for entity in crosshair_query.iter() {
                    commands.entity(entity).despawn();
                }
                for entity in projection_area_query.iter() {
                    commands.entity(entity).despawn();
                }
                
                info!("Calibration deactivated globally");
            } else {
                // Activate calibration
                calibration_state.enabled = true;
                
                // Spawn calibration elements
                spawn_calibration_elements(&mut commands, Vec3::ZERO, &scene_query, &scene_setup, &calibration_state);
                
                info!("Calibration activated globally");
            }
        }
    }
}

fn handle_mouse_position_updates(
    mut mouse_events: MessageReader<MousePositionEvent>,
    mut calibration_state: ResMut<CalibrationState>,
) {
    // Always track mouse positions from all clients
    for event in mouse_events.read() {
        if let Some(world_pos) = event.position {
            calibration_state.mouse_positions.insert(event.client_id, world_pos);
        } else {
            calibration_state.mouse_positions.remove(&event.client_id);
        }
    }
}

fn update_crosshair_positions(
    calibration_state: Res<CalibrationState>,
    scene_setup: Res<SceneSetup>,
    mut crosshair_query: Query<(&mut Transform, &mut GlobalTransform, &CalibrationCrosshair)>,
) {
    // Only update if calibration is enabled
    if !calibration_state.enabled {
        return;
    }
    
    let scene_y = scene_setup.scene.origin.translation.y;
    
    for (mut transform, mut global_transform, crosshair) in crosshair_query.iter_mut() {
        if let Some(world_pos) = calibration_state.mouse_positions.get(&crosshair.client_id) {
            // Invert Y axis around the scene center Y position
            let corrected_pos = Vec3::new(world_pos.x, 2.0 * scene_y - world_pos.y, world_pos.z);
            transform.translation = corrected_pos;
            *global_transform = GlobalTransform::from(Transform::from_translation(corrected_pos));
        }
    }
}

fn cleanup_disconnected_clients(
    mut connection_lost_events: MessageReader<ConnectionLostEvent>,
    mut calibration_state: ResMut<CalibrationState>,
) {
    for connection_lost in connection_lost_events.read() {
        let client_id = connection_lost.id;
        
        // Remove from mouse positions tracking
        calibration_state.mouse_positions.remove(&client_id);
        
        info!("Cleaned up mouse tracking for disconnected client {}", client_id);
    }
}

fn spawn_calibration_elements(
    commands: &mut Commands,
    world_position: Vec3,
    scene_query: &Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: &SceneSetup,
    calibration_state: &CalibrationState,
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
        for &client_id in calibration_state.mouse_positions.keys() {
            spawn_crosshair_at_position(commands, client_id, local_position);
        }
        
        // Spawn single projection area rectangle (shared for all clients)
        spawn_projection_area_rectangle(commands, &scene_setup, scene_entity, scene_transform);
    } else {
        warn!("No scene entity found to parent calibration elements to");
        
        // Fallback: spawn without parenting
        for &client_id in calibration_state.mouse_positions.keys() {
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
    _scene_transform: &Transform,
) {
    // Calculate rectangle size to fill the projector's FOV
    // Projector is at (0, 1.5, 0) with 60° FOV, looking at scene at (0, 3, -10)
    // Distance from projector to scene: ~10m
    // At 60° FOV, the projection width at 10m distance: 2 * 10 * tan(30°) ≈ 11.55m
    // Use a moderate size that shows well - 60% of max to have good margins
    let fov_degrees = 60.0f32;
    let projector_pos = Vec3::new(0.0, 1.5, 0.0);
    let scene_pos = scene_setup.scene.origin.translation;
    let distance = projector_pos.distance(scene_pos);
    let half_fov_rad = (fov_degrees / 2.0).to_radians();
    let max_width = 2.0 * distance * half_fov_rad.tan();
    let square_size = max_width * 0.7; // Use 80% of max for clear visibility with margins
    
    info!("Calculating projection area rectangle: distance={:.2}m, FOV={}°, max_width={:.2}m, using {:.2}m square", 
          distance, fov_degrees, max_width, square_size);
    
    // Position at scene center - this is the billboard/projection surface
    let center_world_pos = scene_setup.scene.origin.translation;
    
    info!("Spawning projection area rectangle at scene center {:?}", center_world_pos);
    
    // Create 4 separate corner segments with dwell 15 each
    let half_size = square_size / 2.0;
    let corners = [
        Vec2::new(-half_size, -half_size), // Bottom-left
        Vec2::new(half_size, -half_size),  // Bottom-right
        Vec2::new(half_size, half_size),   // Top-right
        Vec2::new(-half_size, half_size),  // Top-left
    ];
    
    let green = Color::srgb(0.0, 1.0, 0.0);
    let blank = Color::srgb(0.0, 0.0, 0.0);
    let corner_dwell = 15; // High dwell for sharp, bright corners
    
    // Create one segment per corner point
    let mut segments = Vec::new();
    for corner in &corners {
        let mut segment = common::path::PathSegment::empty();
        segment.push(corner.x, corner.y, blank, 15);
        segment.push(corner.x, corner.y, green, 25);
        segment.push(corner.x, corner.y, blank, 15);
        segments.push(segment);
    }
    
    let rectangle_universal_path = UniversalPath {
        segments,
    };
    
    // Spawn at world position in XY plane (flat, not rotated)
    let transform = Transform::from_translation(center_world_pos);
    
    let _rectangle_entity = commands.spawn((
        transform,
        GlobalTransform::from(transform),
        Visibility::default(),
        ProjectionAreaRectangle,
        rectangle_universal_path.clone(),
        common::path::PathRenderable::default(),
    )).id();
    
    info!("Spawned test square ({}m x {}m) at {:?}, {} segments", 
          square_size, square_size, center_world_pos, rectangle_universal_path.segments.len());
}

/// Spawn crosshairs for new clients when they first send mouse events
fn spawn_crosshairs_for_new_clients(
    mut mouse_events: MessageReader<MousePositionEvent>,
    calibration_state: Res<CalibrationState>,
    mut commands: Commands,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    crosshair_query: Query<&CalibrationCrosshair>,
) {
    if !calibration_state.enabled {
        return;
    }
    
    // Get existing crosshair client IDs
    let existing_clients: std::collections::HashSet<u64> = crosshair_query
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
        }
    }
}