use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use bevy_quinnet::client::QuinnetClient;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use common::network::NetworkMessage;
use common::path::{PathPoint, PathRenderable, PathSegment, UniversalPath};
use common::scene::SceneEntity;
use common::toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem};

const BTN_NAME: &str = "shape_editor";

/// Single point data structure for JSON template loading and saving
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditablePoint {
    pub x: f32,
    pub y: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub dwell: u8,
}

/// JSON Shape Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditableShape {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_line_style")]
    pub line_style: String,
    pub points: Vec<EditablePoint>,
}

fn default_line_style() -> String {
    "Continuous".to_string()
}

impl Default for EditableShape {
    fn default() -> Self {
        Self {
            name: "New Shape".to_string(),
            description: "Single projection shape".to_string(),
            line_style: "Continuous".to_string(),
            points: vec![
                EditablePoint { x: 0.0, y: 0.4, r: 255, g: 0, b: 0, dwell: 3 },
                EditablePoint { x: -0.4, y: -0.4, r: 0, g: 255, b: 0, dwell: 3 },
                EditablePoint { x: 0.4, y: -0.4, r: 0, g: 0, b: 255, dwell: 3 },
                EditablePoint { x: 0.0, y: 0.4, r: 255, g: 0, b: 0, dwell: 3 },
            ],
        }
    }
}

/// State resource for the terminal-side Shape Editor
#[derive(Resource)]
pub struct ShapeEditorState {
    pub is_open: bool,
    pub shape: EditableShape,
    pub active_file_path: String,
    pub selected_point_idx: Option<usize>,
    pub live_project: bool,
    pub status_message: String,
    pub scanned_templates: Vec<String>,
    pub last_file_modified: Option<std::time::SystemTime>,
}

impl Default for ShapeEditorState {
    fn default() -> Self {
        let mut state = Self {
            is_open: false,
            shape: EditableShape::default(),
            active_file_path: "assets/shapes/templates/active_shape.json".to_string(),
            selected_point_idx: None,
            live_project: true,
            status_message: "Shape Editor Ready".to_string(),
            scanned_templates: Vec::new(),
            last_file_modified: None,
        };
        state.scan_templates();
        state
    }
}

