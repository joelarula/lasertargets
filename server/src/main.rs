use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use server::create_server_app;
use std::time::Duration;

/// Helper to create the server application with production settings (60 FPS loop).
pub fn create_production_server_app() -> App {
    let mut app = create_server_app(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f64(1.0 / 60.0), // 60 FPS
    ));

    // Add production-only plugins and systems
    app.add_plugins(LogPlugin::default())
        .add_systems(Startup, setup_server_system)
        .add_systems(Update, handle_connections_system);

    app
}

fn main() {
    create_production_server_app().run();
}

/// System to set up the server at application startup.
fn setup_server_system() {
    info!("Server setup system executed.");
}

/// System to handle incoming network connections and messages during regular updates.
fn handle_connections_system() {
    // Logic for production connections
}
