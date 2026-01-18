use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneSetup;
use common::path::{UniversalPath, PathSegment};
use crate::dac::helios::{HeliosDacController, HeliosPoint, HELIOS_FLAGS_DEFAULT, HELIOS_MAX_COORD};
use std::sync::{Arc, Mutex};
use std::thread;

/// Shared buffer for laser points - double buffered for smooth rendering
#[derive(Resource, Clone)]
pub struct LaserPointBuffer {
    points: Arc<Mutex<Vec<HeliosPoint>>>,
}

impl Default for LaserPointBuffer {
    fn default() -> Self {
        Self {
            points: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Resource for managing the Helios DAC controller
#[derive(Resource)]
pub struct ProjectorDacController {
    pub controller: Option<HeliosDacController>,
    initialized: bool,
    thread_running: bool,
}

impl Default for ProjectorDacController {
    fn default() -> Self {
        Self {
            controller: None,
            initialized: false,
            thread_running: false,
        }
    }
}

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectorConfiguration {
            enabled: true, // Enable by default for testing
            ..Default::default()
        })
            .insert_resource(ProjectorDacController::default())
            .insert_resource(LaserPointBuffer::default())
            .add_systems(Startup, initialize_projector_dac)
            .add_systems(Update, update_projector)
            .add_systems(Update, update_point_buffer);
    }
}

