use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneEntity;
use common::scene::SceneSetup;
use common::path::UniversalPath;
use common::settings::{ConfigAppExt, ConfigSection};
use common::state::CalibrationState;
use crate::plugins::calibration::{CalibrationPath, DacSpacePath};
use laserlogic::helios::{HeliosDacController, HeliosPoint, HELIOS_MAX_COORD, HELIOS_FLAGS_DEFAULT};
use laserlogic::{LaserPoint, LaserSegment, OptimizeConfig};
use serde::{Deserialize, Serialize};

#[derive(Resource, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LaserOptimizeConfig(pub OptimizeConfig);

impl ConfigSection for LaserOptimizeConfig {
    const KEY: &'static str = "projector.optimize";
}

/// Helios DAC hardware settings.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct DacSettings {
    /// When false the DAC library is never loaded (machines without a projector attached).
    pub enabled: bool,
    /// Seconds between reconnect attempts after the DAC drops out.
    pub reconnect_secs: f32,
}

impl Default for DacSettings {
    fn default() -> Self {
        Self { enabled: true, reconnect_secs: 5.0 }
    }
}

impl ConfigSection for DacSettings {
    const KEY: &'static str = "projector.dac";
}

impl Default for LaserOptimizeConfig {
    fn default() -> Self {
        Self(OptimizeConfig {
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
        })
    }
}
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Sender};
use std::thread;






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
    /// When false the DAC library is never loaded and no output thread is started (headless tests).
    pub enabled: bool,
    pub controller: Option<HeliosDacController>,
    pub initialized: bool,
    pub switched_on: bool,
    pub thread_running: bool,
    pub shutdown_sender: Option<Sender<()>>,
    pub switched_on_flag: Arc<AtomicBool>,
    pub connected_flag: Arc<AtomicBool>,
}

