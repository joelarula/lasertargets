//! Camera ↔ projector calibration. The projector draws one dot at a time on a grid over the
//! scene, alone; the camera finds each dot; a least-squares fit of camera → projector coordinates
//! then lets the vision plugin place everything the camera sees on the scene, in metres.
//!
//! Started from a terminal (`NetworkMessage::StartCameraCalibration`) or with
//! [`CameraCalibrationCommand`]. Progress and the result go to terminals as
//! `NetworkMessage::CameraCalibrationStatus`. The solved mapping is applied at once and saved
//! (`vision.calibration.file`), and loaded again on startup.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy_quinnet::server::QuinnetServer;
use common::camera::{CameraCalibrationPhase, CameraCalibrationStatus, PhysicalCameraSettings};
use common::config::ProjectorConfiguration;
use common::network::NetworkMessage;
use common::path::UniversalPath;
use common::scene::SceneSetup;
use common::settings::{ConfigAppExt, ConfigSection, ConfigSource};
use common::vision::VisionSet;
use laser_vision::{fit_calibration, grid_pattern, median, Homography};
use serde::{Deserialize, Serialize};

use crate::plugins::calibration::DacSpacePath;
use crate::plugins::network::{broadcast_network_message, FromClientMessage};
use crate::plugins::projector::{ndc_to_dac, ProjectorSolo};
use crate::plugins::vision::{CameraObservation, VisionCalibration, VisionMuted};

/// Config section `vision.calibration`.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct CameraCalibrationSettings {
    /// Dots across and down the scene
    pub grid: [u32; 2],
    /// Share of the scene width/height left free at each edge
    pub margin: f32,
    /// Wait after moving the dot before measuring it, ms (galvo, exposure and camera latency)
    pub settle_ms: f32,
    /// How long each dot is measured, ms
    pub measure_ms: f32,
    /// Extra wait before the first dot when calibration had to start the camera, ms
    pub camera_warmup_ms: f32,
    /// Dot colour; must be what the camera's spot analyzer looks for (green by default)
    pub dot_color: [u8; 3],
    /// Dot radius in DAC units
    pub dot_radius_dac: f32,
    /// Detection label the dot shows up as
    pub label: String,
    /// Where the solved calibration is kept; relative to the config directory
    pub file: String,
}

impl Default for CameraCalibrationSettings {
    fn default() -> Self {
        Self {
            grid: [3, 3],
            margin: 0.1,
            settle_ms: 350.0,
            measure_ms: 400.0,
            camera_warmup_ms: 1500.0,
            dot_color: [0, 255, 0],
            dot_radius_dac: 25.0,
            label: "laser".into(),
            file: "camera_calibration.json".into(),
        }
    }
}

impl ConfigSection for CameraCalibrationSettings {
    const KEY: &'static str = "vision.calibration";
}

#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraCalibrationCommand {
    Start,
    Cancel,
    /// Forget the measured mapping (and its file)
    Reset,
}

/// Current calibration status; terminals get it whenever it changes.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct CameraCalibration(pub CameraCalibrationStatus);

/// The calibration file
#[derive(Debug, Serialize, Deserialize)]
struct SavedCalibration {
    homography: Homography,
    rms_error_m: f32,
    max_error_m: f32,
    points: usize,
}

/// A calibration in progress
#[derive(Resource)]
struct CalibrationRun {
    /// Time spent on the current dot; starts negative while the camera warms up
    elapsed_ms: f32,
    samples: Vec<Vec2>,
    dot: Entity,
}

/// The step that measures the projected dot; camera sources run before it
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraCalibrationSet;

pub struct CameraCalibrationPlugin;

impl Plugin for CameraCalibrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<CameraCalibrationSettings>()
            .init_resource::<CameraCalibration>()
            .add_message::<CameraCalibrationCommand>()
            .add_systems(Startup, load_saved_calibration)
            .add_systems(
                Update,
                (
                    network_calibration_requests,
                    handle_calibration_commands,
                    measure_calibration_dots.in_set(CameraCalibrationSet).after(VisionSet::Ingest),
                    broadcast_calibration_status,
                )
                    .chain(),
            );
    }
}

/// `file` resolved against the directory `server.yaml` was found in
fn calibration_path(file: &str) -> PathBuf {
    let path = PathBuf::from(file);
    if path.is_absolute() {
        return path;
    }
    ConfigSource::locate("server.yaml")
        .and_then(|config| config.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("config"))
        .join(path)
}

fn load_saved_calibration(
    settings: Res<CameraCalibrationSettings>,
    mut vision: ResMut<VisionCalibration>,
    mut status: ResMut<CameraCalibration>,
) {
    let path = calibration_path(&settings.file);
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    match serde_json::from_str::<SavedCalibration>(&text) {
        Ok(saved) => {
            vision.0 = Some(saved.homography);
            status.0 = CameraCalibrationStatus {
                calibrated: true,
                rms_error_m: Some(saved.rms_error_m),
                max_error_m: Some(saved.max_error_m),
                message: format!("Loaded calibration from {} dots ({})", saved.points, error_text(saved.rms_error_m, saved.max_error_m)),
                ..default()
            };
            info!("Camera calibration: loaded {}", path.display());
        }
        Err(e) => warn!("Camera calibration: ignoring unreadable {}: {e}", path.display()),
    }
}

