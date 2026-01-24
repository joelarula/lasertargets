use crate::{actor::Actor, state::GameState};
use serde::{Deserialize, Serialize};
use bevy::{asset::uuid::Uuid, platform::collections::HashMap, prelude::*};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: u16,
    pub name: String,
}

#[derive(Component,Debug, Clone, Serialize, Deserialize)]
pub struct GameSession {
    pub game_id: u16,      // The type of game (from registry)
    pub session_id: Uuid,  // Unique session identifier
    pub name: String,
    pub state: crate::state::GameState,
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
}

#[derive(Message, Debug, Clone)]
pub struct GameSessionCreated {
    pub game_session: GameSession,
}

#[derive(Message, Debug, Clone)]
pub struct GameSessionUpdate {
    pub game_session: GameSession,
}

/// Event to signal exit game action
#[derive(Message, Debug, Clone)]
pub struct ExitGameEvent{
    pub game_session_uuid: Uuid,
}

impl GameSession {
    pub fn start(&mut self) {
        if self.start_timestamp.is_none() {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
            self.start_timestamp = Some(timestamp);
        }
        self.state = crate::state::GameState::InGame;
    }

    pub fn stop(&mut self) {
        if(self.start_timestamp.is_some()) {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
            self.end_timestamp = Some(timestamp);
            self.state = crate::state::GameState::Finished;
        }
    }

    pub fn pause(&mut self) {
        if(self.start_timestamp.is_some() && self.end_timestamp.is_none()) {
            self.state = crate::state::GameState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if(self.start_timestamp.is_some() && self.end_timestamp.is_none()) {
            self.state = crate::state::GameState::InGame;
        }
    }
}





#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameRegistry {
    games: HashMap<u16, Game>,
}

impl GameRegistry {
    pub fn register_game(&mut self, game: Game) {
        self.games.insert(game.id, game);
    }

    pub fn get_game_by_id(&self, id: u16) -> Option<&Game> {
        self.games.get(&id)
    }
}


pub struct GameRegistryPlugin;

impl Plugin for GameRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
        app.add_message::<ExitGameEvent>();
        app.add_message::<GameSessionUpdate>();
        app.add_message::<GameSessionCreated>();
        app.insert_resource(GameRegistry {
            games: HashMap::new(),
        });
    }
}