pub use curator_core;

use crate::plugins::actor::ActorPlugin;
use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::game::GamePlugin;
use crate::plugins::gamepad::GamepadInputPlugin;
use crate::plugins::menu::ServerMenuWorkflowPlugin;
use crate::plugins::network::NetworkingPlugin;
use crate::plugins::path::PathNetworkPlugin;
use crate::plugins::projector::ProjectorPlugin;
use crate::plugins::scene::ScenePlugin;
use crate::plugins::status::StatusPlugin;
use bevy::app::ScheduleRunnerPlugin;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_quinnet::server::QuinnetServerPlugin;
use common::game::GameRegistryPlugin;
use common::scene::SceneSetupPlugin;
use common::state::{CalibrationState, SessionState, ServerInstanceId, ServerState};
use curator_core::CuratorCorePlugin;

pub mod dac;
pub mod plugins;

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
    app.insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP))
        .insert_resource(ServerInstanceId(Some(bevy::asset::uuid::Uuid::new_v4())))
        .add_plugins(StatesPlugin)
        .init_state::<ServerState>()
        .init_state::<CalibrationState>()
        .init_state::<SessionState>()
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
        .add_plugins(GamePlugin)
        .add_plugins(CuratorCorePlugin)
        .add_plugins(ServerMenuWorkflowPlugin)
        .add_plugins(GamepadInputPlugin)
        .add_plugins(StatusPlugin)
        // Log state transitions for states without existing handlers
        .add_systems(OnExit(ServerState::Menu), || {
            info!("Exiting ServerState::Menu")
        })
        .add_systems(OnEnter(ServerState::Session), || {
            info!("Entering ServerState::Session")
        })
        .add_systems(OnEnter(SessionState::Active), || {
            info!("Entering SessionState::Active")
        })
        .add_systems(OnExit(SessionState::Active), || {
            info!("Exiting SessionState::Active")
        })
        .add_systems(OnEnter(SessionState::Paused), || {
            info!("Entering SessionState::Paused")
        })
        .add_systems(OnExit(SessionState::Paused), || {
            info!("Exiting SessionState::Paused")
        })
        .add_systems(OnEnter(SessionState::Finished), || {
            info!("Entering SessionState::Finished")
        })
        .add_systems(OnExit(SessionState::Finished), || {
            info!("Exiting SessionState::Finished")
        });
}
