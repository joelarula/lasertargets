
use bevy_egui::EguiContexts;
use bevy_egui::egui;
use common::config::{SceneConfiguration, ProjectorConfiguration};

use crate::plugins::camera::DisplayMode;
use crate::plugins::{
    calibration::CalibrationSystemSet, 
    toolbar::{ToolbarRegistry, ToolbarItem, Docking, ToolabarButton},
};
use bevy::prelude::*;
use bevy_egui::{ EguiPrimaryContextPass};
pub struct SettingsPlugin;


const BTN_NAME: &str = "settings";

#[derive(Resource, Default)]
pub struct OverlayVisible(pub bool);

#[derive(Resource)]
pub struct SectionExpandedState {
    pub scene: bool,
    pub camera: bool,
    pub projector: bool,
}

impl Default for SectionExpandedState {
    fn default() -> Self {
        Self {
            scene: true,
            camera: true,
            projector: true,
        }
    }
}


impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register_settings_button);
        app.add_systems(Update, handle_settings_button);
        app.insert_resource(OverlayVisible(false));
        app.insert_resource(SectionExpandedState::default());
        app.add_systems(
            EguiPrimaryContextPass,
            overlay_ui_system.after(CalibrationSystemSet),
        );

    }
}

fn register_settings_button(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_button(ToolbarItem {
        name: BTN_NAME.to_string(),
        label: "Settings".to_string(),
        icon: Some("\u{f04fe}".to_string()),
        is_active: false,
        docking: Docking::Left,
        button_size: 36.0,
    });
}

fn handle_settings_button(
    button_query: Query<(&Interaction, &ToolabarButton), Changed<Interaction>>,
    mut overlay_visible: ResMut<OverlayVisible>,
    mut toolbar_registry: ResMut<ToolbarRegistry>,
) {
    for (interaction, button) in &button_query {
        if button.name == BTN_NAME && *interaction == Interaction::Pressed {
            overlay_visible.0 = !overlay_visible.0;
            toolbar_registry.update_button_state(BTN_NAME, overlay_visible.0);
        }
    }
}


