//! Camera calibration panel: starts, cancels and resets the server's camera ↔ projector
//! calibration, shows its progress and error, and shows what the camera currently sees — in the
//! camera image, on the projection, and as positions on the scene in metres.
//!
//! While the panel is open the server streams camera review frames (markers only, no pixels).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use common::camera::{CameraCalibrationPhase, CameraCalibrationStatus, CameraStreamFrame, CameraStreamType};
use common::network::{NetworkMessage, ToServer};
use terminal_ui::{Docking, ToolbarAppExt, ToolbarItem, toggled_on};

use crate::net::ServerMessage;

/// Side of the projection view and width of the camera view, in points
const VIEW_SIZE: f32 = 240.0;

#[derive(Resource, Default, Deref, DerefMut)]
struct CameraCalibrationOpen(bool);

#[derive(Resource, Default)]
struct CameraCalibration {
    status: CameraCalibrationStatus,
    /// Latest camera review frame
    preview: Option<CameraStreamFrame>,
}

pub struct CameraCalibrationPlugin;

impl Plugin for CameraCalibrationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraCalibration>()
            .add_toolbar_toggle::<CameraCalibrationOpen>(
                ToolbarItem::new("camera_calibration").icon("\u{f030}").text("Camera Calibration").order(4).dock(Docking::Left),
            )
            .add_systems(
                Update,
                (stream_while_open.run_if(resource_changed::<CameraCalibrationOpen>), receive_calibration),
            )
            .add_systems(EguiPrimaryContextPass, calibration_panel.run_if(toggled_on::<CameraCalibrationOpen>));
    }
}

/// The camera review stream runs while the panel is open
fn stream_while_open(open: Res<CameraCalibrationOpen>, mut calibration: ResMut<CameraCalibration>, mut to_server: MessageWriter<ToServer>) {
    if open.is_added() {
        return;
    }
    to_server.write(ToServer(NetworkMessage::SetCameraStreamState { enabled: **open, stream_type: CameraStreamType::PiCamera }));
    if **open {
        to_server.write(ToServer(NetworkMessage::QueryCameraCalibration));
    } else {
        calibration.preview = None;
    }
}

fn receive_calibration(
    mut messages: MessageReader<ServerMessage>,
    open: Res<CameraCalibrationOpen>,
    mut calibration: ResMut<CameraCalibration>,
) {
    for ServerMessage(message) in messages.read() {
        match message {
            NetworkMessage::CameraCalibrationStatus(status) => calibration.status = status.clone(),
            NetworkMessage::CameraPreviewFrame(frame) if **open => calibration.preview = Some(frame.clone()),
            _ => {}
        }
    }
}

const TARGET: egui::Color32 = egui::Color32::from_rgb(140, 140, 150);
const CURRENT: egui::Color32 = egui::Color32::from_rgb(255, 220, 40);
const MEASURED: egui::Color32 = egui::Color32::from_rgb(60, 220, 90);
const ERROR: egui::Color32 = egui::Color32::from_rgb(255, 80, 60);
const LIVE: egui::Color32 = egui::Color32::from_rgb(60, 200, 255);
const PROJECTOR: egui::Color32 = egui::Color32::from_rgb(255, 160, 40);

fn cross(painter: &egui::Painter, at: egui::Pos2, size: f32, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5, color);
    painter.line_segment([at - egui::vec2(size, 0.0), at + egui::vec2(size, 0.0)], stroke);
    painter.line_segment([at - egui::vec2(0.0, size), at + egui::vec2(0.0, size)], stroke);
}

fn view_frame(ui: &mut egui::Ui, size: egui::Vec2) -> (egui::Rect, egui::Painter) {
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = response.rect;
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(18, 18, 24));
    (rect, painter)
}

/// The projector's full range (-1..1, +Y down, as on the DAC): dots, their measured positions
/// under the solved mapping, and live detections
fn projection_view(ui: &mut egui::Ui, status: &CameraCalibrationStatus, preview: Option<&CameraStreamFrame>) {
    let (rect, painter) = view_frame(ui, egui::vec2(VIEW_SIZE, VIEW_SIZE));
    let to_screen = |p: Vec2| rect.center() + egui::vec2(p.x, p.y) * (VIEW_SIZE / 2.0);
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, ERROR.gamma_multiply(0.6)), egui::StrokeKind::Inside);

    for (i, target) in status.targets.iter().enumerate() {
        let at = to_screen(*target);
        if status.current == Some(i) {
            painter.circle_filled(at, 6.0, CURRENT);
        } else if status.measured.get(i).is_some_and(Option::is_some) {
            painter.circle_filled(at, 4.0, MEASURED);
        } else {
            painter.circle_stroke(at, 5.0, egui::Stroke::new(1.0, TARGET));
        }
        // Residual: where the dot lands under the solved mapping
        if let Some(Some(predicted)) = status.predicted.get(i) {
            painter.line_segment([at, to_screen(*predicted)], egui::Stroke::new(1.5, ERROR));
        }
    }
    for marker in preview.map(|p| p.detected_events.as_slice()).unwrap_or_default() {
        cross(&painter, to_screen(marker.galvo_pos), 6.0, LIVE);
    }
}

