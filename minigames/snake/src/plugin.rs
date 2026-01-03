use bevy::{app::{App, Plugin, Startup}, ecs::system::ResMut};
use common::game::{Game, GameRegistry};

const GAME_ID: u16 = 2;
const GAME_NAME: &str = "snake"; 

pub struct SnakeGamePlugin;

impl Plugin for SnakeGamePlugin {
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