pub fn overlay_ui_system(
    mut egui_context: EguiContexts,
    overlay_visible: Res<OverlayVisible>,
    mut section_expanded: ResMut<SectionExpandedState>,
    mut display_mode: ResMut<DisplayMode>,
    mut scene_configuration: ResMut<SceneConfiguration>,
    mut camera_configuration: ResMut<common::config::CameraConfiguration>,
    mut projector_config: ResMut<ProjectorConfiguration>,
) {
    // Early return if overlay not visible - prevents all work
    if !overlay_visible.0 {
        return;
    }
    
    // Clone current values to detect actual changes after egui modifies them
    let orig_scene_config = scene_configuration.clone();
    let orig_camera_config = camera_configuration.clone();
    let orig_projector_config = projector_config.clone();
    
    // Scope the bypassed references so they're dropped before we call set_changed()
    {
        // Use bypass_change_detection to prevent egui from triggering changes
        let scene_config_ref = scene_configuration.bypass_change_detection();
        let camera_config_ref = camera_configuration.bypass_change_detection();
        let projector_config_ref = projector_config.bypass_change_detection();
        
        if let Ok(ctx) = egui_context.ctx_mut() {
            if overlay_visible.0 {
                let overlay_size = [600.0, 500.0];
                
                egui::Window::new("Configuration Inspector")
                    .collapsible(false)
                    .default_pos([100.0, 100.0])
                    .resizable(true)
                    .default_size(overlay_size)
                    .show(ctx, |ui| {
                        // Reset button at the top
                        ui.horizontal(|ui| {
                            if ui.button("🔄 Reset All to Defaults").clicked() {
                                *scene_config_ref = SceneConfiguration::default();
                                *camera_config_ref = common::config::CameraConfiguration::default();
                                *projector_config_ref = ProjectorConfiguration::default();
                                *display_mode = DisplayMode::Mode2D;
                            }
                        });
                        ui.separator();
                        
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            // Scene Configuration Section
                            let scene_response = egui::CollapsingHeader::new("Scene Configuration")
                                .open(Some(section_expanded.scene))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Reset").clicked() {
                                            *scene_config_ref = SceneConfiguration::default();
                                        }
                                    });
                                    property_row(ui, "Target Distance", |ui| {
                                        // The Z component of origin.translation is the target distance from camera (negative Z)
                                        let mut distance = scene_config_ref.origin.translation.z.abs();
                                        let response = ui.add(
                                            egui::DragValue::new(&mut distance)
                                                .range(0.0..=100.0)
                                                .speed(0.1)
                                                .suffix(" m"),
                                        );
                                        if response.changed() {
                                            scene_config_ref.origin.translation.z = -distance;
                                        }
                                        response
                                    });
                                    property_row(ui, "Scene Dimensions", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut scene_config_ref.scene_dimension.x)
                                                    .range(1..=4000)
                                                    .speed(1.0)
                                                    .prefix("W: "),
                                            );
                                            ui.add(
                                                egui::DragValue::new(&mut scene_config_ref.scene_dimension.y)
                                                    .range(1..=4000)
                                                    .speed(1.0)
                                                    .prefix("H: "),
                                            );
                                        });
                                    });
                                    property_row(ui, "Y-Difference from Origin", |ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut scene_config_ref.y_difference)
                                                .speed(0.1)
                                                .suffix(" m"),
                                        )
                                    });
                                    property_row(ui, "Scene Origin Y", |ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut scene_config_ref.origin.translation.y)
                                                .speed(0.1)
                                                .prefix("Y: "),
                                        )
                                    });
                                });
                            if scene_response.header_response.clicked() {
                                section_expanded.scene = !section_expanded.scene;
                            }
                            ui.add_space(10.0);

                            // Camera Configuration Section
                            let camera_response = egui::CollapsingHeader::new("Camera Configuration")
                                .open(Some(section_expanded.camera))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Reset").clicked() {
                                            *camera_config_ref = common::config::CameraConfiguration::default();
                                            *display_mode = DisplayMode::Mode2D;
                                        }
                                    });
                                    property_row(ui, "Display Mode", |ui| {
                                        let mut mode = match *display_mode {
                                            DisplayMode::Mode2D => 0,
                                            DisplayMode::Mode3D => 1,
                                        };
                                        let mut changed = false;
                                        egui::ComboBox::from_id_salt("display_mode")
                                            .selected_text(if mode == 0 { "2D" } else { "3D" })
                                            .show_ui(ui, |ui| {
                                                changed |= ui.selectable_value(&mut mode, 0, "2D").clicked();
                                                changed |= ui.selectable_value(&mut mode, 1, "3D").clicked();
                                            });
                                        if changed {
                                            *display_mode = if mode == 0 { DisplayMode::Mode2D } else { DisplayMode::Mode3D };
                                        }
                                    });
                                    property_row(ui, "Field of View", |ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut camera_config_ref.angle)
                                                .range(10.0..=120.0)
                                                .speed(0.5)
                                                .suffix("°"),
                                        )
                                    });
                                    property_row(ui, "Camera Position", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut camera_config_ref.origin.translation.y)
                                                    .speed(0.1)
                                                    .prefix("Y: "),
                                            );
                                        });
                                    });
                                });
                            if camera_response.header_response.clicked() {
                                section_expanded.camera = !section_expanded.camera;
                            }
                            ui.add_space(10.0);

                            // Projector Configuration Section
                            let projector_response = egui::CollapsingHeader::new("Projector Configuration")
                                .open(Some(section_expanded.projector))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Reset").clicked() {
                                            *projector_config_ref = ProjectorConfiguration::default();
                                        }
                                    });
                                    property_row(ui, "Enabled", |ui| {
                                        ui.checkbox(&mut projector_config_ref.enabled, "")
                                    });
                                    property_row(ui, "Resolution", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("W: {}", projector_config_ref.resolution.x));
                                            ui.label(format!("H: {}", projector_config_ref.resolution.y));
                                        });
                                    });

                                    property_row(ui, "Projector Position", |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(&mut projector_config_ref.origin.translation.y)
                                                    .speed(0.1)
                                                    .prefix("Y: "),
                                            );
                                        });
                                    });
                                });
                            if projector_response.header_response.clicked() {
                                section_expanded.projector = !section_expanded.projector;
                            }
                        });
                    });
            }
        }
    } // End of scope - bypassed references are dropped here
    
    // Only trigger change detection if values actually changed
    // Compare current values (after egui modifications) with original clones
    if *scene_configuration != orig_scene_config {
        debug!("Settings UI: Scene config changed, triggering change detection");
        scene_configuration.set_changed();
    }
    if *camera_configuration != orig_camera_config {
        debug!("Settings UI: Camera config changed, triggering change detection");
        camera_configuration.set_changed();
    }
    if *projector_config != orig_projector_config {
        debug!("Settings UI: Projector config changed, triggering change detection");
        projector_config.set_changed();
    }
}

/// Helper function to create a two-column property row (label | value widget)
fn property_row<R>(
    ui: &mut egui::Ui,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.horizontal(|ui| {
        ui.add_sized([180.0, 0.0], egui::Label::new(label));
        add_contents(ui)
    }).inner
}
