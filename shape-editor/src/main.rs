use common::path::LineStyle;
use common::shapes::{ShapePoint, ShapeTemplate};
use eframe::egui;
use std::fs;
use std::os::raw::{c_int, c_uchar, c_uint};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

const DEFAULT_SHAPE_PATH: &str = "assets/shapes/templates/active_shape.json";

// ── Helios DAC FFI & Native Point Structures ──────────────────────────────
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeliosPoint {
    pub x: u16, // 0 to 4095
    pub y: u16, // 0 to 4095
    pub r: u8,  // 0 to 255
    pub g: u8,  // 0 to 255
    pub b: u8,  // 0 to 255
    pub i: u8,  // Intensity, 0 to 255
}

impl HeliosPoint {
    pub fn new(x: u16, y: u16, r: u8, g: u8, b: u8, i: u8) -> Self {
        Self { x, y, r, g, b, i }
    }
    pub fn blanked(x: u16, y: u16) -> Self {
        Self { x, y, r: 0, g: 0, b: 0, i: 0 }
    }
}

type FnOpenDevices = unsafe extern "C" fn() -> c_int;
type FnCloseDevices = unsafe extern "C" fn() -> c_int;
type FnWriteFrame = unsafe extern "C" fn(c_uint, c_uint, c_uchar, *const HeliosPoint, c_uint) -> c_int;
type FnGetStatus = unsafe extern "C" fn(c_uint) -> c_int;
type FnSetShutter = unsafe extern "C" fn(c_uint, bool) -> c_int;

pub struct LocalHeliosDac {
    _lib: libloading::Library,
    open_devices: FnOpenDevices,
    close_devices: FnCloseDevices,
    write_frame: FnWriteFrame,
    get_status: FnGetStatus,
    set_shutter: FnSetShutter,
    pub num_devices: i32,
}

impl LocalHeliosDac {
    pub fn new() -> Result<Self, String> {
        let dll_names = if cfg!(target_os = "windows") {
            vec!["HeliosLaserDAC.dll", "target/debug/HeliosLaserDAC.dll"]
        } else {
            vec!["libHeliosLaserDAC.so", "/opt/lasertargets/libHeliosLaserDAC.so"]
        };

        let mut last_err = String::new();
        for path in dll_names {
            if let Ok(lib) = unsafe { libloading::Library::new(path) } {
                unsafe {
                    let open_devices: libloading::Symbol<FnOpenDevices> = lib.get(b"HeliosOpenDevices\0").map_err(|e| e.to_string())?;
                    let close_devices: libloading::Symbol<FnCloseDevices> = lib.get(b"HeliosCloseDevices\0").map_err(|e| e.to_string())?;
                    let write_frame: libloading::Symbol<FnWriteFrame> = lib.get(b"HeliosWriteFrame\0").map_err(|e| e.to_string())?;
                    let get_status: libloading::Symbol<FnGetStatus> = lib.get(b"HeliosGetStatus\0").map_err(|e| e.to_string())?;
                    let set_shutter: libloading::Symbol<FnSetShutter> = lib.get(b"HeliosSetShutter\0").map_err(|e| e.to_string())?;

                    let open_fn = *open_devices;
                    let close_fn = *close_devices;
                    let write_fn = *write_frame;
                    let status_fn = *get_status;
                    let shutter_fn = *set_shutter;

                    let mut dac = Self {
                        _lib: lib,
                        open_devices: open_fn,
                        close_devices: close_fn,
                        write_frame: write_fn,
                        get_status: status_fn,
                        set_shutter: shutter_fn,
                        num_devices: 0,
                    };
                    dac.num_devices = (dac.open_devices)();
                    return Ok(dac);
                }
            } else {
                last_err = format!("Could not load DAC DLL from {}", path);
            }
        }
        Err(last_err)
    }

    pub fn write_frame_ready(&self, dac_num: u32, pps: u32, points: &[HeliosPoint]) -> Result<bool, String> {
        if points.is_empty() { return Ok(false); }

        // Wait until status is ready or transient busy
        for _attempt in 0..60 {
            let st = unsafe { (self.get_status)(dac_num as c_uint) };
            if st == 1 {
                let res = unsafe { (self.write_frame)(dac_num as c_uint, pps, 0, points.as_ptr(), points.len() as c_uint) };
                if res >= 0 {
                    return Ok(true);
                } else {
                    return Err(format!("WriteFrame error {}", res));
                }
            } else if st == 0 || st == -1002 || st == -1000 || st == -5007 {
                thread::sleep(Duration::from_millis(1));
            } else {
                return Err(format!("GetStatus error {}", st));
            }
        }
        Ok(false)
    }

    pub fn set_shutter(&self, dac_num: u32, on: bool) {
        unsafe { (self.set_shutter)(dac_num as c_uint, on); }
    }
}

impl Drop for LocalHeliosDac {
    fn drop(&mut self) {
        unsafe {
            (self.set_shutter)(0, false);
            (self.close_devices)();
        }
    }
}

