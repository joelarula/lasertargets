use bevy::prelude::*;
use log::info;
use crate::plugins::calibration::CalibrationSystemSet;
use crate::plugins::toolbar::ToolbarRegistry;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectorSystemSet;

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, register_projector.in_set(ProjectorSystemSet).after(CalibrationSystemSet))
            .add_systems(Update,  update_projector_system.in_set(ProjectorSystemSet).after(CalibrationSystemSet));
    }
}

fn register_projector(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_icon_button(
        "Laser Projector".to_string(),
        projector_callback,
        "\u{f1484}".to_string(), // Nerd Font projector icon
    );
}

fn projector_callback() {
    info!("Projector button pressed!");
    // Add projector-specific functionality here
}

fn update_projector_system() {
    // Main projector system logic will go here
}
