use common::path::LineStyle;
use common::shapes::{ShapePoint, ShapeTemplate};
use eframe::egui;
use laserlogic::helios::{HeliosDacController, HeliosPoint};
use laserlogic::{LaserPoint, LaserSegment, OptimizeConfig};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_SHAPE_PATH: &str = "assets/shapes/templates/active_shape.json";

fn main() -> eframe::Result<()> {
    pretty_env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Laser Shape Studio — Reusable Laserlogic & Helios DAC")
            .with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Laser Shape Studio",
        options,
        Box::new(|_cc| Ok(Box::new(ShapeEditorApp::default()))),
    )
}

struct ShapeEditorApp {
    file_path: PathBuf,
    template: ShapeTemplate,
    raw_json: String,
    last_modified: Option<SystemTime>,
    json_error: Option<String>,

    // Laserlogic optimization config
    optimize_config: OptimizeConfig,
    optimized_point_count: usize,

    // Shared Helios DAC Thread Controls
    dac_points: Arc<Mutex<Vec<HeliosPoint>>>,
    dac_laser_on: Arc<AtomicBool>,
    dac_status_msg: Arc<Mutex<String>>,
    dac_connected: Arc<AtomicBool>,

    // Canvas interaction
    dragged_point_idx: Option<usize>,
}

impl Default for ShapeEditorApp {
    fn default() -> Self {
        let path = PathBuf::from(DEFAULT_SHAPE_PATH);
        let mut template = ShapeTemplate {
            name: "star".to_string(),
            description: "Interactive local shape test".to_string(),
            tags: vec!["custom".to_string()],
            author: "User & Copilot".to_string(),
            line_style: LineStyle::Continuous,
            points: vec![
                ShapePoint::new(0.0, 0.5, 255, 0, 0, 3),
                ShapePoint::new(-0.4, -0.4, 0, 255, 0, 3),
                ShapePoint::new(0.4, -0.4, 0, 0, 255, 3),
                ShapePoint::new(0.0, 0.5, 255, 0, 0, 3),
            ],
        };
        let mut raw_json = template.to_json().unwrap_or_default();
        let mut last_modified = None;

        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(metadata) = fs::metadata(&path) {
                    last_modified = metadata.modified().ok();
                }
                if let Ok(loaded) = ShapeTemplate::from_json(&contents) {
                    template = loaded;
                    raw_json = contents;
                }
            }
        } else {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(&path, &raw_json);
            if let Ok(metadata) = fs::metadata(&path) {
                last_modified = metadata.modified().ok();
            }
        }

        let optimize_config = OptimizeConfig::default();
        let initial_helios = build_optimized_helios_points(&template, &optimize_config);
        let optimized_point_count = initial_helios.len();

        let dac_points = Arc::new(Mutex::new(initial_helios));
        let dac_laser_on = Arc::new(AtomicBool::new(true));
        let dac_status_msg = Arc::new(Mutex::new("Initializing USB DAC...".to_string()));
        let dac_connected = Arc::new(AtomicBool::new(false));

        // Launch background thread using laserlogic::helios::HeliosDacController
        let points_clone = Arc::clone(&dac_points);
        let laser_on_clone = Arc::clone(&dac_laser_on);
        let status_clone = Arc::clone(&dac_status_msg);
        let conn_clone = Arc::clone(&dac_connected);

        thread::spawn(move || {
            match HeliosDacController::new() {
                Ok(mut dac) => {
                    match dac.open_devices() {
                        Ok(num) if num > 0 => {
                            *status_clone.lock().unwrap() = format!("✓ USB Helios DAC Connected ({} Device Found)", num);
                            conn_clone.store(true, Ordering::Relaxed);
                            let _ = dac.set_shutter(0, true);

                            let pps = 30000;
                            let min_pts = 1024;

                            while conn_clone.load(Ordering::Relaxed) {
                                let is_on = laser_on_clone.load(Ordering::Relaxed);
                                let pts = if is_on {
                                    points_clone.lock().unwrap().clone()
                                } else {
                                    vec![HeliosPoint::blanked(2048, 2048)]
                                };

                                let _ = dac.write_frame_ready(0, pps, 0, &pts, min_pts);
                            }
                        }
                        _ => {
                            *status_clone.lock().unwrap() = "✗ No USB Helios DAC hardware detected on this PC".to_string();
                        }
                    }
                }
                Err(e) => {
                    *status_clone.lock().unwrap() = format!("✗ DAC Init Warning: {} (Running local UI preview)", e);
                }
            }
        });

        Self {
            file_path: path,
            template,
            raw_json,
            last_modified,
            json_error: None,
            optimize_config,
            optimized_point_count,
            dac_points,
            dac_laser_on,
            dac_status_msg,
            dac_connected,
            dragged_point_idx: None,
        }
    }
}

