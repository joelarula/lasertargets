use std::env;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use common::state::TerminalState;
use log::info;

mod plugins;
mod util;
use crate::plugins::instructions::InstructionsPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::scene::{ScenePlugin};
use crate::plugins::toolbar::ToolbarPlugin;
use crate::plugins::settings::SettingsPlugin;
use crate::plugins::target::TargetPlugin;
use crate::plugins::basictarget::BasicTargetPlugin;
use hunter::plugin::HunterGamePlugin;
use snake::plugin::SnakeGamePlugin;


fn main() {

    util::setup_logging();
    unsafe {
        env::set_var("RUST_LOG", "debug");
    }

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LaserTargets Server".to_string(),
                present_mode: bevy::window::PresentMode::AutoNoVsync ,
                ..Default::default()
            }),
            ..Default::default()
        })
    )
    .init_state::<TerminalState>()
    .add_plugins(EguiPlugin::default())
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(InstructionsPlugin)
    .add_plugins(ConfigPlugin)
    .add_plugins(SceneSetupPlugin)
    .add_plugins(ScenePlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(CalibrationPlugin)
    .add_plugins(ProjectorPlugin)
    .add_plugins(ToolbarPlugin)
    .add_plugins(SettingsPlugin)
    .add_plugins(BasicTargetPlugin)
    .add_plugins(TargetPlugin)
    .add_plugins(GameRegistryPlugin)
    .add_plugins(HunterGamePlugin)
    .add_plugins(SnakeGamePlugin);

    app.run();
}