impl ShapeEditorState {
    pub fn scan_templates(&mut self) {
        self.scanned_templates.clear();
        let dir = Path::new("assets/shapes/templates");
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                        if let Some(path_str) = path.to_str() {
                            self.scanned_templates.push(path_str.replace('\\', "/"));
                        }
                    }
                }
            }
        }
        self.scanned_templates.sort();
    }

    pub fn load_file(&mut self, file_path: &str) -> bool {
        let path = Path::new(file_path);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => match serde_json::from_str::<EditableShape>(&content) {
                    Ok(loaded_shape) => {
                        self.shape = loaded_shape;
                        self.active_file_path = file_path.to_string();
                        self.status_message = format!("Loaded {}", file_path);
                        if let Ok(meta) = fs::metadata(path) {
                            self.last_file_modified = meta.modified().ok();
                        }
                        return true;
                    }
                    Err(e) => self.status_message = format!("JSON Error: {}", e),
                },
                Err(e) => self.status_message = format!("Read Error: {}", e),
            }
        } else {
            self.status_message = "File not found".to_string();
        }
        false
    }

    pub fn load_red_test(&mut self) {
        self.shape = EditableShape {
            name: "Red Diode Test".to_string(),
            description: "Pure Red Modulation Test".to_string(),
            line_style: "Continuous".to_string(),
            points: vec![
                EditablePoint { x: -0.7, y: 0.7, r: 255, g: 0, b: 0, dwell: 4 },
                EditablePoint { x: 0.7, y: 0.7, r: 255, g: 0, b: 0, dwell: 4 },
                EditablePoint { x: 0.7, y: -0.7, r: 255, g: 0, b: 0, dwell: 4 },
                EditablePoint { x: -0.7, y: -0.7, r: 255, g: 0, b: 0, dwell: 4 },
                EditablePoint { x: -0.7, y: 0.7, r: 255, g: 0, b: 0, dwell: 4 },
            ],
        };
        self.status_message = "🔴 Red Diode Test Active".to_string();
    }

    pub fn load_green_test(&mut self) {
        self.shape = EditableShape {
            name: "Green Diode Test".to_string(),
            description: "Pure Green Modulation Test".to_string(),
            line_style: "Continuous".to_string(),
            points: vec![
                EditablePoint { x: -0.7, y: 0.7, r: 0, g: 255, b: 0, dwell: 4 },
                EditablePoint { x: 0.7, y: 0.7, r: 0, g: 255, b: 0, dwell: 4 },
                EditablePoint { x: 0.7, y: -0.7, r: 0, g: 255, b: 0, dwell: 4 },
                EditablePoint { x: -0.7, y: -0.7, r: 0, g: 255, b: 0, dwell: 4 },
                EditablePoint { x: -0.7, y: 0.7, r: 0, g: 255, b: 0, dwell: 4 },
            ],
        };
        self.status_message = "🟢 Green Diode Test Active".to_string();
    }

    pub fn load_blue_test(&mut self) {
        self.shape = EditableShape {
            name: "Blue Diode Test".to_string(),
            description: "Pure Blue Modulation Test".to_string(),
            line_style: "Continuous".to_string(),
            points: vec![
                EditablePoint { x: -0.7, y: 0.7, r: 0, g: 0, b: 255, dwell: 4 },
                EditablePoint { x: 0.7, y: 0.7, r: 0, g: 0, b: 255, dwell: 4 },
                EditablePoint { x: 0.7, y: -0.7, r: 0, g: 0, b: 255, dwell: 4 },
                EditablePoint { x: -0.7, y: -0.7, r: 0, g: 0, b: 255, dwell: 4 },
                EditablePoint { x: -0.7, y: 0.7, r: 0, g: 0, b: 255, dwell: 4 },
            ],
        };
        self.status_message = "🔵 Blue Diode Test Active".to_string();
    }

    pub fn load_rgb_xy_test(&mut self) {
        self.shape = EditableShape {
            name: "Full RGB & XY Cable Test".to_string(),
            description: "Top-Left Red, Top-Right Green, Bottom-Right Blue, Bottom-Left White".to_string(),
            line_style: "Continuous".to_string(),
            points: vec![
                EditablePoint { x: -0.8, y: 0.8, r: 255, g: 0, b: 0, dwell: 5 },
                EditablePoint { x: 0.8, y: 0.8, r: 0, g: 255, b: 0, dwell: 5 },
                EditablePoint { x: 0.8, y: -0.8, r: 0, g: 0, b: 255, dwell: 5 },
                EditablePoint { x: -0.8, y: -0.8, r: 255, g: 255, b: 255, dwell: 5 },
                EditablePoint { x: -0.8, y: 0.8, r: 255, g: 0, b: 0, dwell: 5 },
                EditablePoint { x: 0.0, y: 0.8, r: 255, g: 255, b: 0, dwell: 3 },
                EditablePoint { x: 0.0, y: -0.8, r: 0, g: 255, b: 255, dwell: 3 },
                EditablePoint { x: -0.8, y: 0.0, r: 255, g: 0, b: 255, dwell: 3 },
                EditablePoint { x: 0.8, y: 0.0, r: 255, g: 255, b: 255, dwell: 3 },
            ],
        };
        self.status_message = "🔬 Full RGB & XY Cable Test Loaded".to_string();
    }
}

#[derive(Component)]
pub struct ShapeEditorButton;

/// Marker component for the single edited shape preview entity in the 3D scene
#[derive(Component)]
pub struct EditorPreviewShape;

/// Plugin registering the terminal Shape Editor module
pub struct ShapeEditorPlugin;

