pub use server_core::*;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use hunter::common::HunterGamePlugin;
use hunter::server::HunterGameServerPlugin;
use snake::common::SnakeGamePlugin;
use snake::server::SnakeGameServerPlugin;

/// Creates a server app composed with all registered minigames (Hunter, Snake)
pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    add_server_plugins(&mut app, schedule_runner);
    app
}

/// Adds the core server plugins and all registered minigames (Hunter, Snake).
/// Insert a [`common::settings::ConfigSource`] beforehand to configure them from YAML.
pub fn add_server_plugins(app: &mut App, schedule_runner: ScheduleRunnerPlugin) {
    server_core::add_server_plugins(app, schedule_runner);
    app.add_plugins(HunterGamePlugin)
        .add_plugins(HunterGameServerPlugin)
        .add_plugins(SnakeGamePlugin)
        .add_plugins(SnakeGameServerPlugin);
}

/// Alias for [`create_server_app`].
#[inline]
pub fn create_composed_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    create_server_app(schedule_runner)
}
