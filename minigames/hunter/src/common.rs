use bevy::{app::{App, Plugin, Startup, Update}, ecs::{message::MessageReader, system::ResMut}, state::{app::AppExtStates, state::{NextState, OnEnter, SubStates}}, time::Time};
use common::{game::{Game, GameRegistry, GameSessionCreated}, state::ServerState};
use serde::{Deserialize, Serialize};
use bevy::prelude::StateSet;
use crate::model::{HunterGameStats, GameReport};

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
        app.add_systems(Update, (set_hunter_game_state_on, init_hunter_stats));
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

/// Initialize game statistics when game session starts
fn init_hunter_stats(
    mut commands: bevy::ecs::system::Commands,
    mut events: MessageReader<GameSessionCreated>,
    time: bevy::ecs::system::Res<Time>,
) {
    for event in events.read() {
        if event.game_session.game_id == GAME_ID {
            commands.insert_resource(HunterGameStats {
                session_id: event.game_session.session_id,
                targets_spawned: 0,
                targets_popped: 0,
                score: 0,
                target_events: Vec::new(),
                game_start_time: time.elapsed_secs_f64(),
            });
            bevy::log::info!("Initialized Hunter game stats for session {}", event.game_session.session_id);
        }
    }
}

/// Generate post-game report from statistics
pub fn generate_game_report(stats: &HunterGameStats, end_time: f64) -> GameReport {
    let mut spawned_events = Vec::new();
    let mut popped_events = Vec::new();
    
    // Separate events by type
    for event in &stats.target_events {
        match event.event_type.as_str() {
            "spawned" => spawned_events.push(event),
            "popped" => popped_events.push(event),
            _ => {}
        }
    }
    
    let total_game_time = end_time - stats.game_start_time;
    
    // Calculate average spawn interval
    let avg_spawn_interval = if spawned_events.len() > 1 {
        total_game_time / (spawned_events.len() - 1) as f64
    } else {
        0.0
    };
    
    // Calculate average target lifetime
    let mut lifetimes = Vec::new();
    for popped in &popped_events {
        if let Some(spawned) = spawned_events.iter()
            .find(|s| s.target_uuid == popped.target_uuid) {
            lifetimes.push(popped.timestamp - spawned.timestamp);
        }
    }
    let avg_lifetime = if !lifetimes.is_empty() {
        lifetimes.iter().sum::<f64>() / lifetimes.len() as f64
    } else {
        0.0
    };
    
    GameReport {
        total_targets_spawned: stats.targets_spawned,
        total_targets_popped: stats.targets_popped,
        total_score: stats.score,
        total_game_time,
        avg_spawn_interval,
        avg_target_lifetime: avg_lifetime,
        spawn_positions: spawned_events.iter().map(|e| e.position).collect(),
        pop_positions: popped_events.iter().map(|e| e.position).collect(),
        timeline: stats.target_events.clone(),
    }
}