use bevy::prelude::*;
use log::info;
use std::sync::Mutex;
use crate::plugins::calibration::CalibrationSystemSet;
use crate::plugins::scene::{SceneData, SceneTag};
use crate::plugins::toolbar::ToolbarRegistry;
use crate::plugins::instructions::InstructionState;
use common::config::ProjectorConfiguration;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectorSystemSet;

#[derive(Resource, Default)]
pub struct ProjectorLockToScene(pub bool);

static TOGGLE_PROJECTOR_REQUESTED: Mutex<bool> = Mutex::new(false);

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ProjectorConfiguration::default())
            .insert_resource(ProjectorLockToScene(true))
            .add_systems(Startup, (register_projector, register_projector_instructions).in_set(ProjectorSystemSet).after(CalibrationSystemSet))
            .add_systems(Update,  update_projector_system.in_set(ProjectorSystemSet).after(CalibrationSystemSet));
    }
}

fn register_projector(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_icon_button(
        "Projector".to_string(),
        projector_callback,
        "\u{f0eb}".to_string(), // Laser icon
        crate::plugins::toolbar::Docking::Left,
        36.0,
    );
}

fn register_projector_instructions(mut instructions: ResMut<InstructionState>) {
    instructions.add_instruction("Press [F1] to toggle projector".to_string());
    instructions.add_instruction("Press [L] to toggle lock projector to scene".to_string());
}

fn projector_callback() {
    if let Ok(mut toggle) = TOGGLE_PROJECTOR_REQUESTED.lock() {
        *toggle = true;
    }
}

fn update_projector_system(
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut lock_to_scene: ResMut<ProjectorLockToScene>,
    mut toolbar_registry: ResMut<ToolbarRegistry>,
    scene_query: Query<&SceneData, With<SceneTag>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let prev_enabled = projector_config.enabled;
    
    // Check if toggle was requested from button
    if let Ok(mut toggle) = TOGGLE_PROJECTOR_REQUESTED.lock() {
        if *toggle {
            projector_config.enabled = !projector_config.enabled;
            info!("Projector toggled via button: {}", projector_config.enabled);
            *toggle = false;
        }
    }
    
    configure_projector(&mut projector_config, &mut lock_to_scene, &keyboard);
    
    // Update toolbar button state if projector enabled state changed
    if prev_enabled != projector_config.enabled {
        toolbar_registry.update_button_state("Projector", projector_config.enabled);
    }
    
    // If locked to scene, calculate and set angle from scene data
    if lock_to_scene.0 {
        if let Ok(scene_data) = scene_query.single() {
            projector_config.angle = scene_data.calculate_projector_angle_for_scene_width();
        }
    }
}

fn configure_projector(
    projector_config: &mut ResMut<ProjectorConfiguration>,
    lock_to_scene: &mut ResMut<ProjectorLockToScene>,
    keyboard: &Res<ButtonInput<KeyCode>>,
) {
    // Toggle projector enabled state with F1 key
    if keyboard.just_pressed(KeyCode::F1) {
        projector_config.enabled = !projector_config.enabled;
        info!("Projector enabled: {}", projector_config.enabled);
    }
    
    // Toggle lock to scene with a key (e.g., L key)
    if keyboard.just_pressed(KeyCode::KeyL) {
        lock_to_scene.0 = !lock_to_scene.0;
        info!("Projector lock to scene: {}", lock_to_scene.0);
    }
    
}