fn error_text(rms_m: f32, max_m: f32) -> String {
    format!("error {:.1} cm RMS, {:.1} cm max", rms_m * 100.0, max_m * 100.0)
}

/// Terminal requests → commands; a status query re-sends the status to everyone
fn network_calibration_requests(
    mut client_messages: MessageReader<FromClientMessage>,
    mut commands: MessageWriter<CameraCalibrationCommand>,
    mut status: ResMut<CameraCalibration>,
) {
    for msg in client_messages.read() {
        match msg.message {
            NetworkMessage::StartCameraCalibration => {
                commands.write(CameraCalibrationCommand::Start);
            }
            NetworkMessage::CancelCameraCalibration => {
                commands.write(CameraCalibrationCommand::Cancel);
            }
            NetworkMessage::ResetCameraCalibration => {
                commands.write(CameraCalibrationCommand::Reset);
            }
            NetworkMessage::QueryCameraCalibration => status.set_changed(),
            _ => {}
        }
    }
}

/// The calibration dot at `target` (projector coordinates), drawn in DAC space
fn dot_path(target: Vec2, settings: &CameraCalibrationSettings) -> UniversalPath {
    let [r, g, b] = settings.dot_color;
    UniversalPath::circle(ndc_to_dac(target), settings.dot_radius_dac, Color::srgb_u8(r, g, b))
}