// ── Application Entry Point ───────────────────────────────────────────────
fn main() -> eframe::Result<()> {
    pretty_env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Laser Shape Studio — Local USB DAC & Interactive Editor")
            .with_inner_size([1280.0, 800.0]),
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
    
    // Local DAC Thread Controls
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

        let dac_points = Arc::new(Mutex::new(build_helios_points(&template)));
        let dac_laser_on = Arc::new(AtomicBool::new(true));
        let dac_status_msg = Arc::new(Mutex::new("Initializing USB DAC...".to_string()));
        let dac_connected = Arc::new(AtomicBool::new(false));

        // Launch background local USB DAC streaming thread
        let points_clone = Arc::clone(&dac_points);
        let laser_on_clone = Arc::clone(&dac_laser_on);
        let status_clone = Arc::clone(&dac_status_msg);
        let conn_clone = Arc::clone(&dac_connected);

        thread::spawn(move || {
            match LocalHeliosDac::new() {
                Ok(dac) => {
                    if dac.num_devices > 0 {
                        *status_clone.lock().unwrap() = format!("✓ USB Helios DAC Connected ({} Device Found)", dac.num_devices);
                        conn_clone.store(true, Ordering::Relaxed);
                        dac.set_shutter(0, true);

                        let pps = 30000;
                        let min_pts = 1024;

                        while conn_clone.load(Ordering::Relaxed) {
                            let is_on = laser_on_clone.load(Ordering::Relaxed);
                            let pts = if is_on {
                                points_clone.lock().unwrap().clone()
                            } else {
                                vec![HeliosPoint::blanked(2048, 2048)]
                            };

                            let mut frame = pts;
                            if frame.len() < min_pts {
                                let last = *frame.last().unwrap_or(&HeliosPoint::blanked(2048, 2048));
                                while frame.len() < min_pts {
                                    frame.push(HeliosPoint::blanked(last.x, last.y));
                                }
                            } else if frame.len() > min_pts {
                                frame.truncate(min_pts);
                            }

                            let _ = dac.write_frame_ready(0, pps, &frame);
                            thread::sleep(Duration::from_millis(5));
                        }
                    } else {
                        *status_clone.lock().unwrap() = "✗ No USB Helios DAC hardware detected on this PC".to_string();
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
            dac_points,
            dac_laser_on,
            dac_status_msg,
            dac_connected,
            dragged_point_idx: None,
        }
    }
}

impl ShapeEditorApp {
    /// Watch file on disk for Copilot / external edits
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

    fn update_dac_points(&self) {
        let pts = build_helios_points(&self.template);
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

fn build_helios_points(template: &ShapeTemplate) -> Vec<HeliosPoint> {
    let mut points = Vec::new();
    for pt in &template.points {
        // Map normalized x, y [-1.0, 1.0] to 12-bit DAC coords [0, 4095]
        let dac_x = (((pt.x + 1.0) / 2.0).clamp(0.0, 1.0) * 4095.0) as u16;
        let dac_y = (((pt.y + 1.0) / 2.0).clamp(0.0, 1.0) * 4095.0) as u16;
        let intensity = if pt.r > 0 || pt.g > 0 || pt.b > 0 { 255 } else { 0 };

        let hp = HeliosPoint::new(dac_x, dac_y, pt.r, pt.g, pt.b, intensity);
        
        // Add dwell points
        let count = (pt.dwell as usize).max(1);
        for _ in 0..count {
            points.push(hp);
        }
    }
    points
}

impl eframe::App for ShapeEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(50));
        self.check_file_watcher();

        // ── 1. Side Panel: Inspector & Controls ──
        egui::SidePanel::left("control_panel")
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.heading("🎯 Laser Shape Studio (Local DAC)");
                ui.separator();

                // ── Local USB Helios DAC Status ──
                ui.group(|ui| {
                    ui.label(egui::RichText::new("⚡ Local USB Helios Laser DAC").strong());
                    let status_msg = self.dac_status_msg.lock().unwrap().clone();
                    let is_conn = self.dac_connected.load(Ordering::Relaxed);
                    let status_color = if is_conn { egui::Color32::GREEN } else { egui::Color32::LIGHT_RED };
                    ui.colored_label(status_color, &status_msg);

                    let mut laser_on = self.dac_laser_on.load(Ordering::Relaxed);
                    if ui.checkbox(&mut laser_on, "Laser Output Shutter Enabled").changed() {
                        self.dac_laser_on.store(laser_on, Ordering::Relaxed);
                    }
                });

                ui.add_space(8.0);

                // ── Template Quick Load ──
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

                ui.add_space(8.0);

                // ── Point Geometry Inspector ──
                ui.group(|ui| {
                    ui.label(egui::RichText::new("✏️ Point Geometry Inspector").strong());
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        if ui.text_edit_singleline(&mut self.template.name).changed() {
                            self.save_and_update();
                        }
                    });

                    egui::ScrollArea::vertical()
                        .max_height(240.0)
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

                ui.add_space(8.0);

                // ── Raw JSON Editor (Copilot Chat Sync) ──
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
                            .desired_rows(12)
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

        // ── 2. Central Panel: Interactive 2D Vector Painter ──
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎨 Interactive 2D Vector Canvas (Drag Vertices with Mouse)");

            let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
            let rect = response.rect;
            let center = rect.center();
            let scale = (rect.width().min(rect.height()) / 2.8).max(50.0);

            // Background canvas fill
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(18, 18, 24));

            // Grid Axes
            let grid_color = egui::Color32::from_rgb(45, 45, 60);
            painter.line_segment(
                [egui::pos2(rect.left(), center.y), egui::pos2(rect.right(), center.y)],
                egui::Stroke::new(1.0, grid_color),
            );
            painter.line_segment(
                [egui::pos2(center.x, rect.top()), egui::pos2(center.x, rect.bottom())],
                egui::Stroke::new(1.0, grid_color),
            );

            // 1.0m Bounding box outline
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

            // Mouse Drag vertex interaction
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

            // Draw shape vector lines & vertices
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

                // Draw vertex dots and index numbers
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
