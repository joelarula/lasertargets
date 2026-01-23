use bevy::{app::{App, Plugin, Startup}, ecs::system::{Commands, ResMut}, state::{app::AppExtStates, state::{OnEnter, OnExit}}};
use common::state::ServerState;

use crate::common::HunterGamePlugin;

pub struct HunterTerminalPlugin;

impl Plugin for HunterTerminalPlugin {
    fn build(&self, app: &mut App) {      
        app.add_systems(OnEnter(ServerState::Menu), spawn_menu_toolbar);
        app.add_systems(OnExit(ServerState::Menu), despawn_menu_toolbar);

    }
}


fn spawn_menu_toolbar(mut commands: Commands) {
    
}


fn despawn_menu_toolbar(mut commands: Commands) {
    
}