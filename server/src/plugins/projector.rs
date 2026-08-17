use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneEntity;
use common::scene::SceneSetup;
use common::path::UniversalPath;
use common::state::CalibrationState;
use crate::plugins::calibration::CalibrationPath;
use crate::dac::helios::{HeliosDacController, HeliosPoint, HELIOS_MAX_COORD, HELIOS_FLAGS_DEFAULT};
use laserlogic::{LaserPoint, LaserSegment, OptimizeConfig};

#[derive(Resource, Clone)]
pub struct LaserOptimizeConfig(pub OptimizeConfig);
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender};
use std::thread;


static CONNECTED_ARC: Mutex<Option<Arc<Mutex<bool>>>> = Mutex::new(None);

static SWITCHED_ON_ARC: Mutex<Option<Arc<Mutex<bool>>>> = Mutex::new(None);

#[derive(Resource, Clone)]
pub struct LaserPointBuffer {
    pub points: Arc<Mutex<Vec<HeliosPoint>>>,
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
    pub initialized: bool,
    pub switched_on: bool,
    pub thread_running: bool,
    pub shutdown_sender: Option<Sender<()>>,
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
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
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
            .insert_resource(LaserOptimizeConfig(OptimizeConfig::default()))
            .init_resource::<DacReconnectTimer>()
            .add_systems(Startup, initialize_projector_dac)
            .add_systems(Update, update_projector)
            .add_systems(Update, update_point_buffer)
            .add_systems(Update, update_laser_optimize_config)
            .add_systems(Last, shutdown_projector_dac.run_if(on_message::<AppExit>));
    }
}

