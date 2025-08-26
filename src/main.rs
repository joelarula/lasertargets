use bevy::prelude::*;
mod plugins;
mod util;
use crate::plugins::instructions::InstructionsPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::scene::ScenePlugin;

const FIXED_TIMESTEP: f64 = 1.0 / 50.0; 

fn main() {

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Video Targets".to_string(),
                present_mode: bevy::window::PresentMode::AutoNoVsync ,
                ..Default::default()
            }),
            ..Default::default()
    }))
    .insert_resource(ClearColor(Color::BLACK)) 
    .insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP))
    .add_plugins(InstructionsPlugin)
    .add_plugins(ConfigPlugin)
    .add_plugins(ScenePlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(CalibrationPlugin);
    app.run();
}
