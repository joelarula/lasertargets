use crate::plugins::network::NetworkingPlugin;
use crate::plugins::actor::ActorPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::camera::CameraPlugin;
use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_quinnet::server::QuinnetServerPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use common::state::ServerState;
use hunter::plugin::HunterGamePlugin;
use snake::plugin::SnakeGamePlugin;

pub mod plugins;

const FIXED_TIMESTEP: f64 = 1.0 / 50.0; 

pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(schedule_runner));
    add_common_server_plugins(&mut app);
    app
}

pub fn add_common_server_plugins(app: &mut App) {
  
    app.add_plugins(SceneSetupPlugin)
    .insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP))
    .init_state::<ServerState>()
    .add_plugins(ProjectorPlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(ActorPlugin)
    .add_plugins(GameRegistryPlugin)
    .add_plugins(QuinnetServerPlugin::default())
    .add_plugins(NetworkingPlugin)
    .add_plugins(HunterGamePlugin)
    .add_plugins(SnakeGamePlugin);
}
