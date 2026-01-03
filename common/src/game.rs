use crate::actor::Actor;
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
    pub id: u16,
    pub name: String,
    pub uuid: Uuid,
    pub started: bool,
    pub paused: bool,
    pub start_timestamp: Option<u64>,
    pub end_timestamp: Option<u64>,
}


impl GameSession {

        pub fn start(&mut self) {
            if self.start_timestamp.is_none() {
                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
                self.start_timestamp = Some(timestamp);
            }
            self.started = true;
        }

        pub fn stop(&mut self) {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();
            self.end_timestamp = Some(timestamp);
        }

        pub fn pause(&mut self) {
            self.paused = true;
        }

        pub fn resume(&mut self) {
            self.paused = false;
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

}


pub struct GameRegistryPlugin;

impl Plugin for GameRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameRegistry {
            games: HashMap::new(),
        });
    }
}