impl Plugin for ShapeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeEditorState>()
            .add_systems(Startup, (setup_editor_preview_entity, register_editor_button))
            .add_systems(Update, (handle_editor_button, update_editor_preview_shape, auto_watch_disk_modifications))
            .add_systems(EguiPrimaryContextPass, ui_shape_editor_panel);
    }
}

/// Disk watcher system that automatically reloads active_file_path when modified on disk (live Copilot chat edits)
fn auto_watch_disk_modifications(mut editor_state: ResMut<ShapeEditorState>) {
    let path_str = editor_state.active_file_path.clone();
    let path = Path::new(&path_str);
    if path.exists() {
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                if editor_state.last_file_modified.map_or(true, |last| modified > last) {
                    if editor_state.load_file(&path_str) {
                        editor_state.status_message = "⚡ Auto-reloaded disk update".to_string();
                    }
                }
            }
        }
    }
}

fn register_editor_button(mut commands: Commands) {
    commands.spawn((
        ToolbarItem {
            name: BTN_NAME.to_string(),
            order: 3,
            text: Some("Shape Editor".to_string()),
            icon: Some("\u{f044}".to_string()),
            state: ItemState::Off,
            docking: Docking::Left,
            button_size: 36.0,
            ..default()
        },
        ShapeEditorButton,
    ));
}

fn handle_editor_button(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut editor_state: ResMut<ShapeEditorState>,
    mut editor_button_query: Query<&mut ToolbarItem, With<ShapeEditorButton>>,
    mut client: ResMut<QuinnetClient>,
) {
    for (interaction, button) in &button_query {
        if button.name == BTN_NAME && *interaction == Interaction::Pressed {
            editor_state.is_open = !editor_state.is_open;

            if let Ok(mut item) = editor_button_query.single_mut() {
                item.state = if editor_state.is_open { ItemState::On } else { ItemState::Off };
            }

            // Notify server of Receiver mode state
            if let Some(connection) = client.get_connection_mut() {
                let msg = NetworkMessage::SetReceiverMode {
                    active: editor_state.is_open,
                    source_name: Some(editor_state.shape.name.clone()),
                };
                if let Ok(payload) = msg.to_bytes() {
                    let _ = connection.send_payload(payload);
                }
            }
        }
    }
}

/// Spawns the single preview shape entity in the 3D scene
fn setup_editor_preview_entity(mut commands: Commands, scene_query: Query<Entity, With<SceneEntity>>) {
    let preview_entity = commands
        .spawn((
            UniversalPath::new(),
            Transform::from_xyz(0.0, 3.0, -10.0),
            PathRenderable { visible: true },
            EditorPreviewShape,
        ))
        .id();

    if let Ok(scene_entity) = scene_query.single() {
        commands.entity(scene_entity).add_child(preview_entity);
    }
}