impl Default for ProjectorDacController {
    fn default() -> Self {
        Self {
            enabled: true,
            controller: None,
            initialized: false,
            switched_on: false,
            thread_running: false,
            shutdown_sender: None,
            switched_on_flag: Arc::new(AtomicBool::new(false)),
            connected_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub struct ProjectorPlugin;

#[derive(Resource)]
struct DacReconnectTimer(Timer);

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_config_or(ProjectorConfiguration {
            switched_on: true, // Enable by default for testing
            ..Default::default()
        })
            .init_config::<LaserOptimizeConfig>()
            .init_config::<DacSettings>();
        let dac = app.world().resource::<DacSettings>().clone();
        app.insert_resource(ProjectorDacController { enabled: dac.enabled, ..Default::default() })
            .insert_resource(LaserPointBuffer::default())
            .insert_resource(DacReconnectTimer(Timer::from_seconds(dac.reconnect_secs, TimerMode::Repeating)))
            .add_systems(Startup, initialize_projector_dac)
            .add_systems(Update, update_projector)
            .add_systems(Update, update_point_buffer)
            .add_systems(Update, handle_gamepad_laser_toggle)
            .add_systems(Last, shutdown_projector_dac.run_if(on_message::<AppExit>));
    }
}

fn handle_gamepad_laser_toggle(
    mut button_events: MessageReader<gamepad::GamepadButtonEvent>,
    mut projector_config: ResMut<ProjectorConfiguration>,
) {
    for event in button_events.read() {
        if event.button == gamepad::Btn::Start && event.pressed {
            projector_config.switched_on = !projector_config.switched_on;
            info!("Laser toggled via Gamepad Start: {}", if projector_config.switched_on { "ON" } else { "OFF" });
        }
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
    if !dac_controller.enabled {
        return false;
    }
    match HeliosDacController::new() {
        Ok(controller) => {
            info!("Helios DAC library loaded successfully");

            // Synchronize shared atomic flags for the thread
            dac_controller.switched_on_flag.store(projector_config.switched_on, Ordering::Release);
            dac_controller.connected_flag.store(false, Ordering::Release);
            
            // Start background thread for continuous DAC output
            info!("✓ Starting DAC output thread (PPS={}, min_pts={})...", projector_config.dac_pps, projector_config.dac_min_points);
            let shutdown_sender = start_dac_output_thread(
                controller,
                point_buffer.clone(),
                dac_controller.switched_on_flag.clone(),
                dac_controller.connected_flag.clone(),
                projector_config.dac_pps,
                projector_config.dac_min_points,
            );

            dac_controller.thread_running = true;
            dac_controller.initialized = true;
            dac_controller.shutdown_sender = Some(shutdown_sender);
            dac_controller.switched_on = projector_config.switched_on;
            
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

/// Helper to prime DAC FIFO with blank points and safely open shutter
fn prime_and_open_shutter(
    controller: &mut HeliosDacController,
    pps: u32,
    blank_frame: &[HeliosPoint],
    context: &str,
) -> bool {
    let mut prime_ok = false;
    for attempt in 0..5 {
        if let Err(err) = controller.write_frame_native(0, pps, 0, blank_frame) {
            warn!("Thread: Priming write failed during {} (attempt {}/5): {}", context, attempt + 1, err);
            std::thread::sleep(std::time::Duration::from_millis(100));
        } else {
            prime_ok = true;
            break;
        }
    }

    if prime_ok {
        let _ = controller.set_shutter(0, true);
        true
    } else {
        false
    }
}

/// Start background thread that continuously sends frames to the DAC
fn start_dac_output_thread(
    controller: HeliosDacController,
    point_buffer: LaserPointBuffer,
    switched_on: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
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
                    connected.store(true, Ordering::Release);
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
            connected.store(false, Ordering::Release);
            return;
        }

        // Allow the DAC firmware to settle after opening
        std::thread::sleep(std::time::Duration::from_millis(200));

        let pps = dac_pps;
        let flags = HELIOS_FLAGS_DEFAULT;

        // Prime the DAC FIFO with a valid blank frame BEFORE opening the laser shutter!
        let mut init_ok = false;
        if let Err(e) = controller.write_frame_native(0, pps, flags, &blank_frame) {
            warn!("Thread: Initial blank priming frame write failed: {}", e);
        } else {
            info!("✓ Thread: Initial DAC blank frame primed successfully");
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Open the shutter
            if let Err(e) = controller.set_shutter(0, true) {
                warn!("Thread: Failed to open laser shutter: {}", e);
            } else {
                info!("✓ Thread: Laser shutter opened");
                std::thread::sleep(std::time::Duration::from_millis(150));
                init_ok = true;
            }
        }

        if !init_ok {
            error!("✗ Thread: Initial DAC setup failed. Entering recovery.");
            connected.store(false, Ordering::Release);
        } else {
            connected.store(true, Ordering::Release);
        }

        let pps = dac_pps;
        let flags = HELIOS_FLAGS_DEFAULT;
        let mut frame_count: u64 = 0;
        let mut consecutive_write_failures: u32 = 0;
        let mut recovery_grace_remaining: u32 = 0;
        let mut last_report = std::time::Instant::now();

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
                connected.store(false, Ordering::Release);
                break;
            }

            // ── 2. Check if laser output is enabled ────────────────────────
            let is_switched_on = switched_on.load(Ordering::Acquire);

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
                        let fps = 600.0 / last_report.elapsed().as_secs_f32();
                        last_report = std::time::Instant::now();
                        info!("✓ [DAC Telemetry] Active: {} frames sent ({:.1} FPS) | Points: {} | PPS: {} | MinPts: {}",
                            frame_count, fps, frame_buf.len(), pps, dac_min_points);
                    }
                }
                Ok(false) => {
                    // DAC is busy playing current frame — micro-sleep 1ms and retry smoothly on next tick (NOT an error!)
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(e) => {
                    consecutive_write_failures += 1;
                    let elapsed_secs = thread_start.elapsed().as_secs_f32();
                    let since_last_err = last_error_time
                        .map(|t| format!("{:.2}s ago", t.elapsed().as_secs_f32()))
                        .unwrap_or_else(|| "first error".to_string());
                    last_error_time = Some(std::time::Instant::now());

                    const MAX_WRITE_FAILURES: u32 = 5;
                    warn!("✗ [DAC Diagnostic] Write error #{}/{} at T+{:.1}s (prev err: {}) | Error: {}",
                        consecutive_write_failures, MAX_WRITE_FAILURES, elapsed_secs, since_last_err, e);

                    // Trigger USB endpoint reset & library reload if 5 consecutive writes fail (~150ms of failure)
                    if consecutive_write_failures >= MAX_WRITE_FAILURES {
                        info!("Thread: USB transient error threshold reached — executing fast recovery & blank priming...");
                        let _ = controller.stop(0);
                        std::thread::sleep(std::time::Duration::from_millis(30));
                        let _ = controller.close_devices();
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        let mut fast_reset_ok = false;
                        if let Ok(devs) = controller.open_devices() {
                            if devs > 0 {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                                if prime_and_open_shutter(&mut controller, pps, &blank_frame, "fast reset") {
                                    info!("✓ Thread: USB fast reset & blank priming successful!");
                                    consecutive_write_failures = 0;
                                    recovery_grace_remaining = 15;
                                    fast_reset_ok = true;
                                }
                            }
                        }

                        if fast_reset_ok {
                            continue;
                        }

                        warn!("Thread: Fast reset failed — executing clean C library reload fallback...");
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        
                        let mut reload_ok = false;
                        if let Ok(mut new_controller) = HeliosDacController::new() {
                            if let Ok(devs) = new_controller.open_devices() {
                                if devs > 0 {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                    if prime_and_open_shutter(&mut new_controller, pps, &blank_frame, "library reload") {
                                        info!("✓ Thread: Clean C library reload recovery successful!");
                                        consecutive_write_failures = 0;
                                        recovery_grace_remaining = 20;
                                        controller = new_controller;
                                        reload_ok = true;
                                    }
                                }
                            }
                        }

                        if reload_ok {
                            continue;
                        }

                        // ── Infinite Persistent Auto-Reconnect Loop ────────────
                        // If USB hardware was physically unplugged or endpoint lost, stay alive in a background retry loop!
                        warn!("✗ Thread: Hardware link lost — entering persistent auto-reconnect loop (retrying every 2s)...");
                        connected.store(false, Ordering::Release);

                        loop {
                            if shutdown_rx.try_recv().is_ok() {
                                info!("✓ DAC output thread received shutdown signal while reconnecting");
                                break;
                            }

                            std::thread::sleep(std::time::Duration::from_secs(2));

                            info!("Thread: Attempting auto-reconnect to Helios DAC hardware...");
                            if let Ok(mut new_controller) = HeliosDacController::new() {
                                if let Ok(devs) = new_controller.open_devices() {
                                    if devs > 0 {
                                        std::thread::sleep(std::time::Duration::from_millis(500));
                                        if prime_and_open_shutter(&mut new_controller, pps, &blank_frame, "auto-reconnect") {
                                            info!("✓ Thread: Auto-reconnect successful! Hardware link restored.");
                                            controller = new_controller;
                                            consecutive_write_failures = 0;
                                            recovery_grace_remaining = 20;
                                            connected.store(true, Ordering::Release);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Micro-sleep backoff between transient retries (2ms, 4ms, 8ms)
                        let backoff_ms = match consecutive_write_failures {
                            1 => 2,
                            2 => 4,
                            _ => 8,
                        };
                        std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    }
                }
            }
        }
        info!("✓ DAC output thread loop exited gracefully.");
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
        dac_controller.switched_on_flag.store(projector_config.switched_on, Ordering::Release);
    }
    // Always keep ProjectorConfiguration.connected in sync with thread connection status
    let connected = if dac_controller.thread_running {
        dac_controller.connected_flag.load(Ordering::Acquire)
    } else {
        dac_controller.initialized
    };

    if projector_config.connected != connected {
        projector_config.bypass_change_detection().connected = connected;
        projector_config.set_changed();
    }

    if !connected && !dac_controller.thread_running {
        reconnect_timer.0.tick(time.delta());
        if reconnect_timer.0.just_finished() {
            info!("Attempting DAC initialization...");
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
    path_query: Query<(&UniversalPath, &Transform, Option<&CalibrationPath>, Option<&DacSpacePath>)>,
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

    for (universal_path, transform, calibration_path, dac_space) in path_query.iter() {
        if calibration_path.is_some() && !is_calibration_on {
            // Calibration mode is OFF: skip all calibration paths (DAC perimeter, scene box, crosshairs)
            continue;
        }
        if dac_space.is_some() {
            general_segments.extend(dac_space_segments(universal_path));
            continue;
        }
        // Calibration overlays must stay visible outside the scene, e.g. while the scene is resized
        let cull_to_scene = calibration_path.is_none();
        let global_transform = if let Some(scene_transform) = scene_transform {
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
                let is_inside_scene = !cull_to_scene
                    || (transformed.x >= min_x
                        && transformed.x <= max_x
                        && transformed.y >= min_y
                        && transformed.y <= max_y);

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

    let optimized = laserlogic::compose_frame(
        &general_segments,
        &text_segments,
        &optimize_config.0,
    );

    debug!("Total points after optimization: {}", optimized.len());

    // Convert to HeliosPoints and update the shared buffer
    let helios_points: Vec<HeliosPoint> = optimized.into_iter().map(HeliosPoint::from).collect();
    if let Ok(mut buffer) = point_buffer.points.lock() {
        *buffer = helios_points;
    }
}

/// Raw DAC-coordinate path to laser segments: no transform, no culling, clamped to the DAC range.
fn dac_space_segments(path: &UniversalPath) -> Vec<LaserSegment> {
    let max = HELIOS_MAX_COORD as f32;
    path.segments
        .iter()
        .map(|segment| {
            let points: Vec<LaserPoint> = segment
                .expand_line_style()
                .iter()
                .flat_map(|p| {
                    let x = p.x.round().clamp(0.0, max) as u16;
                    let y = p.y.round().clamp(0.0, max) as u16;
                    std::iter::repeat_n(LaserPoint::new(x, y, p.r, p.g, p.b, 255), p.dwell.max(1) as usize)
                })
                .collect();
            LaserSegment::new(points)
        })
        .filter(|segment| !segment.points.is_empty())
        .collect()
}

/// Transform world coordinates to projector coordinates using perspective projection
fn world_to_projector_coordinates(
    world_pos: Vec3,
    projector_config: &ProjectorConfiguration,
) -> Option<(u16, u16)> {
    let Some(ndc) = common::scene::projector_ndc(world_pos, projector_config) else {
        debug!("Point {:?} behind projector", world_pos);
        return None;
    };
    let (projected_x, projected_y) = (ndc.x, ndc.y);

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

#[cfg(test)]
mod tests {
    use super::*;
    use common::config::ConfigTransform;

    /// Projector at the origin looking down -Z with a 60 deg field of view.
    fn projector_at(translation: Vec3, angle: f32) -> ProjectorConfiguration {
        ProjectorConfiguration {
            angle,
            origin: ConfigTransform { translation, rotation: Quat::IDENTITY, scale: Vec3::ONE },
            ..Default::default()
        }
    }

    fn project(p: Vec3) -> Option<(u16, u16)> {
        world_to_projector_coordinates(p, &projector_at(Vec3::ZERO, 60.0))
    }

    #[test]
    fn point_on_axis_maps_to_center() {
        assert_eq!(project(Vec3::new(0.0, 0.0, -10.0)), Some((2047, 2047)));
        assert_eq!(project(Vec3::new(0.0, 0.0, -1.0)), Some((2047, 2047)));
    }

    #[test]
    fn points_behind_or_level_with_projector_are_rejected() {
        assert_eq!(project(Vec3::new(0.0, 0.0, 10.0)), None);
        assert_eq!(project(Vec3::new(1.0, 1.0, 0.0)), None);
    }

    #[test]
    fn points_outside_fov_are_rejected() {
        // Half FOV of 30 deg: the edge at 10 m is 10 * tan(30) = 5.77 m off axis
        let edge = 10.0 * 30f32.to_radians().tan();
        let (x, _) = project(Vec3::new(edge - 0.01, 0.0, -10.0)).unwrap();
        assert!(x > 4080, "x = {}", x);
        assert_eq!(project(Vec3::new(edge + 0.01, 0.0, -10.0)), None);
        assert_eq!(project(Vec3::new(0.0, -(edge + 0.01), -10.0)), None);
    }

    #[test]
    fn axes_orientation() {
        let (x_right, y_right) = project(Vec3::new(1.0, 0.0, -10.0)).unwrap();
        assert!(x_right > 2047);
        assert_eq!(y_right, 2047);

        // DAC y grows downward: world up maps to smaller y
        let (x_up, y_up) = project(Vec3::new(0.0, 1.0, -10.0)).unwrap();
        assert_eq!(x_up, 2047);
        assert!(y_up < 2047);
    }

    #[test]
    fn projection_is_perspective() {
        // The same lateral offset appears smaller twice as far away
        let (near, _) = project(Vec3::new(1.0, 0.0, -5.0)).unwrap();
        let (far, _) = project(Vec3::new(1.0, 0.0, -10.0)).unwrap();
        let near_off = near as i32 - 2047;
        let far_off = far as i32 - 2047;
        assert!((near_off - 2 * far_off).abs() <= 1, "near {} far {}", near_off, far_off);
    }

    #[test]
    fn narrower_fov_magnifies() {
        let p = Vec3::new(1.0, 0.0, -10.0);
        let (wide, _) = world_to_projector_coordinates(p, &projector_at(Vec3::ZERO, 60.0)).unwrap();
        let (narrow, _) = world_to_projector_coordinates(p, &projector_at(Vec3::ZERO, 30.0)).unwrap();
        assert!(narrow > wide);
    }

    #[test]
    fn projector_translation_and_rotation_are_respected() {
        let shifted = projector_at(Vec3::new(5.0, 2.0, 0.0), 60.0);
        assert_eq!(world_to_projector_coordinates(Vec3::new(5.0, 2.0, -10.0), &shifted), Some((2047, 2047)));

        // Rotated to look down +X: a point on +X is centred, a point on -Z is now out of view
        let mut turned = projector_at(Vec3::ZERO, 60.0);
        turned.origin.rotation = Transform::IDENTITY.looking_at(Vec3::X, Vec3::Y).rotation;
        assert_eq!(world_to_projector_coordinates(Vec3::new(10.0, 0.0, 0.0), &turned), Some((2047, 2047)));
        assert_eq!(world_to_projector_coordinates(Vec3::new(0.0, 0.0, -10.0), &turned), None);
    }

    #[test]
    fn default_projector_sees_default_scene() {
        // Default projector locked onto the default scene centre: all four scene corners are in view
        let mut config = ProjectorConfiguration::default();
        let scene = common::config::SceneConfiguration::default();
        let center = scene.origin.translation;
        config.origin.rotation = Transform::from_translation(config.origin.translation)
            .looking_at(center, Vec3::Y)
            .rotation;
        assert_eq!(world_to_projector_coordinates(center, &config), Some((2047, 2047)));

        let half = scene.scene_dimension / 2.0;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let corner = center + Vec3::new(sx * half.x, sy * half.y, 0.0);
            assert!(world_to_projector_coordinates(corner, &config).is_some(), "corner {:?} out of view", corner);
        }
    }

    #[test]
    fn laser_optimize_defaults_are_sane() {
        let config = LaserOptimizeConfig::default().0;
        assert!(!config.dynamic_dwell);
        assert_eq!(config.simplify_min_distance, 0.0);
        assert!(config.interp_spacing > 0.0);
        assert!(config.blank_jump_steps >= 4);
    }
}



