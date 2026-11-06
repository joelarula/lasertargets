//! The terminal's controls: toolbar buttons, their egui panels, and the on-screen help.

mod camera_calibration;
mod editor;
mod instructions;
mod projector;
mod prompt;
mod session;
mod settings;

use bevy::prelude::*;

pub use instructions::InstructionsAppExt;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            instructions::InstructionsPlugin,
            projector::ProjectorPlugin,
            settings::SettingsPlugin,
            editor::ShapeEditorPlugin,
            camera_calibration::CameraCalibrationPlugin,
            prompt::PromptPlugin,
            session::SessionPlugin,
        ));
    }
}
