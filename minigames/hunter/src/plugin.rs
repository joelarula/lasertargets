use bevy::{app::{App, Plugin, Startup}, ecs::system::ResMut};
use common::game::{Game, GameRegistry};

const GAME_ID: u16 = 1;
const GAME_NAME: &str = "hunter"; 

pub struct HunterGamePlugin;

impl Plugin for HunterGamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, |mut registry: ResMut<GameRegistry>| {
            let game = Game {
                name: GAME_NAME.to_string(),
                id: GAME_ID,
            };
            registry.register_game(game);
        });
    }
}