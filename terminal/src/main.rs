use std::env;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use common::state::{CalibrationState, ServerState, TerminalState};
use hunter::terminal::HunterTerminalPlugin;
// Removed unused import: use log::info;

mod plugins;
mod util;
use crate::plugins::game::GamePlugin;
use crate::plugins::instructions::InstructionsPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::scene::{ScenePlugin};
use crate::plugins::toolbar::ToolbarPlugin;
use crate::plugins::settings::SettingsPlugin;
use crate::plugins::network::NetworkPlugin;
use crate::plugins::path::PathPlugin;
use crate::plugins::keyboard::KeyboardPlugin;
use crate::plugins::mouse::MousePlugin;
use hunter::common::HunterGamePlugin;
use snake::common::SnakeGamePlugin;
use snake::terminal::SnakeTerminalPlugin;


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
    .init_state::<ServerState>()
    .init_state::<CalibrationState>()
    .init_state::<TerminalState>()
    .add_plugins(NetworkPlugin)
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
    .add_plugins(PathPlugin)
    .add_plugins(KeyboardPlugin)
    .add_plugins(MousePlugin)
    .add_plugins(GameRegistryPlugin)
    .add_plugins(GamePlugin)
    .add_plugins(HunterGamePlugin)
    .add_plugins(HunterTerminalPlugin)
    .add_plugins(SnakeGamePlugin)
    .add_plugins(SnakeTerminalPlugin);

    app.run();
}