/// Dot positions in projector coordinates: a grid over the scene, keeping only points the
/// projector can reach
fn calibration_targets(settings: &CameraCalibrationSettings, scene: &SceneSetup) -> Vec<Vec2> {
    grid_pattern(settings.grid[0], settings.grid[1], settings.margin)
        .into_iter()
        .filter_map(|uv| scene.projector_ndc(scene.billboard_point(uv)))
        .filter(|ndc| ndc.abs().max_element() <= 1.0)
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn handle_calibration_commands(
    mut commands: Commands,
    mut requests: MessageReader<CameraCalibrationCommand>,
    settings: Res<CameraCalibrationSettings>,
    scene: Res<SceneSetup>,
    projector: Res<ProjectorConfiguration>,
    run: Option<Res<CalibrationRun>>,
    mut camera: ResMut<PhysicalCameraSettings>,
    mut muted: ResMut<VisionMuted>,
    mut vision: ResMut<VisionCalibration>,
    mut status: ResMut<CameraCalibration>,
) {
    for request in requests.read() {
        match request {
            CameraCalibrationCommand::Start if run.is_none() => {
                let calibrated = status.0.calibrated;
                if !projector.switched_on {
                    status.0 = failed(calibrated, "The laser is off; switch it on and start again");
                    continue;
                }
                let targets = calibration_targets(&settings, &scene);
                if targets.len() < laser_vision::MIN_CALIBRATION_POINTS {
                    status.0 = failed(calibrated, "Too little of the scene is inside the projector range");
                    continue;
                }

                let warmup = if camera.enabled { 0.0 } else { settings.camera_warmup_ms };
                camera.enabled = true;
                muted.0 = true;
                let dot = commands.spawn((dot_path(targets[0], &settings), Transform::default(), DacSpacePath, ProjectorSolo)).id();
                commands.insert_resource(CalibrationRun { elapsed_ms: -warmup, samples: Vec::new(), dot });

                info!("Camera calibration: measuring {} dots", targets.len());
                status.0 = CameraCalibrationStatus {
                    phase: CameraCalibrationPhase::Running,
                    calibrated,
                    measured: vec![None; targets.len()],
                    predicted: vec![None; targets.len()],
                    current: Some(0),
                    message: format!("Measuring dot 1 of {}", targets.len()),
                    targets,
                    ..default()
                };
            }
            CameraCalibrationCommand::Start => {}
            CameraCalibrationCommand::Cancel => {
                if let Some(run) = &run {
                    end_run(&mut commands, run, &mut muted);
                    status.0.phase = CameraCalibrationPhase::Idle;
                    status.0.current = None;
                    status.0.message = "Cancelled; the previous calibration is still in use".into();
                }
            }
            CameraCalibrationCommand::Reset => {
                if let Some(run) = &run {
                    end_run(&mut commands, run, &mut muted);
                }
                vision.0 = None;
                let path = calibration_path(&settings.file);
                if path.exists() {
                    if let Err(e) = std::fs::remove_file(&path) {
                        warn!("Camera calibration: could not remove {}: {e}", path.display());
                    }
                }
                status.0 = CameraCalibrationStatus { message: "Calibration cleared; using the uncalibrated default".into(), ..default() };
            }
        }
    }
}

fn failed(calibrated: bool, message: &str) -> CameraCalibrationStatus {
    warn!("Camera calibration: {message}");
    CameraCalibrationStatus { phase: CameraCalibrationPhase::Failed, calibrated, message: message.into(), ..default() }
}

fn end_run(commands: &mut Commands, run: &CalibrationRun, muted: &mut VisionMuted) {
    commands.entity(run.dot).try_despawn();
    commands.remove_resource::<CalibrationRun>();
    muted.0 = false;
}

/// Measures the current dot, then moves to the next one; solves after the last
#[allow(clippy::too_many_arguments)]
fn measure_calibration_dots(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<CameraCalibrationSettings>,
    scene: Res<SceneSetup>,
    run: Option<ResMut<CalibrationRun>>,
    mut observations: MessageReader<CameraObservation>,
    mut paths: Query<&mut UniversalPath>,
    mut muted: ResMut<VisionMuted>,
    mut vision: ResMut<VisionCalibration>,
    mut status: ResMut<CameraCalibration>,
) {
    let Some(mut run) = run else {
        observations.clear();
        return;
    };
    run.elapsed_ms += time.delta_secs() * 1000.0;

    let measuring = run.elapsed_ms >= settings.settle_ms;
    for observation in observations.read() {
        // The strongest detection of the dot's label in each frame
        let strongest = observation
            .objects
            .iter()
            .filter(|o| o.label == settings.label)
            .max_by(|a, b| a.confidence.total_cmp(&b.confidence));
        if let (true, Some(object)) = (measuring, strongest) {
            run.samples.push(object.camera_pos);
        }
    }
    if run.elapsed_ms < settings.settle_ms + settings.measure_ms {
        return;
    }

    let Some(index) = status.0.current else { return };
    status.0.measured[index] = median(&run.samples);
    run.samples.clear();
    run.elapsed_ms = 0.0;

    let next = index + 1;
    if let Some(&target) = status.0.targets.get(next) {
        if let Ok(mut path) = paths.get_mut(run.dot) {
            *path = dot_path(target, &settings);
        }
        status.0.current = Some(next);
        status.0.message = format!("Measuring dot {} of {}", next + 1, status.0.targets.len());
        return;
    }

    end_run(&mut commands, &run, &mut muted);
    solve(&settings, &scene, &mut vision, &mut status.0);
}

/// Fits the mapping from the measured dots, applies and saves it
fn solve(settings: &CameraCalibrationSettings, scene: &SceneSetup, vision: &mut VisionCalibration, status: &mut CameraCalibrationStatus) {
    status.current = None;
    let measured: Vec<(usize, Vec2, Vec2)> = status
        .measured
        .iter()
        .zip(&status.targets)
        .enumerate()
        .filter_map(|(i, (camera, target))| camera.map(|c| (i, c, *target)))
        .collect();
    let seen = measured.len();
    let total = status.targets.len();

    let pairs: Vec<(Vec2, Vec2)> = measured.iter().map(|(_, c, t)| (*c, *t)).collect();
    let Some(fit) = fit_calibration(&pairs) else {
        *status = CameraCalibrationStatus {
            targets: std::mem::take(&mut status.targets),
            measured: std::mem::take(&mut status.measured),
            ..failed(
                status.calibrated,
                &format!(
                    "The camera saw {seen} of {total} dots, too few to calibrate. Check that it sees the scene, \
                     and that the dot colour is the one it detects"
                ),
            )
        };
        return;
    };

    // Error on the scene: where each used dot lands under the fit vs where it was projected
    let errors_m: Vec<f32> = fit
        .used
        .iter()
        .filter_map(|&k| {
            let predicted = scene.projector_ndc_to_world(fit.predicted[k])?;
            let target = scene.projector_ndc_to_world(pairs[k].1)?;
            Some(predicted.distance(target))
        })
        .collect();
    let rms = (errors_m.iter().map(|e| e * e).sum::<f32>() / errors_m.len().max(1) as f32).sqrt();
    let max = errors_m.iter().copied().fold(0.0, f32::max);

    for (k, (i, ..)) in measured.iter().enumerate() {
        status.predicted[*i] = fit.used.contains(&k).then_some(fit.predicted[k]);
    }
    vision.0 = Some(fit.homography);
    status.phase = CameraCalibrationPhase::Solved;
    status.calibrated = true;
    status.rms_error_m = Some(rms);
    status.max_error_m = Some(max);
    status.message = format!("Calibrated from {} of {total} dots: {}", fit.used.len(), error_text(rms, max));
    info!("Camera calibration: {}", status.message);

    let saved = SavedCalibration { homography: fit.homography, rms_error_m: rms, max_error_m: max, points: fit.used.len() };
    let path = calibration_path(&settings.file);
    let written = serde_json::to_string_pretty(&saved)
        .map_err(|e| e.to_string())
        .and_then(|json| std::fs::write(&path, json).map_err(|e| e.to_string()));
    if let Err(e) = written {
        warn!("Camera calibration: could not save {}: {e}", path.display());
        status.message.push_str(" (not saved)");
    }
}

fn broadcast_calibration_status(status: Res<CameraCalibration>, server: Option<ResMut<QuinnetServer>>) {
    if !status.is_changed() {
        return;
    }
    let Some(mut server) = server else { return };
    let Some(endpoint) = server.get_endpoint_mut() else { return };
    broadcast_network_message(endpoint, &NetworkMessage::CameraCalibrationStatus(status.0.clone()), "CameraCalibrationStatus");
}