/// Egui UI Panel for editing single projection shapes
fn ui_shape_editor_panel(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<ShapeEditorState>,
) {
    if !editor_state.is_open {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("✏️ Shape Editor (Single Projection)")
        .default_size([420.0, 520.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut editor_state.live_project, "⚡ Live Projection Preview");
                ui.label(format!("Status: {}", editor_state.status_message));
            });

            ui.separator();

            // Template Folder Scanner Controls
            ui.horizontal(|ui| {
                ui.label("Templates:");
                let current_file = editor_state.active_file_path.clone();
                let scanned = editor_state.scanned_templates.clone();
                let mut selected_template = current_file;

                egui::ComboBox::from_id_salt("template_select")
                    .selected_text(Path::new(&selected_template).file_name().map_or("Select...", |n| n.to_str().unwrap_or("")))
                    .show_ui(ui, |ui| {
                        for t in &scanned {
                            let filename = Path::new(t).file_name().map_or(t.as_str(), |n| n.to_str().unwrap_or(t));
                            if ui.selectable_value(&mut selected_template, t.clone(), filename).clicked() {
                                editor_state.load_file(t);
                            }
                        }
                    });

                if ui.button("🔄 Rescan").clicked() {
                    editor_state.scan_templates();
                }
            });

            // Cable Hardware Test Suite Quick Buttons
            ui.horizontal(|ui| {
                ui.label("🔬 Cable Tests:");
                if ui.button("🔴 Red").clicked() {
                    editor_state.load_red_test();
                }
                if ui.button("🟢 Green").clicked() {
                    editor_state.load_green_test();
                }
                if ui.button("🔵 Blue").clicked() {
                    editor_state.load_blue_test();
                }
                if ui.button("🌈 Full RGB/XY").clicked() {
                    editor_state.load_rgb_xy_test();
                }
            });

            ui.separator();

            // File IO Controls
            ui.horizontal(|ui| {
                ui.label("File:");
                ui.text_edit_singleline(&mut editor_state.active_file_path);

                let path_to_load = editor_state.active_file_path.clone();
                if ui.button("📂 Load").clicked() {
                    editor_state.load_file(&path_to_load);
                }

                if ui.button("💾 Save").clicked() {
                    match serde_json::to_string_pretty(&editor_state.shape) {
                        Ok(json_content) => {
                            if let Some(parent) = Path::new(&editor_state.active_file_path).parent() {
                                let _ = fs::create_dir_all(parent);
                            }
                            match fs::write(&editor_state.active_file_path, json_content) {
                                Ok(_) => {
                                    editor_state.status_message = "Saved to disk".to_string();
                                    if let Ok(meta) = fs::metadata(&editor_state.active_file_path) {
                                        editor_state.last_file_modified = meta.modified().ok();
                                    }
                                }
                                Err(e) => editor_state.status_message = format!("Write Error: {}", e),
                            }
                        }
                        Err(e) => editor_state.status_message = format!("Serialize Error: {}", e),
                    }
                }
            });

            ui.separator();

            // Metadata fields
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut editor_state.shape.name);
            });

            ui.separator();

            // Interactive 2D Painter Canvas
            ui.heading("2D Vector Canvas (Drag Vertices)");
            let (response, painter) = ui.allocate_painter(egui::vec2(380.0, 220.0), egui::Sense::drag());
            let rect = response.rect;

            // Draw Canvas Background
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 20, 25));
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 60, 80)),
                egui::StrokeKind::Outside,
            );

            // Convert normalized [-1.0, 1.0] coordinates to Canvas Pixels
            let to_canvas = |x: f32, y: f32| -> egui::Pos2 {
                let px = rect.min.x + (x + 1.0) * 0.5 * rect.width();
                let py = rect.min.y + (1.0 - (y + 1.0) * 0.5) * rect.height(); // Flip Y
                egui::pos2(px, py)
            };

            let to_norm = |pos: egui::Pos2| -> (f32, f32) {
                let x = (pos.x - rect.min.x) / rect.width() * 2.0 - 1.0;
                let y = (1.0 - (pos.y - rect.min.y) / rect.height()) * 2.0 - 1.0;
                (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0))
            };

            // Draw Lines connecting points
            let pts = &editor_state.shape.points;
            if pts.len() >= 2 {
                for i in 0..pts.len() - 1 {
                    let p1 = to_canvas(pts[i].x, pts[i].y);
                    let p2 = to_canvas(pts[i + 1].x, pts[i + 1].y);
                    let color = egui::Color32::from_rgb(pts[i].r, pts[i].g, pts[i].b);
                    painter.line_segment([p1, p2], egui::Stroke::new(2.0, color));
                }
            }

            // Draw & Drag Vertex Control Handles
            let mut dragged_idx = None;
            let mut clicked_idx = None;
            for (idx, p) in editor_state.shape.points.iter().enumerate() {
                let pos = to_canvas(p.x, p.y);
                let is_selected = editor_state.selected_point_idx == Some(idx);

                let handle_color = if is_selected {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::from_rgb(p.r, p.g, p.b)
                };

                painter.circle_filled(pos, 6.0, handle_color);
                painter.circle_stroke(pos, 8.0, egui::Stroke::new(1.5, egui::Color32::WHITE));

                let handle_rect = egui::Rect::from_center_size(pos, egui::vec2(16.0, 16.0));
                if response.dragged() && response.interact_pointer_pos().map_or(false, |p| handle_rect.contains(p)) {
                    dragged_idx = Some(idx);
                }
                if response.clicked() && response.interact_pointer_pos().map_or(false, |p| handle_rect.contains(p)) {
                    clicked_idx = Some(idx);
                }
            }

            if let Some(idx) = clicked_idx {
                editor_state.selected_point_idx = Some(idx);
            }

            // Handle Drag Movement
            if let Some(idx) = dragged_idx {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let (nx, ny) = to_norm(pointer_pos);
                    editor_state.shape.points[idx].x = (nx * 100.0).round() / 100.0;
                    editor_state.shape.points[idx].y = (ny * 100.0).round() / 100.0;
                    editor_state.selected_point_idx = Some(idx);
                }
            }

            ui.separator();

            // Point Operations Toolbar
            ui.horizontal(|ui| {
                if ui.button("➕ Add Point").clicked() {
                    editor_state.shape.points.push(EditablePoint {
                        x: 0.0,
                        y: 0.0,
                        r: 255,
                        g: 255,
                        b: 255,
                        dwell: 3,
                    });
                    editor_state.selected_point_idx = Some(editor_state.shape.points.len() - 1);
                }

                if ui.button("🗑️ Delete Point").clicked() {
                    if let Some(idx) = editor_state.selected_point_idx {
                        if idx < editor_state.shape.points.len() {
                            editor_state.shape.points.remove(idx);
                            editor_state.selected_point_idx = None;
                        }
                    }
                }

                if ui.button("🔄 Close Loop").clicked() {
                    if let Some(first) = editor_state.shape.points.first().cloned() {
                        editor_state.shape.points.push(first);
                    }
                }
            });

            ui.separator();

            // Selected Point Details Table
            if let Some(idx) = editor_state.selected_point_idx {
                if idx < editor_state.shape.points.len() {
                    ui.label(format!("Editing Point #{}", idx + 1));
                    let point = &mut editor_state.shape.points[idx];

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        ui.add(egui::DragValue::new(&mut point.x).speed(0.01).range(-1.0..=1.0));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut point.y).speed(0.01).range(-1.0..=1.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut rgb = [point.r as f32 / 255.0, point.g as f32 / 255.0, point.b as f32 / 255.0];
                        if ui.color_edit_button_rgb(&mut rgb).changed() {
                            point.r = (rgb[0] * 255.0) as u8;
                            point.g = (rgb[1] * 255.0) as u8;
                            point.b = (rgb[2] * 255.0) as u8;
                        }

                        ui.label("Corner Dwell:");
                        ui.add(egui::DragValue::new(&mut point.dwell).range(0..=8));
                    });
                }
            }
        });
}

/// Updates the UniversalPath on EditorPreviewShape to reflect edited points in real-time
fn update_editor_preview_shape(
    editor_state: Res<ShapeEditorState>,
    mut query: Query<(&mut UniversalPath, &mut PathRenderable), With<EditorPreviewShape>>,
) {
    if !editor_state.live_project {
        for (_, mut renderable) in query.iter_mut() {
            renderable.visible = false;
        }
        return;
    }

    for (mut path, mut renderable) in query.iter_mut() {
        renderable.visible = true;

        let points = editor_state
            .shape
            .points
            .iter()
            .map(|p| PathPoint::new(p.x, p.y, p.r, p.g, p.b, p.dwell))
            .collect();

        let segment = PathSegment::new(points);
        *path = UniversalPath::from_segment(segment);
    }
}
