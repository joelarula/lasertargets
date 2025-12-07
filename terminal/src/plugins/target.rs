use bevy::prelude::*;
use log::info;
use std::sync::Mutex;
use crate::plugins::toolbar::ToolbarRegistry;
use crate::plugins::instructions::InstructionState;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TargetSystemSet;

#[derive(Resource, Default)]
pub struct TargetEnabled(pub bool);

static TOGGLE_TARGET_REQUESTED: Mutex<bool> = Mutex::new(false);

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(TargetEnabled(false))
            .add_systems(Startup, (register_target, register_target_instructions))
            .add_systems(Update, update_target_system.in_set(TargetSystemSet));
    }
}

fn register_target(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_icon_button(
        "Target".to_string(),
        target_callback,
        "\u{f140}".to_string(), // Target/crosshairs icon
        crate::plugins::toolbar::Docking::Left,
        36.0,
    );
}

fn register_target_instructions(mut instructions: ResMut<InstructionState>) {
    instructions.add_instruction("Press [T] to toggle target".to_string());
}

fn target_callback() {
    if let Ok(mut toggle) = TOGGLE_TARGET_REQUESTED.lock() {
        *toggle = true;
    }
}

fn update_target_system(
    mut target_enabled: ResMut<TargetEnabled>,
    mut toolbar_registry: ResMut<ToolbarRegistry>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let prev_enabled = target_enabled.0;
    
    // Check if toggle was requested from button
    if let Ok(mut toggle) = TOGGLE_TARGET_REQUESTED.lock() {
        if *toggle {
            target_enabled.0 = !target_enabled.0;
            info!("Target toggled via button: {}", target_enabled.0);
            *toggle = false;
        }
    }
    
    // Toggle target with T key
    if keyboard.just_pressed(KeyCode::KeyT) {
        target_enabled.0 = !target_enabled.0;
        info!("Target enabled: {}", target_enabled.0);
    }
    
    // Update toolbar button state if target enabled state changed
    if prev_enabled != target_enabled.0 {
        toolbar_registry.update_button_state("Target", target_enabled.0);
    }
}
