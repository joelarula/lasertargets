use crate::plugins::network::NetworkingPlugin;
use crate::plugins::scene::ScenePlugin;
use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_quinnet::server::QuinnetServerPlugin;

pub mod plugins;

pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(schedule_runner));
    add_common_server_plugins(&mut app);
    app
}

pub fn add_common_server_plugins(app: &mut App) {
    app.add_plugins(LogPlugin::default())
        .add_plugins(ScenePlugin)
        .add_plugins(QuinnetServerPlugin::default())
        .add_plugins(NetworkingPlugin);
}
