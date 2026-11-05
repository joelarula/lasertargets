//! Physical camera → Bevy. A worker thread runs `laser_vision::VisionEngine` (frame source,
//! analyzers, tracker, homography) while `vision.enabled` is set, and this plugin turns its results
//! into `common::vision` messages on the scene plane. `common::vision::VisionReactivePlugin` makes
//! them reactive visuals; games read the same messages for gameplay.
//!
//! Analyzers are pluggable: `app.add_vision_analyzer(|| Box::new(MyModel::load()))` adds one to the
//! worker (applied on its next start). Each produces labelled detections and/or `vision.<name>`
//! signals.

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy_quinnet::server::QuinnetServer;
use common::camera::{CameraStreamFrame, PhysicalCameraSettings, VisualEventMarker};
use common::network::NetworkMessage;
use common::pipeline::plugin::ReactiveAppExt;
use common::scene::SceneSetup;
use common::settings::ConfigAppExt;
use common::vision::{VisionAppExt, VisionDetection, VisionLost, VisionReactivePlugin, VisionSet, VisionSignal};
use laser_vision::{FrameSource, LostTrack, MotionAnalyzer, SpotAnalyzer, VisionAnalyzer, VisionEngine, VisionObject};

/// Results of one camera frame, sent from the worker (no pixels)
struct VisionUpdate {
    timestamp_ms: u64,
    objects: Vec<VisionObject>,
    lost: Vec<LostTrack>,
    signals: Vec<(String, f32)>,
    calibration_quad: Option<[Vec2; 4]>,
}

/// Camera updates buffered between the worker and Bevy; when Bevy falls behind, newer updates are
/// dropped (tracks still end through `VisionReactSettings::stale_after_secs`)
const UPDATE_BUFFER: usize = 256;

type AnalyzerFactory = Arc<dyn Fn() -> Box<dyn VisionAnalyzer> + Send + Sync>;

/// Makes the analyzers for each worker start, in order.
#[derive(Resource, Default, Clone)]
pub struct VisionAnalyzers(pub Vec<AnalyzerFactory>);

pub trait VisionAnalyzerAppExt {
    /// Adds an analyzer to the camera worker
    fn add_vision_analyzer(&mut self, make: impl Fn() -> Box<dyn VisionAnalyzer> + Send + Sync + 'static) -> &mut Self;
}

impl VisionAnalyzerAppExt for App {
    fn add_vision_analyzer(&mut self, make: impl Fn() -> Box<dyn VisionAnalyzer> + Send + Sync + 'static) -> &mut Self {
        self.init_resource::<VisionAnalyzers>();
        self.world_mut().resource_mut::<VisionAnalyzers>().0.push(Arc::new(make));
        self
    }
}

/// Camera → projector mapping: 4 normalised camera points and the projector points they show,
/// ordered top-left, top-right, bottom-right, bottom-left. `None` uses the uncalibrated default.
/// Changing it recalibrates the running worker.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct VisionCalibration(pub Option<CalibrationQuad>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationQuad {
    pub camera_pts: [Vec2; 4],
    pub galvo_pts: [Vec2; 4],
}

/// Whether review frames are broadcast to terminals
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisionStreaming(pub bool);

enum VisionCommand {
    SetCalibration(Option<CalibrationQuad>),
}

/// Channels to the running worker. Removing it stops the worker.
#[derive(Resource)]
pub struct VisionBridge {
    updates: Mutex<Receiver<VisionUpdate>>,
    commands: mpsc::Sender<VisionCommand>,
}

/// Latest camera update, for the review stream
#[derive(Resource, Default)]
struct LatestVisionUpdate(Option<VisionUpdate>);

pub struct VisionPlugin;

impl Plugin for VisionPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<PhysicalCameraSettings>()
            .init_resource::<VisionCalibration>()
            .init_resource::<VisionStreaming>()
            .init_resource::<LatestVisionUpdate>()
            .add_plugins(VisionReactivePlugin)
            .add_vision_analyzer(|| Box::new(SpotAnalyzer::default()))
            .add_vision_analyzer(|| Box::new(MotionAnalyzer::default()))
            // Default look for laser dots: a reticle that follows each dot, a ripple where it lands
            .embed_pipeline(include_str!("../../../assets/pipelines/vision/spot.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/vision/impact.pipeline.yaml"))
            .follow_vision("laser", "vision/spot")
            .on_vision_appear("laser", "vision/impact")
            .add_systems(
                Update,
                (
                    (run_vision_worker, send_vision_calibration).chain().before(VisionSet::Ingest),
                    ingest_vision.in_set(VisionSet::Ingest),
                    (handle_camera_network_requests, broadcast_vision_preview).chain(),
                ),
            );
    }
}

