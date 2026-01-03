use bevy::state::state::States;
use serde::{Deserialize, Serialize};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ServerState {
    #[default]
    Menu,
    InGame(u16),
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameState {
    #[default]
    Off,
    Menu(u16),
    InGame(u16),
    Paused(u16),
    Finished(u16),
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TerminalState {
    #[default]
    Connecting,
    Connected,
    Menu,
    InGame(u16),
}