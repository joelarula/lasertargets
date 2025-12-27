use crate::plugins::network::NetworkingPlugin;
use crate::plugins::scene::ScenePlugin;
use crate::plugins::actor::ActorPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::camera::CameraPlugin;
use bevy::app::ScheduleRunnerPlugin;
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
  
    app.add_plugins(ScenePlugin)
    .add_plugins(ProjectorPlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(ActorPlugin)
    .add_plugins(QuinnetServerPlugin::default())
    .add_plugins(NetworkingPlugin);
}
