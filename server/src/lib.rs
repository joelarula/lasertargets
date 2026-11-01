pub use server_core::*;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use hunter::common::HunterGamePlugin;
use hunter::server::HunterGameServerPlugin;
use snake::common::SnakeGamePlugin;
use snake::server::SnakeGameServerPlugin;

/// Creates a server app composed with all registered minigames (Hunter, Snake)
pub fn create_composed_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = server_core::create_server_app(schedule_runner);
    app.add_plugins(HunterGamePlugin)
        .add_plugins(HunterGameServerPlugin)
        .add_plugins(SnakeGamePlugin)
        .add_plugins(SnakeGameServerPlugin);
    app
}

/// Helper to create server application with minigames bundled
pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    create_composed_server_app(schedule_runner)
}