/// The camera image (normalised 0..1, top-left origin): measured dots, the projector's range as
/// the camera sees it, and live detections
fn camera_view(ui: &mut egui::Ui, status: &CameraCalibrationStatus, preview: Option<&CameraStreamFrame>) {
    let aspect = preview.filter(|p| p.width > 0).map(|p| p.height as f32 / p.width as f32).unwrap_or(0.75);
    let (rect, painter) = view_frame(ui, egui::vec2(VIEW_SIZE, VIEW_SIZE * aspect));
    let to_screen = |p: Vec2| rect.min + egui::vec2(p.x * rect.width(), p.y * rect.height());
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, TARGET), egui::StrokeKind::Inside);

    if let Some(quad) = preview.and_then(|p| p.calibration_quad) {
        painter.add(egui::Shape::closed_line(quad.iter().map(|p| to_screen(*p)).collect(), egui::Stroke::new(1.5, PROJECTOR)));
    }
    for measured in status.measured.iter().flatten() {
        painter.circle_filled(to_screen(*measured), 4.0, MEASURED);
    }
    for marker in preview.map(|p| p.detected_events.as_slice()).unwrap_or_default() {
        cross(&painter, to_screen(marker.camera_norm_pos), 6.0, LIVE);
    }
}

fn calibration_panel(
    mut contexts: EguiContexts,
    mut open: ResMut<CameraCalibrationOpen>,
    calibration: Res<CameraCalibration>,
    mut to_server: MessageWriter<ToServer>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let mut still_open = true;
    let mut request = None;
    let status = &calibration.status;
    let preview = calibration.preview.as_ref();
    let running = status.phase == CameraCalibrationPhase::Running;

    egui::Window::new("📷 Camera Calibration").open(&mut still_open).default_width(520.0).show(ctx, |ui| {
        let (phase, color) = match status.phase {
            CameraCalibrationPhase::Idle => ("Idle", TARGET),
            CameraCalibrationPhase::Running => ("Running", CURRENT),
            CameraCalibrationPhase::Solved => ("Solved", MEASURED),
            CameraCalibrationPhase::Failed => ("Failed", ERROR),
        };
        ui.horizontal(|ui| {
            ui.colored_label(color, phase);
            ui.label(&status.message);
        });
        match (status.calibrated, status.rms_error_m, status.max_error_m) {
            (true, Some(rms), Some(max)) => {
                ui.colored_label(MEASURED, format!("Calibrated: error {:.1} cm RMS, {:.1} cm max", rms * 100.0, max * 100.0));
            }
            (true, ..) => {
                ui.colored_label(MEASURED, "Calibrated");
            }
            _ => {
                ui.colored_label(PROJECTOR, "Not calibrated: positions assume the camera sees exactly the projector range");
            }
        }

        ui.horizontal(|ui| {
            if ui.add_enabled(!running, egui::Button::new("▶ Start")).on_hover_text("Project dots over the scene and measure them").clicked() {
                request = Some(NetworkMessage::StartCameraCalibration);
            }
            if ui.add_enabled(running, egui::Button::new("⏹ Cancel")).clicked() {
                request = Some(NetworkMessage::CancelCameraCalibration);
            }
            if ui
                .add_enabled(status.calibrated && !running, egui::Button::new("↺ Forget calibration"))
                .on_hover_text("Delete the saved calibration and use the uncalibrated default")
                .clicked()
            {
                request = Some(NetworkMessage::ResetCameraCalibration);
            }
        });
        if running && !status.targets.is_empty() {
            let done = status.current.unwrap_or(0);
            ui.add(egui::ProgressBar::new(done as f32 / status.targets.len() as f32).text(format!("dot {} of {}", done + 1, status.targets.len())));
        }

        ui.separator();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("Projection (red: projector range)");
                projection_view(ui, status, preview);
            });
            ui.vertical(|ui| {
                ui.label("Camera (orange: projector range)");
                camera_view(ui, status, preview);
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(TARGET, "○ dot");
            ui.colored_label(CURRENT, "● measuring");
            ui.colored_label(MEASURED, "● measured");
            ui.colored_label(ERROR, "— error");
            ui.colored_label(LIVE, "+ detected now");
        });

        ui.separator();
        ui.label("Detected now (position on the scene, metres from its centre, +Y up)");
        let markers = preview.map(|p| p.detected_events.as_slice()).unwrap_or_default();
        if markers.is_empty() {
            ui.weak(if preview.is_some() { "Nothing detected" } else { "Waiting for the camera…" });
        } else {
            egui::Grid::new("camera_detections").striped(true).show(ui, |ui| {
                for header in ["Label", "Track", "X m", "Y m", "Confidence"] {
                    ui.strong(header);
                }
                ui.end_row();
                for marker in markers {
                    ui.label(&marker.label);
                    ui.label(marker.track.to_string());
                    match marker.scene_pos {
                        Some(pos) => {
                            ui.label(format!("{:+.3}", pos.x));
                            ui.label(format!("{:+.3}", pos.y));
                        }
                        None => {
                            ui.label("—");
                            ui.label("—");
                        }
                    }
                    ui.label(format!("{:.0}%", marker.confidence * 100.0));
                    ui.end_row();
                }
            });
        }

        ui.separator();
        ui.weak("Dim the room and make sure the camera sees the whole scene. The dots are green, the colour the camera detects.");
    });

    if let Some(message) = request {
        to_server.write(ToServer(message));
    }
    if !still_open {
        **open = false;
    }
}
