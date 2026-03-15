use bevy::{app::{App, Plugin, Startup, Update}, ecs::{message::MessageReader, system::{Res, ResMut}}, state::{app::AppExtStates, state::{NextState, OnEnter, SubStates}}, time::Time};
use common::{game::{Game, GameRegistry, GameSessionCreated, GameSessionUpdate}, scene::SceneSetup, state::ServerState};
use serde::{Deserialize, Serialize};
use bevy::prelude::StateSet;
use bevy::math::{Mat4, Vec3};
use crate::model::{HunterGameStats, GameReport, TargetEvent};

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
        app.add_systems(Update, (set_hunter_game_state_on, set_hunter_game_state_on_update, init_hunter_stats));
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

fn set_hunter_game_state_on_update(
    mut state: ResMut<NextState<HunterGameState>>,
    mut events: MessageReader<GameSessionUpdate>,
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
    mut created_events: MessageReader<GameSessionCreated>,
    mut update_events: MessageReader<GameSessionUpdate>,
    existing_stats: Option<Res<HunterGameStats>>,
    time: bevy::ecs::system::Res<Time>,
) {
    let mut try_init = |session: &common::game::GameSession| {
        if session.game_id != GAME_ID {
            return;
        }

        if let Some(stats) = existing_stats.as_ref() {
            if stats.session_id == session.session_id {
                return;
            }
        }

        commands.insert_resource(HunterGameStats {
            session_id: session.session_id,
            targets_spawned: 0,
            targets_popped: 0,
            misses: 0,
            score: 0,
            target_events: Vec::new(),
            game_start_time: time.elapsed_secs_f64(),
        });
        bevy::log::info!("Initialized Hunter game stats for session {}", session.session_id);
    };

    for event in created_events.read() {
        try_init(&event.game_session);
    }

    for event in update_events.read() {
        try_init(&event.game_session);
    }
}

/// Convert world-space position to scene-local coordinates
fn world_to_scene_local(world_pos: &Vec3, scene_setup: &SceneSetup) -> Vec3 {
    let origin = &scene_setup.scene.origin;
    let scene_matrix = Mat4::from_scale_rotation_translation(
        origin.scale,
        origin.rotation,
        origin.translation,
    );
    scene_matrix.inverse().transform_point3(*world_pos)
}

/// Generate post-game report from statistics
/// Positions in the report are converted to scene-local coordinates.
pub fn generate_game_report(stats: &HunterGameStats, end_time: f64, scene_setup: &SceneSetup) -> GameReport {
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
        scene_setup: scene_setup.clone(),
        total_targets_spawned: stats.targets_spawned,
        total_targets_popped: stats.targets_popped,
        total_misses: stats.misses,
        total_score: stats.score,
        total_game_time,
        avg_spawn_interval,
        avg_target_lifetime: avg_lifetime,
        spawn_positions: spawned_events.iter().map(|e| world_to_scene_local(&e.position, scene_setup)).collect(),
        pop_positions: popped_events.iter().map(|e| world_to_scene_local(&e.position, scene_setup)).collect(),
        timeline: stats.target_events.iter().map(|e| TargetEvent {
            position: world_to_scene_local(&e.position, scene_setup),
            ..e.clone()
        }).collect(),
    }
}