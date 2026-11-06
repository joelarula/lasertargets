//! The shape editor: edits one laser shape on a 2D canvas, loads and saves it as JSON templates,
//! and previews it on the scene. While open, the server projects the edited shape (receiver mode).
//! The active file is reloaded when it changes on disk, so external edits show up live.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use common::network::{NetworkMessage, ToServer};
use common::path::{PathPoint, PathRenderable, PathSegment, UniversalPath};
use common::scene::{SceneEntity, SceneSystemSet};
use serde::{Deserialize, Serialize};
use terminal_ui::{Docking, ToolbarAppExt, ToolbarItem, toggled_on};

const TEMPLATE_DIR: &str = "assets/shapes/templates";
const DISK_POLL: Duration = Duration::from_millis(500);

/// One point of a shape, in the projector's -1..1 range
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditablePoint {
    x: f32,
    y: f32,
    r: u8,
    g: u8,
    b: u8,
    dwell: u8,
}

const fn point(x: f32, y: f32, (r, g, b): (u8, u8, u8), dwell: u8) -> EditablePoint {
    EditablePoint { x, y, r, g, b, dwell }
}

/// A shape template as stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditableShape {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_line_style")]
    line_style: String,
    points: Vec<EditablePoint>,
}

fn default_line_style() -> String {
    "Continuous".to_string()
}

impl EditableShape {
    fn new(name: impl Into<String>, description: impl Into<String>, points: Vec<EditablePoint>) -> Self {
        Self { name: name.into(), description: description.into(), line_style: default_line_style(), points }
    }

    /// A square in one colour, to check one laser diode
    fn diode_test(colour: &str, rgb: (u8, u8, u8)) -> Self {
        let corners = [(-0.7, 0.7), (0.7, 0.7), (0.7, -0.7), (-0.7, -0.7), (-0.7, 0.7)];
        Self::new(
            format!("{colour} Diode Test"),
            format!("Pure {colour} Modulation Test"),
            corners.map(|(x, y)| point(x, y, rgb, 4)).into(),
        )
    }

    /// Every colour in a corner, then both axes, to check the whole cable
    fn rgb_xy_test() -> Self {
        Self::new(
            "Full RGB & XY Cable Test",
            "Top-Left Red, Top-Right Green, Bottom-Right Blue, Bottom-Left White",
            vec![
                point(-0.8, 0.8, (255, 0, 0), 5),
                point(0.8, 0.8, (0, 255, 0), 5),
                point(0.8, -0.8, (0, 0, 255), 5),
                point(-0.8, -0.8, (255, 255, 255), 5),
                point(-0.8, 0.8, (255, 0, 0), 5),
                point(0.0, 0.8, (255, 255, 0), 3),
                point(0.0, -0.8, (0, 255, 255), 3),
                point(-0.8, 0.0, (255, 0, 255), 3),
                point(0.8, 0.0, (255, 255, 255), 3),
            ],
        )
    }

    fn to_path(&self) -> UniversalPath {
        let points = self.points.iter().map(|p| PathPoint::new(p.x, p.y, p.r, p.g, p.b, p.dwell)).collect();
        UniversalPath::from_segment(PathSegment::new(points))
    }
}

impl Default for EditableShape {
    fn default() -> Self {
        Self::new(
            "New Shape",
            "Single projection shape",
            vec![
                point(0.0, 0.4, (255, 0, 0), 3),
                point(-0.4, -0.4, (0, 255, 0), 3),
                point(0.4, -0.4, (0, 0, 255), 3),
                point(0.0, 0.4, (255, 0, 0), 3),
            ],
        )
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct ShapeEditorOpen(bool);

#[derive(Resource)]
struct ShapeEditor {
    shape: EditableShape,
    file: String,
    selected: Option<usize>,
    live_preview: bool,
    status: String,
    templates: Vec<String>,
    /// Modification time of `file` when last loaded or saved
    file_modified: Option<SystemTime>,
}

impl Default for ShapeEditor {
    fn default() -> Self {
        Self {
            shape: default(),
            file: format!("{TEMPLATE_DIR}/active_shape.json"),
            selected: None,
            live_preview: true,
            status: "Shape Editor Ready".to_string(),
            templates: scan_templates(),
            file_modified: None,
        }
    }
}

fn scan_templates() -> Vec<String> {
    let Ok(entries) = fs::read_dir(TEMPLATE_DIR) else { return Vec::new() };
    let mut templates: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| path.to_str().map(|p| p.replace('\\', "/")))
        .collect();
    templates.sort();
    templates
}

fn modified(path: &str) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn file_name(path: &str) -> &str {
    Path::new(path).file_name().and_then(|name| name.to_str()).unwrap_or(path)
}

impl ShapeEditor {
    fn load(&mut self, file: &str) -> bool {
        let loaded = fs::read_to_string(file)
            .map_err(|e| format!("Read Error: {e}"))
            .and_then(|json| serde_json::from_str(&json).map_err(|e| format!("JSON Error: {e}")));
        match loaded {
            Ok(shape) => {
                self.shape = shape;
                self.file = file.to_string();
                self.file_modified = modified(file);
                self.status = format!("Loaded {file}");
                true
            }
            Err(e) => {
                self.status = e;
                false
            }
        }
    }

