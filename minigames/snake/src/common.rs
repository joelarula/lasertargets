use bevy::prelude::*;
use common::game::{Game, GameRegistry, GameSessionCreated, GameSessionUpdate};
use common::state::ServerState;
use serde::{Deserialize, Serialize};

use crate::model::{GAME_ID, GAME_NAME};

/// Registers the game and tracks whether a snake session is running (server and terminal)
pub struct SnakeGamePlugin;

/// On while the current session is a snake session
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SnakeGameState {
    #[default]
    Off,
    On,
}

/// Where the server's snake round is (server only)
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SnakePhase {
    /// No snake game
    #[default]
    Off,
    /// Snake on the field, waiting for the first direction input
    Waiting,
    Playing,
    /// GAME OVER shows; the next round starts after [`crate::types::RESTART_DELAY_SECS`]
    GameOver,
}

impl Plugin for SnakeGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SnakeGameState>()
            .add_systems(Startup, register_snake_game)
            .add_systems(Update, follow_snake_sessions)
            .add_systems(
                OnEnter(ServerState::Menu),
                |mut state: ResMut<NextState<SnakeGameState>>| state.set(SnakeGameState::Off),
            );
    }
}

fn register_snake_game(mut registry: ResMut<GameRegistry>) {
    registry.register_game(Game {
        name: GAME_NAME.to_string(),
        id: GAME_ID,
    });
}

/// A snake session (created or updated) turns the game on; a new session of another game turns it off
fn follow_snake_sessions(
    mut state: ResMut<NextState<SnakeGameState>>,
    mut created: MessageReader<GameSessionCreated>,
    mut updated: MessageReader<GameSessionUpdate>,
) {
    for event in created.read() {
        state.set(if event.game_session.game_id == GAME_ID {
            SnakeGameState::On
        } else {
            SnakeGameState::Off
        });
    }
    if updated
        .read()
        .any(|event| event.game_session.game_id == GAME_ID)
    {
        state.set(SnakeGameState::On);
    }
}
