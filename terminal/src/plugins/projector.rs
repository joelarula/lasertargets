use bevy::prelude::*;
// Removed unused import: use log::info;
use crate::plugins::calibration::CalibrationSystemSet;
use crate::plugins::toolbar::{ToolbarRegistry, ToolbarItem, Docking, ToolabarButton};
use crate::plugins::instructions::InstructionState;
use common::config::{ProjectorConfiguration};

const BTN_NAME: &str = "projector";

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectorSystemSet;


pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ProjectorConfiguration::default())
            .add_systems(Startup, (register_projector, register_projector_instructions).in_set(ProjectorSystemSet).after(CalibrationSystemSet))
            .add_systems(Update, (
                handle_projector_button,
                update_projector_system,
            ).chain().in_set(ProjectorSystemSet).after(CalibrationSystemSet));
    }
}

fn register_projector(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_button(ToolbarItem {
        name: BTN_NAME.to_string(),
        label: "Projector".to_string(),
        icon: Some("\u{f0eb}".to_string()), // Laser icon
        is_active: false,
        docking: Docking::Left,
        button_size: 36.0,
    });
}

fn register_projector_instructions(mut instructions: ResMut<InstructionState>) {
    instructions.add_instruction("Press [F1] to toggle projector".to_string());
    instructions.add_instruction("Press [L] to toggle lock projector to scene".to_string());
}

fn handle_projector_button(
    button_query: Query<(&Interaction, &ToolabarButton), Changed<Interaction>>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut toolbar_registry: ResMut<ToolbarRegistry>,
) {
    for (interaction, button) in &button_query {
        if button.name == BTN_NAME && *interaction == Interaction::Pressed {
            projector_config.enabled = !projector_config.enabled;
            toolbar_registry.update_button_state(BTN_NAME, projector_config.enabled);
        }
    }
}

fn update_projector_system(
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut toolbar_registry: ResMut<ToolbarRegistry>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let prev_enabled = projector_config.enabled;
    
    configure_projector(&mut projector_config, &keyboard);
    if prev_enabled != projector_config.enabled {
        toolbar_registry.update_button_state(BTN_NAME, projector_config.enabled);
    }
    
}

fn configure_projector(
    projector_config: &mut ResMut<ProjectorConfiguration>,
    keyboard: &Res<ButtonInput<KeyCode>>,
) {
    // Toggle projector enabled state with F1 key
    if keyboard.just_pressed(KeyCode::F1) {
        projector_config.enabled = !projector_config.enabled;
    }
    
    // Toggle lock to scene with a key (e.g., L key)
    if keyboard.just_pressed(KeyCode::KeyL) {
        projector_config.locked_to_scene = !projector_config.locked_to_scene;
    }
}