    fn save(&mut self) {
        let saved = serde_json::to_string_pretty(&self.shape)
            .map_err(|e| format!("Serialize Error: {e}"))
            .and_then(|json| {
                if let Some(parent) = Path::new(&self.file).parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fs::write(&self.file, json).map_err(|e| format!("Write Error: {e}"))
            });
        self.status = match saved {
            Ok(()) => {
                self.file_modified = modified(&self.file);
                "Saved to disk".to_string()
            }
            Err(e) => e,
        };
    }

    fn show(&mut self, shape: EditableShape, status: &str) {
        self.shape = shape;
        self.status = status.to_string();
    }
}

/// The edited shape, drawn on the scene
#[derive(Component)]
struct EditorPreview;

pub struct ShapeEditorPlugin;

impl Plugin for ShapeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeEditor>()
            .add_toolbar_toggle::<ShapeEditorOpen>(
                ToolbarItem::new("shape_editor").icon("\u{f044}").text("Shape Editor").order(3).dock(Docking::Left),
            )
            .add_systems(Startup, spawn_preview.after(SceneSystemSet))
            .add_systems(
                Update,
                (
                    set_receiver_mode.run_if(resource_changed::<ShapeEditorOpen>),
                    update_preview.run_if(resource_changed::<ShapeEditor>),
                    reload_from_disk.run_if(on_timer(DISK_POLL)),
                ),
            )
            .add_systems(EguiPrimaryContextPass, editor_panel.run_if(toggled_on::<ShapeEditorOpen>));
    }
}

/// While the editor is open the server projects its shape instead of the game
fn set_receiver_mode(open: Res<ShapeEditorOpen>, editor: Res<ShapeEditor>, mut to_server: MessageWriter<ToServer>) {
    if !open.is_added() {
        to_server.write(ToServer(NetworkMessage::SetReceiverMode {
            active: **open,
            source_name: Some(editor.shape.name.clone()),
        }));
    }
}

fn spawn_preview(mut commands: Commands, scene: Query<Entity, With<SceneEntity>>) {
    let preview = commands
        .spawn((UniversalPath::new(), Transform::from_xyz(0.0, 3.0, -10.0), PathRenderable { visible: true }, EditorPreview))
        .id();
    if let Ok(scene) = scene.single() {
        commands.entity(scene).add_child(preview);
    }
}

fn update_preview(editor: Res<ShapeEditor>, mut previews: Query<(&mut UniversalPath, &mut PathRenderable), With<EditorPreview>>) {
    for (mut path, mut renderable) in &mut previews {
        renderable.visible = editor.live_preview;
        if editor.live_preview {
            *path = editor.shape.to_path();
        }
    }
}

fn reload_from_disk(mut editor: ResMut<ShapeEditor>) {
    let file = editor.file.clone();
    let Some(on_disk) = modified(&file) else { return };
    if editor.file_modified.is_none_or(|known| on_disk > known) && editor.load(&file) {
        editor.status = "⚡ Auto-reloaded disk update".to_string();
    }
}

