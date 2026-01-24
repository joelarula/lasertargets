use bevy::state::state::{States, SubStates};
use serde::{Deserialize, Serialize};
use bevy::prelude::StateSet;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ServerState {
    #[default]
    Menu,
    InGame,
}

#[derive(SubStates,Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ServerState = ServerState::InGame)]
pub enum GameState {
    #[default]
    InGame,
    Paused,
    Finished,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TerminalState {
    #[default]
    Connecting,
    Connected,
}