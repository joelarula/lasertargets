use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneEntity;
use common::scene::SceneSetup;
use common::path::{UniversalPath, PathSegment};
use crate::plugins::calibration::CalibrationPath;
use crate::dac::helios::{HeliosDacController, HeliosPoint, HELIOS_FLAGS_DEFAULT, HELIOS_MAX_COORD};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::sync::OnceLock;


static CONNECTED_ARC: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();

static SWITCHED_ON_ARC: OnceLock<Arc<Mutex<bool>>> = OnceLock::new();

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
    switched_on: bool,
    thread_running: bool,
    shutdown_sender: Option<Sender<()>>,
}

impl Default for ProjectorDacController {
    fn default() -> Self {
        Self {
            controller: None,
            initialized: false,
            switched_on: false,
            thread_running: false,
            shutdown_sender: None,
        }
    }
}

pub struct ProjectorPlugin;

#[derive(Resource)]
struct DacReconnectTimer(Timer);

impl Default for DacReconnectTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(3.0, TimerMode::Repeating))
    }
}

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectorConfiguration {
            switched_on: true, // Enable by default for testing
            ..Default::default()
        })
            .insert_resource(ProjectorDacController::default())
            .insert_resource(LaserPointBuffer::default())
            .init_resource::<DacReconnectTimer>()
            .add_systems(Startup, initialize_projector_dac)
            .add_systems(Update, update_projector)
            .add_systems(Update, update_point_buffer)
            .add_systems(Last, shutdown_projector_dac.run_if(on_message::<AppExit>));
    }
}

/// Initialize the Helios DAC controller and start background rendering thread
fn initialize_projector_dac(
    mut dac_controller: ResMut<ProjectorDacController>,
    point_buffer: Res<LaserPointBuffer>,
    mut projector_config: ResMut<ProjectorConfiguration>,
) {
    info!("Initializing Helios DAC controller...");
    if try_initialize_projector_dac(&mut dac_controller, &point_buffer, &mut projector_config) {
        info!("✓ Projector initialization complete");
    } else {
        projector_config.connected = false;
    }
}

fn try_initialize_projector_dac(
    dac_controller: &mut ProjectorDacController,
    point_buffer: &LaserPointBuffer,
    projector_config: &mut ProjectorConfiguration,
) -> bool {
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
                return false;
            }

            // Verify device is actually responding with a status check
            match controller.get_status(0) {
                Ok(_) => {
                    info!("✓ DAC status check passed - device is responding");
                }
                Err(e) => {
                    error!("✗ DAC opened but status check failed: {}. Device may not be fully initialized.", e);
                    error!("   Closing devices and retrying on next reconnect tick.");
                    let _ = controller.stop(0);
                    let _ = controller.close_devices();
                    std::thread::sleep(std::time::Duration::from_millis(250));
                    return false;
                }
            }

            // Open the shutter to enable laser output
            if let Err(e) = controller.set_shutter(0, true) {
                warn!("Failed to open laser shutter: {}", e);
            } else {
                info!("✓ Laser shutter opened");
            }

            // Create shared switched_on and connected flags for thread
            let switched_on_flag = Arc::new(Mutex::new(projector_config.switched_on));
            let connected_flag = Arc::new(Mutex::new(true));
            // Start background thread for continuous DAC output
            info!("✓ Starting DAC output thread...");
            let shutdown_sender = start_dac_output_thread(
                controller,
                point_buffer.clone(),
                switched_on_flag.clone(),
                connected_flag.clone(),
            );

            dac_controller.thread_running = true;
            dac_controller.initialized = true;
            dac_controller.shutdown_sender = Some(shutdown_sender);
            dac_controller.switched_on = projector_config.switched_on;
            set_switched_on_arc(switched_on_flag);
            set_connected_arc(connected_flag);
            projector_config.connected = dac_controller.initialized;
            true
        }
        Err(e) => {
            error!("✗ Failed to initialize Helios DAC controller: {}", e);
            false
        }
    }
}