impl ShapeEditorApp {
    fn check_file_watcher(&mut self) {
        if !self.file_path.exists() {
            return;
        }

        if let Ok(metadata) = fs::metadata(&self.file_path) {
            if let Ok(modified) = metadata.modified() {
                if self.last_modified.map_or(true, |last| modified > last) {
                    self.last_modified = Some(modified);
                    if let Ok(contents) = fs::read_to_string(&self.file_path) {
                        if contents != self.raw_json {
                            match ShapeTemplate::from_json(&contents) {
                                Ok(template) => {
                                    self.template = template;
                                    self.raw_json = contents;
                                    self.json_error = None;
                                    self.update_dac_points();
                                    log::info!("Reloaded shape JSON from disk!");
                                }
                                Err(e) => {
                                    self.json_error = Some(format!("JSON Syntax Error: {}", e));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_dac_points(&mut self) {
        let pts = build_optimized_helios_points(&self.template, &self.optimize_config);
        self.optimized_point_count = pts.len();
        *self.dac_points.lock().unwrap() = pts;
    }

    fn save_and_update(&mut self) {
        if let Ok(json) = self.template.to_json() {
            self.raw_json = json.clone();
            self.json_error = None;
            let _ = fs::write(&self.file_path, &json);
            if let Ok(metadata) = fs::metadata(&self.file_path) {
                self.last_modified = metadata.modified().ok();
            }
            self.update_dac_points();
        }
    }
}

fn build_optimized_helios_points(template: &ShapeTemplate, config: &OptimizeConfig) -> Vec<HeliosPoint> {
    let mut segments = Vec::new();
    let mut current_segment_pts = Vec::new();

    for pt in &template.points {
        let dac_x = (((pt.x + 1.0) / 2.0).clamp(0.0, 1.0) * 4095.0) as u16;
        let dac_y = (((pt.y + 1.0) / 2.0).clamp(0.0, 1.0) * 4095.0) as u16;

        let is_blanked = pt.r == 0 && pt.g == 0 && pt.b == 0;
        if is_blanked {
            if !current_segment_pts.is_empty() {
                segments.push(LaserSegment::new(current_segment_pts));
                current_segment_pts = Vec::new();
            }
        } else {
            let lp = LaserPoint::new(dac_x, dac_y, pt.r, pt.g, pt.b, 255);
            let count = (pt.dwell as usize).max(1);
            for _ in 0..count {
                current_segment_pts.push(lp);
            }
        }
    }

    if !current_segment_pts.is_empty() {
        segments.push(LaserSegment::new(current_segment_pts));
    }

    if segments.is_empty() {
        return vec![HeliosPoint::blanked(2048, 2048)];
    }

    let optimized_laser_points = laserlogic::optimize::optimize(&segments, config);
    optimized_laser_points.into_iter().map(HeliosPoint::from).collect()
}

impl eframe::App for ShapeEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(50));
        self.check_file_watcher();

        // ── Side Panel ──
        egui::SidePanel::left("control_panel")
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.heading("🎯 Laser Shape Studio (Shared Laserlogic & Helios Crate)");
                ui.separator();

                ui.group(|ui| {
                    ui.label(egui::RichText::new("⚡ Local USB Helios Laser DAC").strong());
                    let status_msg = self.dac_status_msg.lock().unwrap().clone();
                    let is_conn = self.dac_connected.load(Ordering::Relaxed);
                    let status_color = if is_conn { egui::Color32::GREEN } else { egui::Color32::LIGHT_RED };
                    ui.colored_label(status_color, &status_msg);

                    ui.horizontal(|ui| {
                        let mut laser_on = self.dac_laser_on.load(Ordering::Relaxed);
                        if ui.checkbox(&mut laser_on, "Laser Output Shutter Enabled").changed() {
                            self.dac_laser_on.store(laser_on, Ordering::Relaxed);
                        }
                    });
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("🔬 Server Laserlogic Optimizer Telemetry").strong());
                    ui.horizontal(|ui| {
                        ui.label(format!("Input Vertices: {}", self.template.points.len()));
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Optimized DAC Points: {}", self.optimized_point_count)).strong().color(egui::Color32::LIGHT_GREEN));
                    });

                    ui.collapsing("⚙️ Laserlogic Optimization Parameters", |ui| {
                        let mut changed = false;
                        let mut corner_dwell = self.optimize_config.corner_dwell_points;
                        if ui.add(egui::DragValue::new(&mut corner_dwell).range(0..=15).prefix("Corner Dwells:")).changed() {
                            self.optimize_config.corner_dwell_points = corner_dwell;
                            changed = true;
                        }

                        let mut blank_end = self.optimize_config.blank_end_dwell;
                        if ui.add(egui::DragValue::new(&mut blank_end).range(0..=30).prefix("Blank End Dwells:")).changed() {
                            self.optimize_config.blank_end_dwell = blank_end;
                            changed = true;
                        }

                        let mut blank_start = self.optimize_config.blank_start_dwell;
                        if ui.add(egui::DragValue::new(&mut blank_start).range(0..=30).prefix("Blank Start Dwells:")).changed() {
                            self.optimize_config.blank_start_dwell = blank_start;
                            changed = true;
                        }

                        let mut jump_steps = self.optimize_config.blank_jump_steps;
                        if ui.add(egui::DragValue::new(&mut jump_steps).range(10..=120).prefix("Blank Jump Steps:")).changed() {
                            self.optimize_config.blank_jump_steps = jump_steps;
                            changed = true;
                        }

                        if changed {
                            self.update_dac_points();
                        }
                    });
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("📁 Shape Presets").strong());
                    ui.horizontal(|ui| {
                        if ui.button("Star").clicked() {
                            load_preset(self, "assets/shapes/templates/star.json");
                        }
                        if ui.button("Crosshair").clicked() {
                            load_preset(self, "assets/shapes/templates/crosshair.json");
                        }
                        if ui.button("Circle").clicked() {
                            load_preset(self, "assets/shapes/templates/circle.json");
                        }
                        if ui.button("Target").clicked() {
                            load_preset(self, "assets/shapes/templates/target.json");
                        }
                        if ui.button("Diamond").clicked() {
                            load_preset(self, "assets/shapes/templates/diamond.json");
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("💾 Save Local JSON").clicked() {
                            self.save_and_update();
                        }
                        if ui.button("➕ Add Point").clicked() {
                            self.template.points.push(ShapePoint::new(0.0, 0.0, 255, 255, 255, 3));
                            self.save_and_update();
                        }
                    });
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("✏️ Point Geometry Inspector").strong());
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        if ui.text_edit_singleline(&mut self.template.name).changed() {
                            self.save_and_update();
                        }
                    });

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let mut to_remove = None;
                            let mut changed = false;

                            for (idx, pt) in self.template.points.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("#{}", idx));
                                    let mut x = pt.x;
                                    let mut y = pt.y;
                                    if ui.add(egui::DragValue::new(&mut x).speed(0.01).range(-2.0..=2.0).prefix("X:")).changed() {
                                        pt.x = x;
                                        changed = true;
                                    }
                                    if ui.add(egui::DragValue::new(&mut y).speed(0.01).range(-2.0..=2.0).prefix("Y:")).changed() {
                                        pt.y = y;
                                        changed = true;
                                    }

                                    let mut color = [pt.r, pt.g, pt.b];
                                    if ui.color_edit_button_srgb(&mut color).changed() {
                                        pt.r = color[0];
                                        pt.g = color[1];
                                        pt.b = color[2];
                                        changed = true;
                                    }

                                    let mut dwell = pt.dwell;
                                    if ui.add(egui::DragValue::new(&mut dwell).range(0..=20).prefix("Dwell:")).changed() {
                                        pt.dwell = dwell;
                                        changed = true;
                                    }

                                    if ui.button("❌").clicked() {
                                        to_remove = Some(idx);
                                    }
                                });
                            }

