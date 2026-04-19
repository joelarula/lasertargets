use crate::plugins::network::NetworkingPlugin;
use crate::plugins::actor::ActorPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::game::GamePlugin;
use crate::plugins::gamepad::GamepadInputPlugin;
use crate::plugins::scene::ScenePlugin;
use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::path::PathNetworkPlugin;
use bevy::app::ScheduleRunnerPlugin;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_quinnet::server::QuinnetServerPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use common::state::{CalibrationState, GameState, ServerState};
use hunter::common::HunterGamePlugin;
use hunter::server::HunterGameServerPlugin;
use snake::common::SnakeGamePlugin;
use snake::server::SnakeGameServerPlugin;

pub mod plugins;
pub mod dac;

const FIXED_TIMESTEP: f64 = 1.0 / 50.0; 

pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(schedule_runner));
    app.add_plugins(InputPlugin);
    // On Linux (Raspberry Pi), gilrs uses evdev which works headlessly.
    // On Windows, gilrs uses WGI which requires a focused window — we use XInput directly instead.
    #[cfg(not(target_os = "windows"))]
    app.add_plugins(bevy::gilrs::GilrsPlugin);
    add_common_server_plugins(&mut app);
    app
}

pub fn add_common_server_plugins(app: &mut App) {
  
    app
    .insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP))
    .insert_resource(ServerInstanceId(Some(bevy::asset::uuid::Uuid::new_v4())))
    .add_plugins(StatesPlugin)
    .init_state::<ServerState>()
    .init_state::<CalibrationState>()
    .init_state::<GameState>()
    .add_plugins(SceneSetupPlugin)
    .add_plugins(ScenePlugin)
    .add_plugins(CalibrationPlugin)
    .add_plugins(ProjectorPlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(ActorPlugin)
    .add_plugins(GameRegistryPlugin)
    .add_plugins(QuinnetServerPlugin::default())
    .add_plugins(NetworkingPlugin)
    .add_plugins(PathNetworkPlugin)
    .add_plugins(HunterGamePlugin)
    .add_plugins(HunterGameServerPlugin)
    .add_plugins(SnakeGamePlugin)
    .add_plugins(SnakeGameServerPlugin)
    .add_plugins(GamePlugin)
    .add_plugins(GamepadInputPlugin)
    // Log state transitions for states without existing handlers
    .add_systems(OnExit(ServerState::Menu), || info!("Exiting ServerState::Menu"))
    .add_systems(OnEnter(ServerState::InGame), || info!("Entering ServerState::InGame"))
    .add_systems(OnEnter(GameState::InGame), || info!("Entering GameState::InGame"))
    .add_systems(OnExit(GameState::InGame), || info!("Exiting GameState::InGame"))
    .add_systems(OnEnter(GameState::Paused), || info!("Entering GameState::Paused"))
    .add_systems(OnExit(GameState::Paused), || info!("Exiting GameState::Paused"))
    .add_systems(OnEnter(GameState::Finished), || info!("Entering GameState::Finished"))
    .add_systems(OnExit(GameState::Finished), || info!("Exiting GameState::Finished"));
}
