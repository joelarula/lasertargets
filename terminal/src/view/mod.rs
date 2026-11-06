//! What the terminal draws in the 3D view: the scene, the camera looking at it, the laser paths
//! the server broadcasts, and the calibration overlay.

mod calibration;
mod camera;
mod paths;
mod scene;

use bevy::prelude::*;

pub use camera::DisplayMode;

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((scene::ScenePlugin, camera::CameraPlugin, paths::PathsPlugin, calibration::CalibrationPlugin));
    }
}