                            if let Some(idx) = to_remove {
                                if self.template.points.len() > 1 {
                                    self.template.points.remove(idx);
                                    changed = true;
                                }
                            }

                            if changed {
                                if let Ok(json) = self.template.to_json() {
                                    self.raw_json = json.clone();
                                    self.json_error = None;
                                    let _ = fs::write(&self.file_path, &json);
                                    self.update_dac_points();
                                }
                            }
                        });
                });

                ui.add_space(6.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new("📝 Raw Shape JSON (Copilot Live Sync)").strong());
                    if let Some(ref err) = self.json_error {
                        ui.colored_label(egui::Color32::RED, err);
                    }

                    let mut json_text = self.raw_json.clone();
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut json_text)
                            .font(egui::TextStyle::Monospace)
                            .code_editor()
                            .desired_rows(10)
                            .desired_width(f32::INFINITY),
                    );

                    if response.changed() {
                        self.raw_json = json_text.clone();
                        match ShapeTemplate::from_json(&json_text) {
                            Ok(template) => {
                                self.template = template;
                                self.json_error = None;
                                let _ = fs::write(&self.file_path, &json_text);
                                self.update_dac_points();
                            }
                            Err(e) => {
                                self.json_error = Some(format!("Syntax Error: {}", e));
                            }
                        }
                    }
                });
            });

        // ── Central Panel ──
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎨 Interactive 2D Vector Canvas (Drag Vertices with Mouse)");

            let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
            let rect = response.rect;
            let center = rect.center();
            let scale = (rect.width().min(rect.height()) / 2.8).max(50.0);

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 24));

            let grid_color = egui::Color32::from_rgb(45, 45, 60);
            painter.line_segment(
                [egui::pos2(rect.left(), center.y), egui::pos2(rect.right(), center.y)],
                egui::Stroke::new(1.0, grid_color),
            );
            painter.line_segment(
                [egui::pos2(center.x, rect.top()), egui::pos2(center.x, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );

            let min_box = egui::pos2(center.x - scale, center.y - scale);
            let max_box = egui::pos2(center.x + scale, center.y + scale);
            painter.rect_stroke(
                egui::Rect::from_min_max(min_box, max_box),
                0.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)),
                egui::StrokeKind::Outside,
            );

            let shape_to_screen = |x: f32, y: f32| -> egui::Pos2 {
                egui::pos2(center.x + x * scale, center.y - y * scale)
            };

            let screen_to_shape = |pos: egui::Pos2| -> (f32, f32) {
                let x = (pos.x - center.x) / scale;
                let y = (center.y - pos.y) / scale;
                (x, y)
            };

            if response.drag_started() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let mut closest_idx = None;
                    let mut closest_dist = 25.0 * 25.0;
                    for (idx, pt) in self.template.points.iter().enumerate() {
                        let screen_pos = shape_to_screen(pt.x, pt.y);
                        let dist_sq = screen_pos.distance_sq(pointer_pos);
                        if dist_sq < closest_dist {
                            closest_dist = dist_sq;
                            closest_idx = Some(idx);
                        }
                    }
                    self.dragged_point_idx = closest_idx;
                }
            }

            if response.dragged() {
                if let Some(idx) = self.dragged_point_idx {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let (new_x, new_y) = screen_to_shape(pointer_pos);
                        if idx < self.template.points.len() {
                            self.template.points[idx].x = (new_x * 100.0).round() / 100.0;
                            self.template.points[idx].y = (new_y * 100.0).round() / 100.0;
                        }
                    }
                }
            }

            if response.drag_stopped() {
                if self.dragged_point_idx.is_some() {
                    self.dragged_point_idx = None;
                    self.save_and_update();
                }
            }

            let points = &self.template.points;
            if points.len() >= 2 {
                for i in 0..points.len() - 1 {
                    let p1 = &points[i];
                    let p2 = &points[i + 1];
                    let pos1 = shape_to_screen(p1.x, p1.y);
                    let pos2 = shape_to_screen(p2.x, p2.y);

                    if p1.r == 0 && p1.g == 0 && p1.b == 0 {
                        painter.line_segment([pos1, pos2], egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 65)));
                    } else {
                        let line_color = egui::Color32::from_rgb(p1.r, p1.g, p1.b);
                        painter.line_segment([pos1, pos2], egui::Stroke::new(2.5, line_color));
                    }
                }

                for (idx, pt) in points.iter().enumerate() {
                    let screen_pos = shape_to_screen(pt.x, pt.y);
                    let dot_color = egui::Color32::from_rgb(pt.r, pt.g, pt.b);
                    let is_being_dragged = self.dragged_point_idx == Some(idx);
                    let radius = if is_being_dragged { 8.0 } else { 5.0 };

                    painter.circle_filled(screen_pos, radius, dot_color);
                    painter.circle_stroke(screen_pos, radius, egui::Stroke::new(1.0, egui::Color32::WHITE));

                    let text_pos = screen_pos + egui::vec2(8.0, -10.0);
                    painter.text(
                        text_pos,
                        egui::Align2::LEFT_TOP,
                        format!("#{}", idx),
                        egui::FontId::monospace(12.0),
                        egui::Color32::YELLOW,
                    );
                }
            }
        });
    }
}

fn load_preset(app: &mut ShapeEditorApp, preset_path: &str) {
    let path = PathBuf::from(preset_path);
    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(template) = ShapeTemplate::from_json(&contents) {
            app.template = template;
            app.raw_json = contents.clone();
            app.json_error = None;
            let _ = fs::write(&app.file_path, &contents);
            app.update_dac_points();
        }
    }
}
