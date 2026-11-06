//! Keeps the terminal's copy of server state current: the scene, camera and projector configs,
//! the server and calibration states. Configs edited locally (settings panel, calibration keys)
//! go back to the server; a value that just came from the server is not sent back.

use bevy::prelude::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::{NetworkMessage, ToServer};
use common::scene::SceneSetup;
use common::state::{CalibrationState, ServerInstanceId, ServerState, TerminalState};

use super::ServerMessage;

/// Server state the terminal keeps a copy of
trait Mirrored: Resource + Clone + PartialEq {
    /// The value `message` carries, if it carries one
    fn from_server(message: &NetworkMessage) -> Option<Self>;
}

/// Mirrored state the terminal may also edit
trait Edited: Mirrored {
    fn to_server(self) -> NetworkMessage;
}

impl Mirrored for SceneSetup {
    fn from_server(message: &NetworkMessage) -> Option<Self> {
        match message {
            NetworkMessage::SceneSetupUpdate(setup) => Some(setup.clone()),
            _ => None,
        }
    }
}

impl Mirrored for SceneConfiguration {
    fn from_server(message: &NetworkMessage) -> Option<Self> {
        match message {
            NetworkMessage::SceneConfigUpdate(config) => Some(config.clone()),
            NetworkMessage::SceneSetupUpdate(setup) => Some(setup.scene.clone()),
            _ => None,
        }
    }
}

impl Edited for SceneConfiguration {
    fn to_server(self) -> NetworkMessage {
        NetworkMessage::UpdateSceneConfig(self)
    }
}

impl Mirrored for CameraConfiguration {
    fn from_server(message: &NetworkMessage) -> Option<Self> {
        match message {
            NetworkMessage::CameraConfigUpdate(config) => Some(config.clone()),
            NetworkMessage::SceneSetupUpdate(setup) => Some(setup.camera.clone()),
            _ => None,
        }
    }
}

impl Edited for CameraConfiguration {
    fn to_server(self) -> NetworkMessage {
        NetworkMessage::UpdateCameraConfig(self)
    }
}

impl Mirrored for ProjectorConfiguration {
    fn from_server(message: &NetworkMessage) -> Option<Self> {
        match message {
            NetworkMessage::ProjectorConfigUpdate(config) => Some(config.clone()),
            NetworkMessage::SceneSetupUpdate(setup) => Some(setup.projector.clone()),
            _ => None,
        }
    }
}

impl Edited for ProjectorConfiguration {
    fn to_server(self) -> NetworkMessage {
        NetworkMessage::UpdateProjectorConfig(self)
    }
}

/// The value last exchanged with the server
#[derive(Resource)]
struct ServerCopy<T>(Option<T>);

impl<T> Default for ServerCopy<T> {
    fn default() -> Self {
        Self(None)
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct Mirror;

trait SyncAppExt {
    fn mirror<T: Mirrored>(&mut self) -> &mut Self;
    fn mirror_and_publish<T: Edited>(&mut self) -> &mut Self;
}

impl SyncAppExt for App {
    fn mirror<T: Mirrored>(&mut self) -> &mut Self {
        self.init_resource::<ServerCopy<T>>().add_systems(Update, mirror::<T>.in_set(Mirror))
    }

    fn mirror_and_publish<T: Edited>(&mut self) -> &mut Self {
        self.mirror::<T>().add_systems(
            Update,
            publish::<T>
                .after(Mirror)
                .run_if(resource_changed::<T>.and(in_state(TerminalState::Connected))),
        )
    }
}

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        // Scene config and setup come from SceneSetupPlugin; nothing else creates these two
        app.init_resource::<CameraConfiguration>()
            .init_resource::<ProjectorConfiguration>()
            .init_resource::<ServerInstanceId>()
            .mirror::<SceneSetup>()
            .mirror_and_publish::<SceneConfiguration>()
            .mirror_and_publish::<CameraConfiguration>()
            .mirror_and_publish::<ProjectorConfiguration>()
            .add_systems(OnEnter(TerminalState::Connected), query_server)
            .add_systems(Update, mirror_states);
    }
}

/// Asks for everything the terminal mirrors; the answers arrive as ordinary updates
fn query_server(mut to_server: MessageWriter<ToServer>) {
    to_server.write_batch(
        [
            NetworkMessage::QueryServerState,
            NetworkMessage::QueryCalibrationState,
            NetworkMessage::QueryProjectorConfig,
            NetworkMessage::QueryCameraConfig,
            NetworkMessage::QuerySceneConfig,
            NetworkMessage::QuerySceneSetup,
        ]
        .map(ToServer),
    );
}

fn mirror<T: Mirrored>(mut messages: MessageReader<ServerMessage>, mut local: ResMut<T>, mut copy: ResMut<ServerCopy<T>>) {
    for ServerMessage(message) in messages.read() {
        if let Some(value) = T::from_server(message) {
            local.set_if_neq(value.clone());
            copy.0 = Some(value);
        }
    }
}

fn publish<T: Edited>(local: Res<T>, mut copy: ResMut<ServerCopy<T>>, mut to_server: MessageWriter<ToServer>) {
    if copy.0.as_ref() != Some(&*local) {
        copy.0 = Some(local.clone());
        to_server.write(ToServer(local.clone().to_server()));
    }
}

fn mirror_states(
    mut messages: MessageReader<ServerMessage>,
    mut server_state: ResMut<NextState<ServerState>>,
    mut calibration_state: ResMut<NextState<CalibrationState>>,
    mut instance: ResMut<ServerInstanceId>,
) {
    for ServerMessage(message) in messages.read() {
        match message {
            NetworkMessage::ServerStateUpdate(state) => server_state.set(state.clone()),
            NetworkMessage::CalibrationStateUpdate(state) => calibration_state.set(state.clone()),
            // A restart always drops the connection, so everything is queried again on reconnect
            NetworkMessage::ServerInfo { instance_id } => {
                if instance.0.is_some_and(|known| known != *instance_id) {
                    info!("Server restarted (new instance {instance_id})");
                }
                instance.0 = Some(*instance_id);
            }
            _ => {}
        }
    }
}