/// Start background thread that continuously sends frames to the DAC
fn start_dac_output_thread(
    controller: HeliosDacController,
    point_buffer: LaserPointBuffer,
    switched_on: Arc<Mutex<bool>>,
    connected: Arc<Mutex<bool>>,
) -> Sender<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let shutdown_tx_clone = shutdown_tx.clone();

    thread::spawn(move || {
        let mut controller = controller;
        info!("DAC output thread started");
        let pps = 15000;
        let flags = 0;
        let mut frame_count = 0;
        let mut consecutive_errors = 0;
        let mut consecutive_write_failures = 0;
        let max_consecutive_errors = 100;
        let max_write_failures = 50;

        loop {
            if shutdown_rx.try_recv().is_ok() {
                info!("✓ DAC output thread received shutdown signal, cleaning up...");
                let _ = controller.stop(0);
                let _ = controller.close_devices();
                drop(controller);
                info!("✓ DAC output thread terminated cleanly");
                let mut connected = connected.lock().unwrap();
                *connected = false;
                break;
            }

            // Get current points from buffer first
            let points = {
                let buffer = point_buffer.points.lock().unwrap();
                buffer.clone()
            };

            // Explicitly check switched_on flag
            let laser_enabled = {
                let on = switched_on.lock().unwrap();
                *on
            } && !points.is_empty() && points.iter().any(|p| p.r > 0 || p.g > 0 || p.b > 0);

            match controller.get_status(0) {
                Ok(true) => {
                    consecutive_errors = 0;

                    if laser_enabled {
                        match controller.write_frame_native(0, pps, flags, &points) {
                            Ok(_) => {
                                consecutive_write_failures = 0;
                                frame_count += 1;
                                if frame_count % 600 == 0 {
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
                                    let mut connected = connected.lock().unwrap();
                                    *connected = false;
                                    let _ = controller.stop(0);
                                    let _ = controller.close_devices();
                                    break;
                                }
                                thread::sleep(std::time::Duration::from_millis(50));
                            }
                        }
                    } else {
                        // Projector is switched off: always send a blanked frame
                        let blank = vec![HeliosPoint::blanked(2048, 2048)];
                        if let Err(e) = controller.write_frame_native(0, pps, flags, &blank) {
                            consecutive_write_failures += 1;
                            if consecutive_write_failures >= max_write_failures {
                                error!("✗ CRITICAL: DAC write failures on blank frames. Thread exiting.");
                                let mut connected = connected.lock().unwrap();
                                *connected = false;
                                break;
                            }
                        } else {
                            consecutive_write_failures = 0;
                        }
                    }
                }
                Ok(false) => {
                    consecutive_errors = 0;
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
                    if consecutive_errors >= max_consecutive_errors * 5 {
                        error!("✗ CRITICAL: {} consecutive status check failures. DAC appears disconnected. Thread exiting.", consecutive_errors);
                        error!("   Please check USB connection and restart the server.");
                        let mut connected = connected.lock().unwrap();
                        *connected = false;
                        let _ = controller.stop(0);
                        let _ = controller.close_devices();
                        break;
                    }
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

    shutdown_tx_clone
}

/// Shutdown the DAC cleanly on app exit
fn shutdown_projector_dac(
    mut dac_controller: ResMut<ProjectorDacController>,
) {
    info!("Shutting down projector DAC...");
    
    if let Some(sender) = dac_controller.shutdown_sender.take() {
        // Signal the thread to shutdown
        if sender.send(()).is_ok() {
            info!("✓ Shutdown signal sent to DAC thread");
        } else {
            warn!("DAC thread already terminated");
        }
    }
    
    dac_controller.thread_running = false;
    dac_controller.initialized = false;
    info!("✓ Projector shutdown complete");
}

fn update_projector(
    mut projector_config: ResMut<ProjectorConfiguration>,
    scene_setup: Res<SceneSetup>,
    mut dac_controller: ResMut<ProjectorDacController>,
    point_buffer: Res<LaserPointBuffer>,
    time: Res<Time>,
    mut reconnect_timer: ResMut<DacReconnectTimer>,
) {
    // Synchronize DAC controller switched_on with projector_config.switched_on
    if projector_config.is_changed() {
        dac_controller.switched_on = projector_config.switched_on;
        // Update the shared Arc<bool> for the DAC thread
        if let Some(flag) = get_switched_on_arc() {
            let mut on = flag.lock().unwrap();
            *on = projector_config.switched_on;
        }
    }
    // Always keep ProjectorConfiguration.connected in sync with thread connection status
    let connected = if let Some(flag) = get_connected_arc() {
        let connected = flag.lock().unwrap();
        *connected
    } else {
        dac_controller.initialized
    };

    if projector_config.connected != connected {
        projector_config.bypass_change_detection().connected = connected;
        projector_config.set_changed();
    }

    if !connected {
        reconnect_timer.0.tick(time.delta());
        if reconnect_timer.0.just_finished() {
            info!("Attempting DAC reconnect...");
            if let Some(sender) = dac_controller.shutdown_sender.take() {
                let _ = sender.send(());
            }
            dac_controller.thread_running = false;
            dac_controller.initialized = false;
            try_initialize_projector_dac(&mut dac_controller, &point_buffer, &mut projector_config);
        }
    } else {
        reconnect_timer.0.reset();
    }

    if projector_config.is_changed() || scene_setup.is_changed() {
        if projector_config.locked_to_scene {
            // Lock projector to scene center
            let scene_center = scene_setup.scene.origin.translation;
            let _new_rotation = Transform::from_translation(projector_config.origin.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
        }
    }
}


/// Update the point buffer with current UniversalPath entities
/// Background thread will continuously send these points to the DAC
fn update_point_buffer(
    projector_config: Res<ProjectorConfiguration>,
    point_buffer: Res<LaserPointBuffer>,
    path_query: Query<(&UniversalPath, &Transform, Option<&ChildOf>, Option<&CalibrationPath>)>,
    scene_query: Query<&Transform, With<SceneEntity>>,
) {
    // Only update buffer if projector is enabled
    if !projector_config.switched_on {
        return;
    }
    
    let path_count = path_query.iter().count();
    if path_count > 0 {
        debug!("Update buffer: Found {} UniversalPath entities", path_count);
    }
    
    // Convert all paths to Helios points
    let mut helios_points = Vec::new();
    
    let scene_transform = scene_query.single().ok();

    for (universal_path, transform, _parent, calibration_path) in path_query.iter() {
        let global_transform = if calibration_path.is_some() {
            GlobalTransform::from(*transform)
        } else if let Some(scene_transform) = scene_transform {
            let world_matrix = scene_transform.to_matrix() * transform.to_matrix();
            GlobalTransform::from(Transform::from_matrix(world_matrix))
        } else {
            GlobalTransform::from(*transform)
        };

        debug!("Converting path with {} segments at {:?}", 
              universal_path.segments.len(), global_transform.translation());
        
        let points = convert_universal_path_to_helios_points(
            universal_path,
            &global_transform,
            &projector_config,
        );
        
        debug!("Generated {} points from path", points.len());
        
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
    
    debug!("Total points collected: {}", helios_points.len());
    
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
    
    debug!("Converting UniversalPath with {} segments at position {:?}", 
          universal_path.segments.len(), global_transform.translation());
    
    for (i, segment) in universal_path.segments.iter().enumerate() {
        let segment_points = convert_path_segment_to_helios_points(
            segment,
            global_transform,
            projector_config,
        );
        debug!("Segment {}: {} points", i, segment_points.len());
        
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
    
    debug!("Total points from UniversalPath: {}", points.len());
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
        debug!("Point {:?} behind projector (local_z: {})", world_pos, local_pos.z);
        return None; // Behind projector
    }
    
    // Use absolute value for projection calculations
    let distance = local_pos.z.abs();
    
    // Apply perspective projection
    let fov_rad = projector_config.angle.to_radians();
    let half_fov = fov_rad / 2.0;
    
    // Project to normalized device coordinates [-1, 1]
    let projected_x = (local_pos.x / distance) / half_fov.tan();
    let projected_y = -((local_pos.y / distance) / half_fov.tan());
    
    // Clip to visible range
    if projected_x.abs() > 1.0 || projected_y.abs() > 1.0 {
        debug!("Point {:?} outside FOV (projected: {}, {})", world_pos, projected_x, projected_y);
        return None; // Outside field of view
    }
    
    // Convert normalized coordinates [-1, 1] to DAC coordinates [0, 4095]
    // -1 maps to 0, 0 maps to 2048 (center), +1 maps to 4095
    let x = ((projected_x + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    let y = ((projected_y + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    
    Some((x, y))
}

fn set_switched_on_arc(flag: Arc<Mutex<bool>>) {
    let _ = SWITCHED_ON_ARC.set(flag);
}
fn get_switched_on_arc() -> Option<Arc<Mutex<bool>>> {
    SWITCHED_ON_ARC.get().cloned()
}
fn set_connected_arc(flag: Arc<Mutex<bool>>) {
    let _ = CONNECTED_ARC.set(flag);
}
fn get_connected_arc() -> Option<Arc<Mutex<bool>>> {
    CONNECTED_ARC.get().cloned()
}