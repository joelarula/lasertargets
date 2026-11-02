pub use curator_core;


use crate::plugins::calibration::CalibrationPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::session::SessionLifecyclePlugin;
use crate::plugins::gamepad::GamepadInputPlugin;
use crate::plugins::keyboard::KeyboardControlPlugin;
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
use common::session::SessionPlugin;
use common::scene::SceneSetupPlugin;
use common::state::{CalibrationState, SessionState, ServerInstanceId, ServerState};
use curator_core::CuratorCorePlugin;
use common::settings::{ConfigAppExt, ConfigSection};
use serde::{Deserialize, Serialize};


pub mod plugins;

/// Main loop, fixed timestep and logging settings.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSettings {
    /// Main loop rate; defaults to the DAC frame rate (30000 PPS / 1024 points = ~29.3 Hz).
    pub tick_hz: f64,
    pub fixed_hz: f64,
    /// Global log level: error, warn, info, debug or trace.
    pub log_level: String,
    /// Extra per-target `tracing` directives, e.g. "wgpu=error,server_core::plugins::projector=debug".
    pub log_filter: String,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            tick_hz: 30000.0 / 1024.0,
            fixed_hz: 50.0,
            log_level: "info".into(),
            log_filter: bevy::log::DEFAULT_FILTER.into(),
        }
    }
}

impl ConfigSection for RuntimeSettings {
    const KEY: &'static str = "runtime";
}

pub fn create_server_app(schedule_runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    add_server_plugins(&mut app, schedule_runner);
    app
}

/// Adds the headless runtime and all core server plugins.
/// Insert a [`common::settings::ConfigSource`] beforehand to configure them from YAML.
pub fn add_server_plugins(app: &mut App, schedule_runner: ScheduleRunnerPlugin) {
    app.add_plugins(MinimalPlugins.set(schedule_runner));
    app.add_plugins(InputPlugin);
    // On Linux (Raspberry Pi), gilrs uses evdev which works headlessly.
    // On Windows, gilrs uses WGI which requires a focused window — we use XInput directly instead.
    #[cfg(not(target_os = "windows"))]
    app.add_plugins(bevy::gilrs::GilrsPlugin);
    add_common_server_plugins(app);
}

pub fn add_common_server_plugins(app: &mut App) {
    app.init_config::<RuntimeSettings>();
    let fixed_hz = app.world().resource::<RuntimeSettings>().fixed_hz;
    app.insert_resource(Time::<Fixed>::from_hz(fixed_hz))
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

        .add_plugins(SessionPlugin)
        .add_plugins(QuinnetServerPlugin::default())
        .add_plugins(NetworkingPlugin)
        .add_plugins(PathNetworkPlugin)
        .add_plugins(SessionLifecyclePlugin)
        .add_plugins(CuratorCorePlugin)
        .add_plugins(ServerMenuWorkflowPlugin)
        .add_plugins(GamepadInputPlugin)
        .add_plugins(KeyboardControlPlugin)
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
