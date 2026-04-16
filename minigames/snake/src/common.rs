use bevy::app::{App, Plugin, Startup, Update};
use bevy::ecs::message::MessageReader;
use bevy::ecs::system::ResMut;
use bevy::log::info;
use bevy::prelude::StateSet;
use bevy::state::app::AppExtStates;
use bevy::state::state::{NextState, OnEnter, SubStates};
use common::game::{Game, GameRegistry, GameSessionCreated, GameSessionUpdate};
use common::state::ServerState;
use serde::{Deserialize, Serialize};

use crate::model::{GAME_ID, GAME_NAME};

pub struct SnakeGamePlugin;

#[derive(SubStates, Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ServerState = ServerState::InGame)]
pub enum SnakeGameState {
    #[default]
    Off,
    On,
}

impl Plugin for SnakeGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SnakeGameState>();
        app.add_systems(Startup, register_snake_game);
        app.add_systems(
            Update,
            (set_snake_game_state_on, set_snake_game_state_on_update),
        );
        app.add_systems(OnEnter(ServerState::Menu), set_snake_game_state_off);
    }
}

fn register_snake_game(mut registry: ResMut<GameRegistry>) {
    let game = Game {
        name: GAME_NAME.to_string(),
        id: GAME_ID,
    };
    registry.register_game(game);
}

fn set_snake_game_state_on(
    mut state: ResMut<NextState<SnakeGameState>>,
    mut events: MessageReader<GameSessionCreated>,
) {
    for event in events.read() {
        if event.game_session.game_id == GAME_ID {
            state.set(SnakeGameState::On);
        }
    }
}

fn set_snake_game_state_on_update(
    mut state: ResMut<NextState<SnakeGameState>>,
    mut events: MessageReader<GameSessionUpdate>,
) {
    for event in events.read() {
        if event.game_session.game_id == GAME_ID {
            state.set(SnakeGameState::On);
        }
    }
}

fn set_snake_game_state_off(mut state: ResMut<NextState<SnakeGameState>>) {
    info!("Entering ServerState::Menu (snake: setting SnakeGameState::Off)");
    state.set(SnakeGameState::Off);
}