/// Starts the worker when the camera is enabled and stops it when disabled
fn run_vision_worker(
    mut commands: Commands,
    settings: Res<PhysicalCameraSettings>,
    analyzers: Res<VisionAnalyzers>,
    calibration: Res<VisionCalibration>,
    bridge: Option<Res<VisionBridge>>,
) {
    match (settings.enabled, bridge.is_some()) {
        (true, false) => {
            let (update_tx, update_rx) = mpsc::sync_channel(UPDATE_BUFFER);
            let (command_tx, command_rx) = mpsc::channel();
            let analyzers: Vec<Box<dyn VisionAnalyzer>> = analyzers.0.iter().map(|make| make()).collect();
            let names: Vec<&str> = analyzers.iter().map(|a| a.name()).collect();
            let started = format!("{}x{} @ {} fps, analyzers: {}",
                settings.stream_width, settings.stream_height, settings.stream_fps, names.join(", "));
            let worker_settings = settings.clone();
            let quad = calibration.0;
            let spawned = std::thread::Builder::new()
                .name("laser-vision-worker".into())
                .spawn(move || run_worker(&worker_settings, analyzers, quad, update_tx, command_rx));
            match spawned {
                Ok(_) => {
                    info!("Vision: camera worker started ({started})");
                    commands.insert_resource(VisionBridge { updates: Mutex::new(update_rx), commands: command_tx });
                }
                Err(e) => error!("Vision: failed to start camera worker: {e}"),
            }
        }
        (false, true) => {
            // Dropping the bridge disconnects the worker's channels, which ends it
            commands.remove_resource::<VisionBridge>();
            info!("Vision: camera worker stopped");
        }
        _ => {}
    }
}

fn send_vision_calibration(calibration: Res<VisionCalibration>, bridge: Option<Res<VisionBridge>>) {
    if let (true, Some(bridge)) = (calibration.is_changed() && !calibration.is_added(), bridge) {
        let _ = bridge.commands.send(VisionCommand::SetCalibration(calibration.0));
    }
}

fn open_frame_source(settings: &PhysicalCameraSettings) -> Box<dyn FrameSource> {
    let (width, height, fps) = (settings.stream_width, settings.stream_height, settings.stream_fps);

    #[cfg(target_os = "linux")]
    match laser_vision::ProcessPipeFrameSource::new(width, height, fps) {
        Ok(source) => {
            info!("Vision: using native rpicam-vid pipe");
            return Box::new(source);
        }
        Err(e) => warn!("Vision: failed to spawn rpicam-vid ({e}), falling back to the mock camera"),
    }

    #[cfg(not(target_os = "linux"))]
    info!("Vision: using the mock camera on this platform");

    Box::new(laser_vision::MockFrameSource::new(width, height, fps))
}

fn apply_calibration(engine: &mut VisionEngine, quad: Option<CalibrationQuad>) {
    match quad {
        Some(q) if engine.set_calibration_quad(q.camera_pts, q.galvo_pts) => info!("Vision: calibration updated"),
        Some(_) => warn!("Vision: calibration quad is degenerate, keeping the previous mapping"),
        None => engine.reset_calibration(),
    }
}