/// Initialize the Helios DAC controller and start background rendering thread
fn initialize_projector_dac(
    mut dac_controller: ResMut<ProjectorDacController>,
    point_buffer: Res<LaserPointBuffer>,
) {
    info!("Initializing Helios DAC controller...");
    match HeliosDacController::new() {
        Ok(mut controller) => {
            info!("Helios DAC library loaded successfully");
            
            // Try to open devices with retries
            let max_retries = 3;
            let mut devices_opened = false;
            
            for attempt in 1..=max_retries {
                info!("Attempting to open Helios DAC devices (attempt {}/{})", attempt, max_retries);
                
                match controller.open_devices() {
                    Ok(num_devices) if num_devices > 0 => {
                        info!("✓ Helios DAC initialized: {} devices found and opened", num_devices);
                        devices_opened = true;
                        break;
                    }
                    Ok(num_devices) if num_devices == 0 => {
                        warn!("No Helios DAC devices found on attempt {}", attempt);
                        if attempt < max_retries {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                    Ok(num_devices) => {
                        error!("Unexpected device count {} on attempt {}", num_devices, attempt);
                        if attempt < max_retries {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                    Err(e) => {
                        error!("Failed to open Helios DAC devices on attempt {}: {}", attempt, e);
                        if attempt < max_retries {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            }
            
            if !devices_opened {
                error!("✗ Failed to open Helios DAC after {} attempts. Is the device connected?", max_retries);
                return;
            }
            
            // Verify device is actually responding with a status check
            match controller.get_status(0) {
                Ok(_) => {
                    info!("✓ DAC status check passed - device is responding");
                }
                Err(e) => {
                    error!("✗ DAC opened but status check failed: {}. Device may not be fully initialized.", e);
                    error!("   Attempting to continue anyway, but laser output may not work.");
                }
            }
            
            // Open the shutter to enable laser output
            if let Err(e) = controller.set_shutter(0, true) {
                warn!("Failed to open laser shutter: {}", e);
            } else {
                info!("✓ Laser shutter opened");
            }
            
            // Start background thread for continuous DAC output
            info!("✓ Starting DAC output thread...");
            start_dac_output_thread(controller, point_buffer.clone());
            
            dac_controller.thread_running = true;
            dac_controller.initialized = true;
            info!("✓ Projector initialization complete");
        }
        Err(e) => {
            error!("✗ Failed to initialize Helios DAC controller: {}", e);
        }
        Err(e) => {
            error!("✗ Failed to initialize Helios DAC controller: {}", e);
        }
    }
}

/// Start background thread that continuously sends frames to the DAC
fn start_dac_output_thread(controller: HeliosDacController, point_buffer: LaserPointBuffer) {
    thread::spawn(move || {
        info!("DAC output thread started");
        let pps = 15000; // Points per second - reduced for better galvo control
        let flags = 0; // Default looping mode - frame plays continuously until new frame arrives
        let mut frame_count = 0;
        let mut consecutive_errors = 0;
        let mut consecutive_write_failures = 0;
        let max_consecutive_errors = 100;
        let max_write_failures = 50;
        
        loop {
            // Get current points from buffer first
            let points = {
                let buffer = point_buffer.points.lock().unwrap();
                buffer.clone()
            };
            
            // Wait for DAC to be ready
            match controller.get_status(0) {
                Ok(true) => {
                    consecutive_errors = 0; // Reset error counter on success
                    
                    if !points.is_empty() {
                        // Send frame to DAC immediately when ready
                        match controller.write_frame_native(0, pps, flags, &points) {
                            Ok(_) => {
                                consecutive_write_failures = 0;
                                frame_count += 1;
                                if frame_count % 60 == 0 {
                                    info!("✓ DAC active: {} frames sent, current frame has {} points", frame_count, points.len());
                                }
                            }
                            Err(e) => {
                                consecutive_write_failures += 1;
                                if consecutive_write_failures == 1 || consecutive_write_failures % 10 == 0 {
                                    error!("✗ DAC write failed (failure #{}): {}. DAC may not be responding.", consecutive_write_failures, e);
                                }
                                
                                if consecutive_write_failures >= max_write_failures {
                                    error!("✗ CRITICAL: {} consecutive write failures. DAC connection appears dead. Thread exiting.", consecutive_write_failures);
                                    error!("   Please restart the server to reinitialize the DAC.");
                                    break;
                                }
                                
                                thread::sleep(std::time::Duration::from_millis(50));
                            }
                        }
                    } else {
                        // No points, send a blank frame to center
                        let blank = vec![HeliosPoint::blanked(2048, 2048)];
                        if let Err(e) = controller.write_frame_native(0, pps, flags, &blank) {
                            consecutive_write_failures += 1;
                            if consecutive_write_failures >= max_write_failures {
                                error!("✗ CRITICAL: DAC write failures on blank frames. Thread exiting.");
                                break;
                            }
                        } else {
                            consecutive_write_failures = 0;
                        }
                    }
                }
                Ok(false) => {
                    consecutive_errors = 0; // Reset on busy (not an error)
                    // DAC not ready yet - actively wait with minimal sleep
                    thread::sleep(std::time::Duration::from_micros(100));
                }
                Err(e) => {
                    consecutive_errors += 1;
                    if consecutive_errors == 1 {
                        error!("✗ DAC status check failed: {}. Is the Helios DAC connected and powered on?", e);
                    } else if consecutive_errors == 10 {
                        error!("✗ DAC status check failed 10 times. Verify USB connection and device power.");
                    } else if consecutive_errors % 100 == 0 {
                        error!("✗ DAC status check failed {} times. Connection appears dead.", consecutive_errors);
                    }
                    
                    // If too many consecutive errors, the DAC is probably not connected
                    if consecutive_errors >= max_consecutive_errors * 5 {
                        error!("✗ CRITICAL: {} consecutive status check failures. DAC appears disconnected. Thread exiting.", consecutive_errors);
                        error!("   Please check USB connection and restart the server.");
                        break;
                    }
                    
                    // Slow down polling on errors
                    if consecutive_errors > max_consecutive_errors {
                        thread::sleep(std::time::Duration::from_millis(100));
                    } else {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        }
        
        error!("✗ DAC output thread terminated due to connection failure.");
    });
}

fn update_projector(
    mut projector_config: ResMut<ProjectorConfiguration>,
    scene_setup: Res<SceneSetup>,
) {
    if projector_config.is_changed() || scene_setup.is_changed() {
        if projector_config.locked_to_scene {
            // Lock projector to scene center
            let scene_center = scene_setup.scene.origin.translation;
            let new_rotation = Transform::from_translation(projector_config.origin.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
        }
    }
}

/// Update the point buffer with current UniversalPath entities
/// Background thread will continuously send these points to the DAC
fn update_point_buffer(
    projector_config: Res<ProjectorConfiguration>,
    point_buffer: Res<LaserPointBuffer>,
    path_query: Query<(&UniversalPath, &GlobalTransform)>,
) {
    // Only update buffer if projector is enabled
    if !projector_config.enabled {
        return;
    }
    
    let path_count = path_query.iter().count();
    if path_count > 0 {
        info!("Update buffer: Found {} UniversalPath entities", path_count);
    }
    
    // Convert all paths to Helios points
    let mut helios_points = Vec::new();
    
    for (universal_path, global_transform) in path_query.iter() {
        info!("Converting path with {} segments at {:?}", 
              universal_path.segments.len(), global_transform.translation());
        
        let points = convert_universal_path_to_helios_points(
            universal_path,
            global_transform,
            &projector_config,
        );
        
        info!("Generated {} points from path", points.len());
        
        // Add transition blanking between shapes (if not the first shape)
        if !helios_points.is_empty() && !points.is_empty() {
            // Get the coordinates of the last point of previous shape and first point of new shape
            let (last_x, last_y) = {
                let last_prev: &HeliosPoint = helios_points.last().unwrap();
                (last_prev.x, last_prev.y)
            };
            let (first_x, first_y) = {
                let first_next: &HeliosPoint = &points[0];
                (first_next.x, first_next.y)
            };
            
            // Professional blanking technique between different shapes:
            // End Dwell: Ensure galvos stopped and laser fully off before leaving previous shape
            for _ in 0..15 {
                helios_points.push(HeliosPoint::blanked(last_x, last_y));
            }
            
            // Create blanked line with many blanked points between shapes
            let jump_steps = 60;
            for step in 1..jump_steps {
                let t = step as f32 / jump_steps as f32;
                let interp_x = (last_x as f32 * (1.0 - t) + first_x as f32 * t) as u16;
                let interp_y = (last_y as f32 * (1.0 - t) + first_y as f32 * t) as u16;
                // Explicitly create blanked points (laser off, r=g=b=0)
                helios_points.push(HeliosPoint::blanked(interp_x, interp_y));
            }
            
            // Start Dwell: Ensure galvos settled and laser stays off before turning ON
            for _ in 0..15 {
                helios_points.push(HeliosPoint::blanked(first_x, first_y));
            }
        }
        
        helios_points.extend(points);
    }
    
    info!("Total points collected: {}", helios_points.len());
    
    // Update the shared buffer - background thread will continuously send it to DAC
    if let Ok(mut buffer) = point_buffer.points.lock() {
        *buffer = helios_points;
    }
}

/// Convert a UniversalPath to Helios points using projector coordinate transformation
fn convert_universal_path_to_helios_points(
    universal_path: &UniversalPath,
    global_transform: &GlobalTransform,
    projector_config: &ProjectorConfiguration,
) -> Vec<HeliosPoint> {
    let mut points = Vec::new();
    
    info!("Converting UniversalPath with {} segments at position {:?}", 
          universal_path.segments.len(), global_transform.translation());
    
    for (i, segment) in universal_path.segments.iter().enumerate() {
        let segment_points = convert_path_segment_to_helios_points(
            segment,
            global_transform,
            projector_config,
        );
        info!("Segment {}: {} points", i, segment_points.len());
        
        // Add smooth blanked galvo transition between segments
        if !points.is_empty() && !segment_points.is_empty() {
            let (last_x, last_y) = {
                let last_prev: &HeliosPoint = points.last().unwrap();
                (last_prev.x, last_prev.y)
            };
            let (first_x, first_y) = {
                let first_next: &HeliosPoint = &segment_points[0];
                (first_next.x, first_next.y)
            };
            
            // End dwell: Ensure galvos stopped and laser fully off before leaving previous segment
            for _ in 0..12 {
                points.push(HeliosPoint::blanked(last_x, last_y));
            }
            
            // Create blanked line with many blanked points for smooth transition between segments
            let jump_steps = 50;
            for step in 1..jump_steps {
                let t = step as f32 / jump_steps as f32;
                let interp_x = (last_x as f32 * (1.0 - t) + first_x as f32 * t) as u16;
                let interp_y = (last_y as f32 * (1.0 - t) + first_y as f32 * t) as u16;
                // Explicitly create blanked points (laser off: r=0, g=0, b=0, i=0)
                points.push(HeliosPoint::blanked(interp_x, interp_y));
            }
            
            // Start dwell: Ensure galvos settled and laser stays off before turning ON
            for _ in 0..12 {
                points.push(HeliosPoint::blanked(first_x, first_y));
            }
        }
        
        points.extend(segment_points);
    }
    
    info!("Total points from UniversalPath: {}", points.len());
    points
}

/// Convert a single path segment to Helios points
fn convert_path_segment_to_helios_points(
    segment: &PathSegment,
    global_transform: &GlobalTransform,
    projector_config: &ProjectorConfiguration,
) -> Vec<HeliosPoint> {
    let mut helios_points = Vec::new();
    
    if segment.points.is_empty() {
        return helios_points;
    }
    
    let mut prev_point: Option<&common::path::PathPoint> = None;
    
    // Transform and convert all points directly
    for point in &segment.points {
        // Check if we need interpolation between previous and current point
        if let Some(prev) = prev_point {
            let dx = point.x - prev.x;
            let dy = point.y - prev.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance > 20.0 {
                // Add interpolated points between distant points
                let num_interp = (distance / 10.0).ceil() as usize;
                for i in 1..num_interp {
                    let t = i as f32 / num_interp as f32;
                    let interp_x = prev.x + dx * t;
                    let interp_y = prev.y + dy * t;
                    
                    let world_pos = Vec3::new(interp_x, interp_y, 0.0);
                    let transformed_world_pos = global_transform.transform_point(world_pos);
                    
                    if let Some((x, y)) = world_to_projector_coordinates(
                        transformed_world_pos,
                        projector_config,
                    ) {
                        // Use color from previous point with dwell=1 (single point)
                        helios_points.push(HeliosPoint::new(
                            x,
                            y,
                            prev.r,
                            prev.g,
                            prev.b,
                            255,
                        ));
                    }
                }
            }
        }
        
        let world_pos = Vec3::new(point.x, point.y, 0.0);
        let transformed_world_pos = global_transform.transform_point(world_pos);
        
        if let Some((x, y)) = world_to_projector_coordinates(
            transformed_world_pos,
            projector_config,
        ) {
            // Add blank points at the start of segment to ensure laser is off on arrival
            if helios_points.is_empty() {
                for _ in 0..3 {
                    helios_points.push(HeliosPoint::blanked(x, y));
                }
            }
            
            // Dwell value directly controls repetition count (minimum 1 point)
            let repeat_count = if point.dwell == 0 { 1 } else { point.dwell as usize };
            
            for _ in 0..repeat_count {
                helios_points.push(HeliosPoint::new(
                    x,
                    y,
                    point.r,
                    point.g,
                    point.b,
                    255, // Full intensity
                ));
            }
        }
        
        prev_point = Some(point);
    }
    
    // Add blank points at the end of segment for smooth fade-out
    if !helios_points.is_empty() {
        let (last_x, last_y) = {
            let last = helios_points.last().unwrap();
            (last.x, last.y)
        };
        for _ in 0..3 {
            helios_points.push(HeliosPoint::blanked(last_x, last_y));
        }
    }
    
    helios_points
}

/// Transform world coordinates to projector coordinates using perspective projection
fn world_to_projector_coordinates(
    world_pos: Vec3,
    projector_config: &ProjectorConfiguration,
) -> Option<(u16, u16)> {
    // Create projector view matrix
    let projector_transform = Mat4::from_scale_rotation_translation(
        projector_config.origin.scale,
        projector_config.origin.rotation,
        projector_config.origin.translation,
    );
    
    // Transform world position to projector local space
    let local_pos = projector_transform.inverse().transform_point3(world_pos);
    
    // Check if point is in front of projector
    // In projector local space, negative Z means the point is in the direction the projector is looking
    if local_pos.z >= 0.0 {
        warn!("Point {:?} behind projector (local_z: {})", world_pos, local_pos.z);
        return None; // Behind projector
    }
    
    // Use absolute value for projection calculations
    let distance = local_pos.z.abs();
    
    // Apply perspective projection
    let fov_rad = projector_config.angle.to_radians();
    let half_fov = fov_rad / 2.0;
    
    // Project to normalized device coordinates [-1, 1]
    let projected_x = (local_pos.x / distance) / half_fov.tan();
    let projected_y = (local_pos.y / distance) / half_fov.tan();
    
    // Clip to visible range
    if projected_x.abs() > 1.0 || projected_y.abs() > 1.0 {
        warn!("Point {:?} outside FOV (projected: {}, {})", world_pos, projected_x, projected_y);
        return None; // Outside field of view
    }
    
    // Convert normalized coordinates [-1, 1] to DAC coordinates [0, 4095]
    // -1 maps to 0, 0 maps to 2048 (center), +1 maps to 4095
    let x = ((projected_x + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    let y = ((projected_y + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    
    Some((x, y))
}
