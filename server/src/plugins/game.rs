
use bevy::{asset::uuid::Uuid, platform::collections::HashMap, prelude::*};
use bevy_quinnet::shared::ClientId;
use common::{actor::Actor, game::Game};
use serde::{Deserialize, Serialize};


#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameRegistry {
    games: HashMap<Uuid, Game>,
}

impl GameRegistry {
    /// Add a game to the registry. If a game with the same UUID exists, it is replaced.
    pub fn add_game(&mut self, game: Game) {
        self.games.insert(game.uuid, game);
    }

    /// Get a reference to a game by UUID.
    pub fn get_game(&self, uuid: Uuid) -> Option<&Game> {
        self.games.get(&uuid)
    }

    /// Get a mutable reference to a game by UUID.
    pub fn get_game_mut(&mut self, uuid: Uuid) -> Option<&mut Game> {
        self.games.get_mut(&uuid)
    }
}


pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameRegistry {
            games: HashMap::new(),
        });
    }
}