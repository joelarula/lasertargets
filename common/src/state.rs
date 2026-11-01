use bevy::state::state::{States, SubStates};
use serde::{Deserialize, Serialize};
use bevy::prelude::{Resource, StateSet};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ServerState {
    #[default]
    Menu,
    Session,
    Receiver,
}

#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ServerState = ServerState::Session)]
pub enum SessionState {
    #[default]
    Active,
    Paused,
    Finished,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TerminalState {
    #[default]
    Connecting,
    Connected,
    Disconnected,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum CalibrationState {
    #[default]
    On,
    Off,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerInstanceId(pub Option<bevy::asset::uuid::Uuid>);