/// System to update LaserOptimizeConfig with fixed, safe values
fn update_laser_optimize_config(
    mut config: ResMut<LaserOptimizeConfig>,
) {
    config.0 = OptimizeConfig {
        start_dwell_points: 6,
        end_dwell_points: 6,
        blank_end_dwell: 20,   // Hold laser-off dwell at shape end before jumping
        blank_start_dwell: 20, // Hold galvo-settle dwell at shape start before firing
        blank_jump_steps: 24,  // Interpolated travel steps between shapes
        interp_distance_threshold: 250.0,
        interp_spacing: 350.0,
        corner_dwell_points: 6,
        corner_angle_threshold: 135.0,
        simplify_min_distance: 0.0,
        simplify_collinear_angle: 0.0,
        dynamic_dwell: false,
        min_dwell: 1,
        max_dwell: 8,
        dwell_distance_threshold: 18.0,
    };
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
        Ok(controller) => {
            info!("Helios DAC library loaded successfully");

            // Create shared switched_on and connected flags for thread
            let switched_on_flag = Arc::new(Mutex::new(projector_config.switched_on));
            // Start as false; background thread will set to true once opened successfully
            let connected_flag = Arc::new(Mutex::new(false)); 
            
            // Start background thread for continuous DAC output
            info!("✓ Starting DAC output thread (PPS={}, min_pts={})...", projector_config.dac_pps, projector_config.dac_min_points);
            let shutdown_sender = start_dac_output_thread(
                controller,
                point_buffer.clone(),
                switched_on_flag.clone(),
                connected_flag.clone(),
                projector_config.dac_pps,
                projector_config.dac_min_points,
            );

            dac_controller.thread_running = true;
            dac_controller.initialized = true;
            dac_controller.shutdown_sender = Some(shutdown_sender);
            dac_controller.switched_on = projector_config.switched_on;
            set_switched_on_arc(switched_on_flag);
            set_connected_arc(connected_flag);
            
            // Assume connected is true initially to let the thread try opening devices.
            // If opening fails, the thread will set connected_flag to false, which
            // Bevy will sync back to projector_config.connected on the next tick.
            projector_config.connected = true;
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
    dac_pps: u32,
    dac_min_points: usize,
) -> Sender<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let shutdown_tx_clone = shutdown_tx.clone();

    thread::spawn(move || {
        let mut controller = controller;
        info!("DAC output thread started (PPS={}, min_points={})", dac_pps, dac_min_points);

        // ── Real-time thread scheduling (Linux only) ───────────────────────
        // Elevate this thread to SCHED_FIFO priority 80 so the Linux scheduler
        // never preempts our DAC write deadline in favour of Bevy game logic,
        // network I/O, or any other normal-priority work on the Raspberry Pi.
        #[cfg(target_os = "linux")]
        unsafe {
            let param = libc::sched_param { sched_priority: 80 };
            let tid = libc::pthread_self();
            let ret = libc::pthread_setschedparam(tid, libc::SCHED_FIFO, &param);
            if ret == 0 {
                info!("✓ DAC thread: real-time scheduling set (SCHED_FIFO prio 80)");
            } else {
                warn!("DAC thread: failed to set real-time scheduling (errno {}). \
                       Add 'AmbientCapabilities=CAP_SYS_NICE' to the service file \
                       or run: sudo setcap cap_sys_nice+eip /opt/lasertargets/server", ret);
            }
        }

        // ── Elapsed-time telemetry ─────────────────────────────────────────
        let thread_start = std::time::Instant::now();
        let mut last_error_time: Option<std::time::Instant> = None;

        // Pre-build a reusable blank frame (avoids regenerating every iteration)
        let blank_frame: Vec<HeliosPoint> = {
            let pts_per_side = (dac_min_points / 4).max(1);
            let mut blank = Vec::with_capacity(dac_min_points);
            let corners: [(i32, i32); 4] = [(1500, 1500), (2500, 1500), (2500, 2500), (1500, 2500)];
            for edge in 0..4 {
                let (ax, ay) = corners[edge];
                let (bx, by) = corners[(edge + 1) % 4];
                for i in 0..pts_per_side {
                    let t = i as f32 / pts_per_side as f32;
                    blank.push(HeliosPoint::blanked(
                        (ax as f32 + t * (bx - ax) as f32) as u16,
                        (ay as f32 + t * (by - ay) as f32) as u16,
                    ));
                }
            }
            while blank.len() < dac_min_points {
                blank.push(HeliosPoint::blanked(1500, 1500));
            }
            blank
        };

        // Open devices inside the thread
        let max_retries = 10;
        let mut devices_opened = false;
        for attempt in 1..=max_retries {
            info!("Thread: Attempting to open Helios DAC devices (attempt {}/{})", attempt, max_retries);
            let _ = controller.close_devices();
            std::thread::sleep(std::time::Duration::from_millis(100));
            match controller.open_devices() {
                Ok(num_devices) if num_devices > 0 => {
                    info!("✓ Thread: Helios DAC: {} device(s) opened", num_devices);
                    devices_opened = true;
                    {
                        let mut conn = connected.lock().unwrap();
                        *conn = true;
                    }
                    // Write initial blank frame immediately with loop mode (0 flags) so the SDK background thread has data and does not time out/error
                    let _ = controller.write_frame_ready(0, dac_pps, 0, &blank_frame, dac_min_points);
                    break;
                }
                Ok(num_devices) if num_devices == 0 => {
                    warn!("Thread: No Helios DAC devices found on attempt {}", attempt);
                    if attempt < max_retries {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
                Ok(num_devices) => {
                    error!("Thread: Unexpected device count {} on attempt {}", num_devices, attempt);
                    if attempt < max_retries {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
                Err(e) => {
                    error!("Thread: Failed to open Helios DAC devices on attempt {}: {}", attempt, e);
                    if attempt < max_retries {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
        }

        if !devices_opened {
            error!("✗ Thread: Failed to open Helios DAC after {} attempts. Thread exiting.", max_retries);
            let mut conn = connected.lock().unwrap();
            *conn = false;
            return;
        }

        // Allow the DAC firmware to settle after opening
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Open the shutter
        if let Err(e) = controller.set_shutter(0, true) {
            warn!("Thread: Failed to open laser shutter: {}", e);
        } else {
            info!("✓ Thread: Laser shutter opened");
            std::thread::sleep(std::time::Duration::from_millis(150));
        }

        // Successfully opened and verified connection! Set flag to true!
        {
            let mut conn = connected.lock().unwrap();
            *conn = true;
        }

        let pps = dac_pps;
        let flags = HELIOS_FLAGS_DEFAULT;
        let mut frame_count: u64 = 0;
        let mut consecutive_write_failures: u32 = 0;
        let max_write_failures: u32 = 50;
        let mut recovery_grace_remaining: u32 = 0;

        // Reusable frame buffer to avoid allocation per loop iteration
        let mut frame_buf: Vec<HeliosPoint> = Vec::with_capacity(dac_min_points * 2);

        loop {
            if recovery_grace_remaining > 0 {
                recovery_grace_remaining -= 1;
            }

            // ── 1. Check shutdown ──────────────────────────────────────────
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

            // ── 2. Check if laser output is enabled ────────────────────────
            let is_switched_on = {
                let on = switched_on.lock().unwrap();
                *on
            };

            // ── 3. Ultra-Fast Zero-Allocation Path Streaming ───────────────
            frame_buf.clear();
            
            let use_blank = if !is_switched_on {
                true
            } else {
                // CRITICAL: use try_lock() not lock() — if the Bevy thread holds this mutex
                // (e.g. during font rendering or path building), blocking here would starve
                // the DAC write deadline and cause a USB pipe stall (-5007).
                // If contested, reuse the previous frame (already in frame_buf from last iteration).
                let locked = point_buffer.points.try_lock();
                if let Ok(buffer) = locked {
                    if !buffer.is_empty() {
                        frame_buf.clear();
                        frame_buf.extend_from_slice(&buffer);
                    }
                    // else: keep previous frame_buf contents
                }
                // else: mutex contested — silently reuse previous frame
                frame_buf.is_empty()
            };

            // If point buffer is empty (e.g. during scene transition) or laser is switched off, use steady pre-built blank frame
            if use_blank {
                frame_buf.extend_from_slice(&blank_frame);
            }

            // Enforce strict constant frame size (exactly dac_min_points) to prevent USB packet negotiation glitches
            if frame_buf.len() < dac_min_points {
                let last = frame_buf.last().cloned().unwrap_or(HeliosPoint::blanked(2048, 2048));
                while frame_buf.len() < dac_min_points {
                    frame_buf.push(HeliosPoint::blanked(last.x, last.y));
                }
            } else if frame_buf.len() > dac_min_points {
                frame_buf.truncate(dac_min_points);
                // Blank the final point of the truncated frame to avoid trailing lines to the next frame start
                if let Some(last) = frame_buf.last_mut() {
                    last.r = 0;
                    last.g = 0;
                    last.b = 0;
                    last.i = 0;
                }
            }

            // ── 4. Write frame to DAC ─
            match controller.write_frame_ready(0, pps, flags, &frame_buf, dac_min_points) {
                Ok(true) => {
                    consecutive_write_failures = 0;
                    frame_count += 1;
                    if frame_count % 600 == 0 {
                        info!("✓ DAC active: {} frames sent, current frame has {} points", frame_count, frame_buf.len());
                    }
                }
                Ok(false) => {
                    // DAC is busy playing current frame — micro-sleep 4ms and retry smoothly on next tick (NOT an error!)
                    std::thread::sleep(std::time::Duration::from_millis(4));
                }
                Err(e) => {
                    consecutive_write_failures += 1;
                    let elapsed_secs = thread_start.elapsed().as_secs_f32();
                    let since_last_err = last_error_time
                        .map(|t| format!("{:.2}s ago", t.elapsed().as_secs_f32()))
                        .unwrap_or_else(|| "first error".to_string());
                    last_error_time = Some(std::time::Instant::now());

                    if consecutive_write_failures == 1 || consecutive_write_failures == 3 {
                        warn!("✗ DAC write failed (#{}) at T+{:.1}s (prev err: {}) | {}",
                            consecutive_write_failures, elapsed_secs, since_last_err, e);
                    }

                    // Only trigger a full USB device reset if 25 consecutive writes fail (~1.2s of persistent failure).
                    let should_reset = consecutive_write_failures >= 25;
                    
                    if should_reset {
                        info!("Thread: USB transient stall/error detected — performing endpoint reset & 3-frame blank priming...");
                        let _ = controller.stop(0);
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let _ = controller.close_devices();
                        std::thread::sleep(std::time::Duration::from_millis(150));

                        let mut fast_reset_ok = false;
                        if let Ok(devs) = controller.open_devices() {
                            if devs > 0 {
                                // ── USB DAC Priming Sequence ──────────────────
                                // Write 3 consecutive blank frames with 80ms delay between them to let hardware FIFO & USB endpoints stabilize!
                                let mut prime_ok = true;
                                for _ in 0..3 {
                                    if let Err(err) = controller.write_frame_ready(0, pps, 0, &blank_frame, dac_min_points) {
                                        warn!("Thread: Priming write failed during reset: {}", err);
                                        prime_ok = false;
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_millis(80));
                                }

                                if prime_ok {
                                    let _ = controller.set_shutter(0, true);
                                    info!("✓ Thread: USB reset & 3-frame blank priming successful!");
                                    consecutive_write_failures = 0;
                                    recovery_grace_remaining = 15; // Cooldown grace: suppress re-recovery for 15 frames (~500ms)
                                    fast_reset_ok = true;
                                }
                            }
                        }

                        if fast_reset_ok {
                            continue;
                        } else {
                            warn!("Thread: Fast reset failed — performing clean C library reload fallback...");
                            drop(controller);
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            match HeliosDacController::new() {
                                Ok(mut new_controller) => {
                                    let mut prime_ok = false;
                                    if let Ok(devs) = new_controller.open_devices() {
                                        if devs > 0 {
                                            prime_ok = true;
                                            for _ in 0..3 {
                                                if let Err(err) = new_controller.write_frame_ready(0, pps, 0, &blank_frame, dac_min_points) {
                                                    warn!("Thread: Priming write failed during fallback reload: {}", err);
                                                    prime_ok = false;
                                                    break;
                                                }
                                                std::thread::sleep(std::time::Duration::from_millis(80));
                                            }
                                        }
                                    }
                                    if prime_ok {
                                        let _ = new_controller.set_shutter(0, true);
                                        info!("✓ Thread: Clean C library reload recovery successful!");
                                        consecutive_write_failures = 0;
                                        recovery_grace_remaining = 20;
                                        controller = new_controller;
                                        continue;
                                    } else {
                                        controller = new_controller;
                                    }
                                }
                                Err(e) => {
                                    error!("Thread: Failed to reload Helios library: {}", e);
                                    let mut connected = connected.lock().unwrap();
                                    *connected = false;
                                    break;
                                }
                            }
                        }
                    } else {
                        // Step out in time with exponential backoff micro-sleeps to let USB endpoints drain:
                        // Failure 1: 2ms | Failure 2: 4ms | Failure 3: 8ms | Failure 4+: 16ms
                        let backoff_ms = match consecutive_write_failures {
                            1 => 2,
                            2 => 4,
                            3 => 8,
                            _ => 16,
                        };
                        std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    }

                    if consecutive_write_failures >= max_write_failures {
                        error!("✗ CRITICAL: {} consecutive write failures. DAC hardware disconnected. Exiting thread.", consecutive_write_failures);
                        let mut connected = connected.lock().unwrap();
                        *connected = false;
                        let _ = controller.stop(0);
                        let _ = controller.close_devices();
                        break;
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
            let new_rotation = Transform::from_translation(projector_config.origin.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
            // Only update if rotation actually changed
            let rotation_dot = projector_config.origin.rotation.dot(new_rotation).abs();
            if rotation_dot < 0.999_999 {
                projector_config.origin.rotation = new_rotation;
            }
        }
    }
}


/// Update the point buffer with current UniversalPath entities
/// Background thread will continuously send these points to the DAC
fn update_point_buffer(
    projector_config: Res<ProjectorConfiguration>,
    scene_setup: Res<SceneSetup>,
    point_buffer: Res<LaserPointBuffer>,
    optimize_config: Res<LaserOptimizeConfig>,
    calibration_state: Option<Res<State<CalibrationState>>>,
    path_query: Query<(&UniversalPath, &Transform, Option<&ChildOf>, Option<&CalibrationPath>)>,
    scene_query: Query<&Transform, With<SceneEntity>>,
) {
    let is_calibration_on = calibration_state
        .as_ref()
        .map(|s| *s.get() == common::state::CalibrationState::On)
        .unwrap_or(true);

    // Only update buffer if projector is enabled
    if !projector_config.switched_on {
        return;
    }

    let path_count = path_query.iter().count();
    if path_count > 0 {
        debug!("Update buffer: Found {} UniversalPath entities", path_count);
    }

    // Scene boundaries for automatic out-of-scene culling & blanking
    let scene_origin = scene_setup.scene.origin.translation;
    let scene_dim = scene_setup.scene.scene_dimension;
    let half_w = scene_dim.x / 2.0;
    let half_h = scene_dim.y / 2.0;

    let min_x = scene_origin.x - half_w;
    let max_x = scene_origin.x + half_w;
    let min_y = scene_origin.y - half_h;
    let max_y = scene_origin.y + half_h;

    // Collect all segments in DAC coordinate space split by hint (Text vs General)
    let mut text_segments: Vec<LaserSegment> = Vec::new();
    let mut general_segments: Vec<LaserSegment> = Vec::new();
    let scene_transform = scene_query.single().ok();

    for (universal_path, transform, _parent, calibration_path) in path_query.iter() {
        if calibration_path.is_some() && !is_calibration_on {
            // Calibration mode is OFF: skip all calibration paths (green rectangle, crosshairs)
            continue;
        }
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

        for segment in &universal_path.segments {
            let styled_points = segment.expand_line_style();
            if styled_points.is_empty() {
                continue;
            }

            let mut active_sub_segment: Vec<LaserPoint> = Vec::new();

            for point in &styled_points {
                let world_pos = Vec3::new(point.x, point.y, 0.0);
                let transformed = global_transform.transform_point(world_pos);

                // Automatic Scene Boundary Culling:
                // Check if point is inside scene rectangle bounds
                let is_inside_scene = transformed.x >= min_x
                    && transformed.x <= max_x
                    && transformed.y >= min_y
                    && transformed.y <= max_y;

                if is_inside_scene {
                    if let Some((x, y)) = world_to_projector_coordinates(transformed, &projector_config) {
                        let repeat_count = if point.dwell == 0 { 1 } else { point.dwell as usize };
                        for _ in 0..repeat_count {
                            active_sub_segment.push(LaserPoint::new(x, y, point.r, point.g, point.b, 255));
                        }
                    }
                } else {
                    // Out-of-scene point: finish current inside sub-segment
                    if !active_sub_segment.is_empty() {
                        let clamped_border = Vec3::new(
                            transformed.x.clamp(min_x, max_x),
                            transformed.y.clamp(min_y, max_y),
                            transformed.z,
                        );
                        if let Some((x, y)) = world_to_projector_coordinates(clamped_border, &projector_config) {
                            if let Some(last) = active_sub_segment.last() {
                                active_sub_segment.push(LaserPoint::new(x, y, last.r, last.g, last.b, 255));
                            }
                        }
                        let laser_seg = LaserSegment::new(active_sub_segment);
                        if segment.hint == common::path::PathHint::Text {
                            text_segments.push(laser_seg);
                        } else {
                            general_segments.push(laser_seg);
                        }
                        active_sub_segment = Vec::new();
                    }
                }
            }

            let is_text = segment.hint == common::path::PathHint::Text;

            if !active_sub_segment.is_empty() {
                let laser_seg = LaserSegment::new(active_sub_segment);
                if is_text {
                    text_segments.push(laser_seg);
                } else {
                    general_segments.push(laser_seg);
                }
            }
        }
    }

    let optimized_text = laserlogic::text_optimize::optimize_text(
        &text_segments,
        &optimize_config.0,
        None,
    );
    let optimized_general = laserlogic::optimize::optimize(&general_segments, &optimize_config.0);

    // Merge: general game shapes first (with laser-off blanking jumps), text on top
    let mut optimized = optimized_general;
    if !optimized_text.is_empty() {
        if !optimized.is_empty() {
            let from = *optimized.last().unwrap();
            let to = optimized_text[0];
            let cfg = &optimize_config.0;
            for _ in 0..cfg.blank_end_dwell {
                optimized.push(LaserPoint::blanked(from.x, from.y));
            }
            let dx = to.x as f32 - from.x as f32;
            let dy = to.y as f32 - from.y as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let steps = ((dist / 250.0).ceil() as u16).clamp(4, cfg.blank_jump_steps.max(4));
            for step in 1..steps {
                let t = step as f32 / steps as f32;
                optimized.push(LaserPoint::blanked(
                    (from.x as f32 + dx * t) as u16,
                    (from.y as f32 + dy * t) as u16,
                ));
            }
            for _ in 0..cfg.blank_start_dwell {
                optimized.push(LaserPoint::blanked(to.x, to.y));
            }
        }
        optimized.extend(optimized_text);
    }

    debug!("Total points after optimization: {}", optimized.len());

    // Convert to HeliosPoints and update the shared buffer
    let helios_points: Vec<HeliosPoint> = optimized.into_iter().map(HeliosPoint::from).collect();
    if let Ok(mut buffer) = point_buffer.points.lock() {
        *buffer = helios_points;
    }
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
    let mut lock = SWITCHED_ON_ARC.lock().unwrap();
    *lock = Some(flag);
}
fn get_switched_on_arc() -> Option<Arc<Mutex<bool>>> {
    let lock = SWITCHED_ON_ARC.lock().unwrap();
    lock.clone()
}
fn set_connected_arc(flag: Arc<Mutex<bool>>) {
    let mut lock = CONNECTED_ARC.lock().unwrap();
    *lock = Some(flag);
}
fn get_connected_arc() -> Option<Arc<Mutex<bool>>> {
    let lock = CONNECTED_ARC.lock().unwrap();
    lock.clone()
}
