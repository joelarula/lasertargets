use bevy::prelude::*;
use common::game::{Game, GameRegistry, GameSessionCreated, GameSessionUpdate};
use common::state::ServerState;
use serde::{Deserialize, Serialize};

pub const GAME_ID: u16 = 101;
pub const GAME_NAME: &str = "huntergame";

/// Registers the game and tracks whether a hunter session is running (server and terminal)
pub struct HunterGamePlugin;

/// On while the current session is a hunter session
#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ServerState = ServerState::Session)]
pub enum HunterGameState {
    #[default]
    Off,
    On,
}

impl Plugin for HunterGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<HunterGameState>()
            .add_systems(Startup, |mut registry: ResMut<GameRegistry>| {
                registry.register_game(Game { name: GAME_NAME.to_string(), id: GAME_ID });
            })
            .add_systems(Update, follow_hunter_sessions)
            .add_systems(OnEnter(ServerState::Menu), |mut state: ResMut<NextState<HunterGameState>>| {
                state.set(HunterGameState::Off)
            });
    }
}

/// A hunter session (created or updated) turns the game on; a new session of another game turns it off
fn follow_hunter_sessions(
    mut state: ResMut<NextState<HunterGameState>>,
    mut created: MessageReader<GameSessionCreated>,
    mut updated: MessageReader<GameSessionUpdate>,
) {
    for event in created.read() {
        state.set(if event.game_session.game_id == GAME_ID {
            HunterGameState::On
        } else {
            HunterGameState::Off
        });
    }
    if updated.read().any(|event| event.game_session.game_id == GAME_ID) {
        state.set(HunterGameState::On);
    }
}
