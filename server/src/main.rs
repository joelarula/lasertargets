use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy_quinnet::{
    server::{
        QuinnetServerPlugin,
    },
};
use crate::plugins::network::NetworkingPlugin;
use crate::plugins::scene::ScenePlugin;
use std::time::Duration;

mod plugins;


fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0), // 60 FPS
        )))
        .add_plugins(LogPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(QuinnetServerPlugin::default())
        .add_plugins(NetworkingPlugin)
        .run();
}


/// System to set up the server at application startup.
/// This could involve binding to a port, configuring connection listeners, etc.
fn setup_server_system() {
    info!("Server setup system executed.");
    // Placeholder for actual server initialization logic.
    // For example, QuinnetServer resources can be configured or started here.
}

/// System to handle incoming network connections and messages during regular updates.
/// This system would typically query for network events, process incoming data, and send responses.
fn handle_connections_system() {
    // Placeholder for logic to process network events, read messages, etc.
    // This system will run every frame (or according to its schedule) and can
    // interact with QuinnetServer's event readers and message handlers.
}