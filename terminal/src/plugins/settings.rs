
use bevy_egui::EguiContexts;
use bevy_egui::egui;
use common::config::{SceneConfiguration, ProjectorConfiguration};
use std::sync::Mutex;

use crate::plugins::camera::DisplayMode;
use crate::plugins::projector::ProjectorLockToScene;
use crate::plugins::{
    calibration::CalibrationSystemSet, scene::SceneSystemSet, toolbar::ToolbarRegistry,
};
use bevy::prelude::*;
use bevy_egui::{ EguiPrimaryContextPass};
pub struct SettingsPlugin;
use crate::plugins::scene::SceneData;
use crate::plugins::scene::SceneTag;

#[derive(Resource, Default)]
pub struct OverlayVisible(pub bool);

static TOGGLE_OVERLAY: Mutex<bool> = Mutex::new(false);

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_settings_button);
        app.add_systems(Update, overlay_trigger_system);
        app.insert_resource(OverlayVisible(false));
        app.add_systems(
            EguiPrimaryContextPass,
            overlay_ui_system.after(CalibrationSystemSet),
        );

    }
}

fn register_settings_button(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_icon_button(
        "Settings".to_string(),
        settings_button_callback,
        "\u{f04fe}".to_string(),
    );
}

fn settings_button_callback() {
    if let Ok(mut show) = TOGGLE_OVERLAY.lock() {
        *show = !*show;
    }
}

pub fn overlay_ui_system(
    mut egui_context: EguiContexts,
    scene_query: Query<(&SceneData), With<SceneTag>>,
    overlay_visible: Res<OverlayVisible>,
    mut scene_configuration: ResMut<SceneConfiguration>,
    mut display_mode: ResMut<DisplayMode>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut lock_to_scene: ResMut<ProjectorLockToScene>,
) {
    for scene_data in scene_query.iter() {
        if let Ok(ctx) = egui_context.ctx_mut() {
            if overlay_visible.0 {
                let overlay_size = [600.0, 600.0];
                // Center in viewport coordinates
                let viewport_size = scene_data.get_viewport_size();
                let overlay_center_in_viewport = Vec2::new(
                    (viewport_size.x as f32 - (overlay_size[0] / scene_data.scale_factor)) * 0.5,
                    100.0,
                );
                // Convert to window coordinates
                let pos = scene_data.translate_viewport_coordinates_to_window_coordinates(
                    overlay_center_in_viewport,
                );

                egui::Window::new("Settings")
                    .collapsible(false)
                    .default_pos([pos.x, pos.y])
                    .resizable(false)
                    .fixed_size(overlay_size)
                    .show(ctx, |ui| {
                        ui.label("Target Area");
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_sized([100.0, 0.0], egui::Label::new("Target distance:"));
                            let mut value = scene_configuration.target_projection_distance;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut value)
                                        .range(0.0..=50.0)
                                        .speed(0.1),
                                )
                                .changed() {
                                    scene_configuration.target_projection_distance = value;
                                     
                            }
                        });
                        ui.label("Camera Settings");
                        ui.horizontal(|ui| {
                            ui.add_sized([100.0, 0.0], egui::Label::new("Display mode:"));
                            let mut mode = match *display_mode {
                                DisplayMode::Mode2D => 0,
                                DisplayMode::Mode3D => 1,
                            };
                            let modes = ["2D", "3D"];
                            let mut mode_changed = false;
                            egui::ComboBox::from_id_salt("display_mode_combo")
                                .selected_text(modes[mode])
                                .show_ui(ui, |ui| {
                                    mode_changed |= ui.selectable_value(&mut mode, 0, "2D").changed();
                                    mode_changed |= ui.selectable_value(&mut mode, 1, "3D").changed();
                                });
                            if mode_changed {
                                *display_mode = if mode == 0 { DisplayMode::Mode2D } else { DisplayMode::Mode3D };
                            }
                        });
                        ui.separator();
                        
                        ui.label("Projector Settings");
                        ui.horizontal(|ui| {
                            ui.add_sized([150.0, 0.0], egui::Label::new("Lock projector to scene:"));
                            let mut locked = lock_to_scene.0;
                            if ui.checkbox(&mut locked, "").changed() {
                                lock_to_scene.0 = locked;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add_sized([100.0, 0.0], egui::Label::new("Projection angle:"));
                            let mut angle = projector_config.angle;
                            ui.add_enabled_ui(!lock_to_scene.0, |ui| {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut angle)
                                            .range(10.0..=60.0)
                                            .speed(0.5)
                                            .suffix("°"),
                                    )
                                    .changed() {
                                        projector_config.angle = angle;
                                }
                            });
                        });
                        ui.separator();
                    });
            }
        }
    }
}

pub fn overlay_trigger_system(mut overlay_visible: ResMut<OverlayVisible>) {
    if let Ok(mut show) = TOGGLE_OVERLAY.lock() {
        if overlay_visible.0 != *show {
            overlay_visible.0 = *show;
        }
    }
}
