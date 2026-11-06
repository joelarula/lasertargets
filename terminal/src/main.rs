//! The operator terminal: shows the scene and what the laser draws, mirrors the server's state,
//! and sends the server input and edits.

mod net;
mod ui;
mod view;

use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_egui::EguiPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use terminal_ui::ToolbarPlugin;

fn main() {
    setup_logging();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LaserTargets Terminal".to_string(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((EguiPlugin::default(), ToolbarPlugin, SceneSetupPlugin, GameRegistryPlugin))
        // The connection plugin creates the server-driven states, so it comes before the games
        .add_plugins((net::NetPlugin, view::ViewPlugin, ui::UiPlugin))
        .add_plugins((
            hunter::common::HunterGamePlugin,
            hunter::terminal::HunterTerminalPlugin,
            snake::common::SnakeGamePlugin,
            snake::terminal::SnakeTerminalPlugin,
        ))
        .run();
}

/// Colourised `log` output without timestamps or module paths
fn setup_logging() {
    use pretty_env_logger::env_logger::{Builder, Env, WriteStyle};
    Builder::from_env(Env::default().filter_or("RUST_LOG", "info"))
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .write_style(WriteStyle::Always)
        .init();
}
