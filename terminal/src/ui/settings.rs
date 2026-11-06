//! The settings panel: edits the scene, camera and projector configs. Edits go to the server
//! through the config sync.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use terminal_ui::{Docking, ToolbarAppExt, ToolbarItem, toggled_on};

use crate::view::DisplayMode;

#[derive(Resource, Default, Deref, DerefMut)]
struct SettingsOpen(bool);

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_toolbar_toggle::<SettingsOpen>(
            ToolbarItem::new("settings").icon("\u{f04fe}").text("Settings").order(2).dock(Docking::Right),
        )
        .add_systems(EguiPrimaryContextPass, settings_panel.run_if(toggled_on::<SettingsOpen>));
    }
}

fn settings_panel(
    mut contexts: EguiContexts,
    mut open: ResMut<SettingsOpen>,
    mut display_mode: ResMut<DisplayMode>,
    mut scene_res: ResMut<SceneConfiguration>,
    mut camera_res: ResMut<CameraConfiguration>,
    mut projector_res: ResMut<ProjectorConfiguration>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };
    // Edit copies, so only real changes reach change detection (and the server)
    let mut scene = scene_res.clone();
    let mut camera = camera_res.clone();
    let mut projector = projector_res.clone();
    let mut mode = *display_mode;
    let mut still_open = true;

    egui::Window::new("Configuration Inspector")
        .open(&mut still_open)
        .default_pos([100.0, 100.0])
        .default_size([600.0, 500.0])
        .show(ctx, |ui| {
            if ui.button("🔄 Reset All to Defaults").clicked() {
                (scene, camera, projector, mode) = default();
            }
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                section(ui, "Scene Configuration", &mut scene, |ui, scene| {
                    row(ui, "Target Distance", |ui| {
                        // The scene lies along -Z from the camera
                        let mut distance = scene.origin.translation.z.abs();
                        if ui.add(metres(&mut distance).range(0.0..=100.0)).changed() {
                            scene.origin.translation.z = -distance;
                        }
                    });
                    row(ui, "Scene Dimensions", |ui| {
                        ui.add(metres(&mut scene.scene_dimension.x).range(0.25..=100.0).speed(0.25).prefix("W: "));
                        ui.add(metres(&mut scene.scene_dimension.y).range(0.25..=100.0).speed(0.25).prefix("H: "));
                    });
                    row(ui, "Y-Difference from Origin", |ui| {
                        ui.add(metres(&mut scene.y_difference));
                    });
                    row(ui, "Scene Origin Y", |ui| {
                        ui.add(metres(&mut scene.origin.translation.y).prefix("Y: "));
                    });
                });
                section(ui, "Camera Configuration", &mut camera, |ui, camera| {
                    row(ui, "Display Mode", |ui| {
                        egui::ComboBox::from_id_salt("display_mode")
                            .selected_text(if mode == DisplayMode::Mode2D { "2D" } else { "3D" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut mode, DisplayMode::Mode2D, "2D");
                                ui.selectable_value(&mut mode, DisplayMode::Mode3D, "3D");
                            });
                    });
                    row(ui, "Field of View", |ui| {
                        ui.add(egui::DragValue::new(&mut camera.angle).range(10.0..=120.0).speed(0.5).suffix("°"));
                    });
                    row(ui, "Camera Position", |ui| {
                        ui.add(metres(&mut camera.origin.translation.y).prefix("Y: "));
                    });
                });
                section(ui, "Projector Configuration", &mut projector, |ui, projector| {
                    row(ui, "Enabled", |ui| {
                        ui.checkbox(&mut projector.switched_on, "");
                    });
                    row(ui, "Resolution", |ui| {
                        ui.label(format!("W: {}  H: {}", projector.resolution.x, projector.resolution.y));
                    });
                    row(ui, "Projector Position", |ui| {
                        ui.add(metres(&mut projector.origin.translation.y).prefix("Y: "));
                    });
                });
            });
        });

    scene_res.set_if_neq(scene);
    camera_res.set_if_neq(camera);
    projector_res.set_if_neq(projector);
    display_mode.set_if_neq(mode);
    if !still_open {
        **open = false;
    }
}

/// A collapsible section editing `value`, with its own reset button
fn section<T: Default>(ui: &mut egui::Ui, title: &str, value: &mut T, add_rows: impl FnOnce(&mut egui::Ui, &mut T)) {
    egui::CollapsingHeader::new(title).default_open(true).show(ui, |ui| {
        if ui.button("Reset").clicked() {
            *value = T::default();
        }
        add_rows(ui, value);
    });
    ui.add_space(10.0);
}

/// A label | widgets row
fn row(ui: &mut egui::Ui, label: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized([180.0, 0.0], egui::Label::new(label));
        add_contents(ui);
    });
}

fn metres(value: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(value).speed(0.1).suffix(" m")
}