/// Worker thread: analyses frames until Bevy drops its end of the channels
fn run_worker(
    settings: &PhysicalCameraSettings,
    analyzers: Vec<Box<dyn VisionAnalyzer>>,
    quad: Option<CalibrationQuad>,
    updates: SyncSender<VisionUpdate>,
    commands: Receiver<VisionCommand>,
) {
    let mut engine = VisionEngine::new(open_frame_source(settings), analyzers);
    if quad.is_some() {
        apply_calibration(&mut engine, quad);
    }

    loop {
        loop {
            match commands.try_recv() {
                Ok(VisionCommand::SetCalibration(quad)) => apply_calibration(&mut engine, quad),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let Some(frame) = engine.process_next_frame() else {
            // Avoid spinning while the source has no frame
            std::thread::sleep(std::time::Duration::from_millis(2));
            continue;
        };
        let update = VisionUpdate {
            timestamp_ms: frame.timestamp_ms,
            objects: frame.objects,
            lost: frame.lost,
            signals: frame.signals,
            calibration_quad: engine.calibration_quad(),
        };
        if let Err(TrySendError::Disconnected(_)) = updates.try_send(update) {
            return;
        }
    }
}

/// Camera updates → vision messages, with positions on the scene plane
fn ingest_vision(
    bridge: Option<Res<VisionBridge>>,
    scene: Res<SceneSetup>,
    mut latest: ResMut<LatestVisionUpdate>,
    mut detections: MessageWriter<VisionDetection>,
    mut lost: MessageWriter<VisionLost>,
    mut signals: MessageWriter<VisionSignal>,
) {
    let Some(bridge) = bridge else { return };
    let Ok(updates) = bridge.updates.lock() else { return };

    while let Ok(update) = updates.try_recv() {
        for object in &update.objects {
            // A projector ray that misses the scene plane can't be placed in the world
            let Some(world_pos) = scene.projector_ndc_to_world(object.galvo_pos) else { continue };
            let size_m = match object.galvo_bounds.map(|corner| scene.projector_ndc_to_world(corner)) {
                [Some(a), Some(b)] => (scene.world_to_billboard(b) - scene.world_to_billboard(a)).abs(),
                _ => Vec2::ZERO,
            };
            detections.write(VisionDetection {
                track: object.track,
                appeared: object.appeared,
                label: object.label.clone(),
                world_pos,
                size_m,
                camera_pos: object.camera_pos,
                galvo_pos: object.galvo_pos,
                confidence: object.confidence,
                timestamp_ms: update.timestamp_ms,
            });
        }
        lost.write_batch(update.lost.iter().map(|l| VisionLost { track: l.track, label: l.label.clone() }));
        signals.write_batch(update.signals.iter().map(|(name, value)| VisionSignal { name: name.clone(), value: *value }));
        latest.0 = Some(update);
    }
}

/// Terminal requests to toggle the review stream; streaming turns the camera on
fn handle_camera_network_requests(
    mut client_messages: MessageReader<crate::plugins::network::FromClientMessage>,
    mut streaming: ResMut<VisionStreaming>,
    mut settings: ResMut<PhysicalCameraSettings>,
) {
    for msg in client_messages.read() {
        if let NetworkMessage::SetCameraStreamState { enabled, stream_type } = &msg.message {
            info!("Vision: review stream enabled={enabled}, type={stream_type:?}");
            streaming.0 = *enabled;
            settings.active_stream = *stream_type;
            if *enabled && !settings.enabled {
                settings.enabled = true;
            }
        }
    }
}

/// Sends the latest camera update to terminals as a review frame with its markers
fn broadcast_vision_preview(
    streaming: Res<VisionStreaming>,
    settings: Res<PhysicalCameraSettings>,
    mut latest: ResMut<LatestVisionUpdate>,
    server: Option<ResMut<QuinnetServer>>,
) {
    if !streaming.0 {
        return;
    }
    let (Some(update), Some(mut server)) = (latest.0.take(), server) else { return };
    let Some(endpoint) = server.get_endpoint_mut() else { return };

    let frame = CameraStreamFrame {
        timestamp_ms: update.timestamp_ms,
        width: settings.stream_width,
        height: settings.stream_height,
        // Pixels are not streamed yet; terminals draw the markers and calibration quad
        jpeg_data: Vec::new(),
        detected_events: update
            .objects
            .iter()
            .map(|o| VisualEventMarker {
                camera_norm_pos: o.camera_pos,
                galvo_pos: o.galvo_pos,
                label: o.label.clone(),
                track: o.track,
                confidence: o.confidence,
                timestamp_ms: update.timestamp_ms,
            })
            .collect(),
        calibration_quad: update.calibration_quad,
    };
    match NetworkMessage::CameraPreviewFrame(frame).to_bytes() {
        Ok(payload) => {
            let _ = endpoint.broadcast_payload(payload);
        }
        Err(e) => error!("Vision: failed to serialize the review frame: {e}"),
    }
}
