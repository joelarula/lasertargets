use bevy::state::state::States;
use serde::{Deserialize, Serialize};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ServerState {
    #[default]
    Menu,
    InGame,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameState {
    #[default]
    Off,
    Menu,
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