use bevy::{app::{App, Plugin, Startup, Update}, ecs::{message::MessageReader, system::ResMut}, state::{app::AppExtStates, state::{NextState, OnEnter, States, SubStates}}};
use common::{game::{Game, GameRegistry, GameSessionCreated}, state::ServerState};
use serde::{Deserialize, Serialize};
use bevy::prelude::StateSet;

pub const GAME_ID: u16 = 101;
pub const GAME_NAME: &str = "huntergame"; 

pub struct HunterGamePlugin;

#[derive(SubStates,Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(ServerState = ServerState::InGame)]
pub enum HunterGameState {
    #[default]
    Off,
    On,
}

impl Plugin for HunterGamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<HunterGameState>();
        app.add_systems(Startup, fun_name);
        app.add_systems(Update, set_hunter_game_state_on);
        app.add_systems(OnEnter(ServerState::Menu), set_hunter_game_state_off);

    }
}

fn fun_name(mut registry: ResMut<GameRegistry>) {
    
    let game = Game {
        name: GAME_NAME.to_string(),
        id: GAME_ID,
    };
    registry.register_game(game);

}


fn set_hunter_game_state_on(
    mut state: ResMut<NextState<HunterGameState>>,
    mut events: MessageReader<GameSessionCreated>,
) {
    for event in events.read() {
        if event.game_session.game_id == GAME_ID {
            state.set(HunterGameState::On);
        }
    }
}

fn set_hunter_game_state_off(mut state: ResMut<NextState<HunterGameState>>) {
        state.set(HunterGameState::Off);
}