fn editor_panel(mut contexts: EguiContexts, mut open: ResMut<ShapeEditorOpen>, mut editor: ResMut<ShapeEditor>) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    let mut still_open = true;

    egui::Window::new("✏️ Shape Editor (Single Projection)")
        .open(&mut still_open)
        .default_size([420.0, 520.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut editor.live_preview, "⚡ Live Projection Preview");
                ui.label(format!("Status: {}", editor.status));
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Templates:");
                let templates = editor.templates.clone();
                egui::ComboBox::from_id_salt("template_select")
                    .selected_text(file_name(&editor.file).to_string())
                    .show_ui(ui, |ui| {
                        for template in &templates {
                            if ui.selectable_label(editor.file == *template, file_name(template)).clicked() {
                                editor.load(template);
                            }
                        }
                    });
                if ui.button("🔄 Rescan").clicked() {
                    editor.templates = scan_templates();
                }
            });

            ui.horizontal(|ui| {
                ui.label("🔬 Cable Tests:");
                let tests = [
                    ("🔴 Red", "Red", (255, 0, 0), "🔴 Red Diode Test Active"),
                    ("🟢 Green", "Green", (0, 255, 0), "🟢 Green Diode Test Active"),
                    ("🔵 Blue", "Blue", (0, 0, 255), "🔵 Blue Diode Test Active"),
                ];
                for (button, colour, rgb, status) in tests {
                    if ui.button(button).clicked() {
                        editor.show(EditableShape::diode_test(colour, rgb), status);
                    }
                }
                if ui.button("🌈 Full RGB/XY").clicked() {
                    editor.show(EditableShape::rgb_xy_test(), "🔬 Full RGB & XY Cable Test Loaded");
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("File:");
                ui.text_edit_singleline(&mut editor.file);
                if ui.button("📂 Load").clicked() {
                    let file = editor.file.clone();
                    editor.load(&file);
                }
                if ui.button("💾 Save").clicked() {
                    editor.save();
                }
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut editor.shape.name);
            });
            ui.separator();

            ui.heading("2D Vector Canvas (Drag Vertices)");
            canvas(ui, &mut editor);
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("➕ Add Point").clicked() {
                    editor.shape.points.push(point(0.0, 0.0, (255, 255, 255), 3));
                    editor.selected = Some(editor.shape.points.len() - 1);
                }
                if ui.button("🗑️ Delete Point").clicked()
                    && let Some(index) = editor.selected.take()
                    && index < editor.shape.points.len()
                {
                    editor.shape.points.remove(index);
                }
                if ui.button("🔄 Close Loop").clicked()
                    && let Some(first) = editor.shape.points.first().cloned()
                {
                    editor.shape.points.push(first);
                }
            });
            ui.separator();

            point_details(ui, &mut editor);
        });

    if !still_open {
        **open = false;
    }
}

/// The shape on a canvas; vertices can be clicked to select and dragged to move
fn canvas(ui: &mut egui::Ui, editor: &mut ShapeEditor) {
    let (response, painter) = ui.allocate_painter(egui::vec2(380.0, 220.0), egui::Sense::drag());
    let rect = response.rect;
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 25));
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)), egui::StrokeKind::Outside);

    // -1..1 with +Y up, to canvas pixels with +Y down, and back
    let to_canvas = |p: &EditablePoint| {
        egui::pos2(rect.min.x + (p.x + 1.0) * 0.5 * rect.width(), rect.min.y + (1.0 - p.y) * 0.5 * rect.height())
    };
    let to_shape = |pos: egui::Pos2| {
        let x = (pos.x - rect.min.x) / rect.width() * 2.0 - 1.0;
        let y = 1.0 - (pos.y - rect.min.y) / rect.height() * 2.0;
        // Rounded to hundredths so saved templates stay readable
        let round = |v: f32| (v.clamp(-1.0, 1.0) * 100.0).round() / 100.0;
        (round(x), round(y))
    };

    let points = &editor.shape.points;
    for pair in points.windows(2) {
        let color = egui::Color32::from_rgb(pair[0].r, pair[0].g, pair[0].b);
        painter.line_segment([to_canvas(&pair[0]), to_canvas(&pair[1])], egui::Stroke::new(2.0, color));
    }

    let pointer = response.interact_pointer_pos();
    let mut grabbed = None;
    for (index, p) in points.iter().enumerate() {
        let at = to_canvas(p);
        let color = if editor.selected == Some(index) { egui::Color32::YELLOW } else { egui::Color32::from_rgb(p.r, p.g, p.b) };
        painter.circle_filled(at, 6.0, color);
        painter.circle_stroke(at, 8.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
        let handle = egui::Rect::from_center_size(at, egui::vec2(16.0, 16.0));
        if (response.dragged() || response.clicked()) && pointer.is_some_and(|pos| handle.contains(pos)) {
            grabbed = Some(index);
        }
    }

    if let Some(index) = grabbed {
        editor.selected = Some(index);
        if response.dragged()
            && let Some(pos) = pointer
        {
            let (x, y) = to_shape(pos);
            editor.shape.points[index].x = x;
            editor.shape.points[index].y = y;
        }
    }
}

fn point_details(ui: &mut egui::Ui, editor: &mut ShapeEditor) {
    let Some(index) = editor.selected.filter(|&i| i < editor.shape.points.len()) else { return };
    ui.label(format!("Editing Point #{}", index + 1));
    let point = &mut editor.shape.points[index];
    ui.horizontal(|ui| {
        ui.label("X:");
        ui.add(egui::DragValue::new(&mut point.x).speed(0.01).range(-1.0..=1.0));
        ui.label("Y:");
        ui.add(egui::DragValue::new(&mut point.y).speed(0.01).range(-1.0..=1.0));
    });
    ui.horizontal(|ui| {
        ui.label("Color:");
        let mut rgb = [point.r, point.g, point.b].map(|c| c as f32 / 255.0);
        if ui.color_edit_button_rgb(&mut rgb).changed() {
            [point.r, point.g, point.b] = rgb.map(|c| (c * 255.0) as u8);
        }
        ui.label("Corner Dwell:");
        ui.add(egui::DragValue::new(&mut point.dwell).range(0..=8));
    });
}
