use bevy::state::state::States;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ServerState {
    #[default]
    Starting,
    Menu,
    InGame(String),
    Paused(String),
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TerminalState {
    #[default]
    Starting,
    Connecting,
    Connected,
    Menu,
    InGame(String),
    Paused(String),
}