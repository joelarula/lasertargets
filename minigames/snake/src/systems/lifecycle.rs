use bevy::prelude::*;
use common::game::BroadcastGameDataPayload;
use common::scene::{SceneEntity, SceneSetup};
use common::state::GameState;

use crate::components::*;
use crate::events::{BroadcastSnakeStatsEvent, SnakeGameOverEvent};
use crate::resources::{SnakeMoveTimer, SnakeState};
use crate::systems::render::{
    random_color, random_gem_position, spawn_gem_entity, spawn_head_entity,
    spawn_snake_body_entity, spawn_snake_title_entity,
};
use crate::types::{CELL_SIZE, GAME_ID, INITIAL_TICK_INTERVAL, SnakeDirection};

/// Initialise the snake game when a new session with our game_id is created
pub fn init_snake_game(
    mut commands: Commands,
    mut created_events: MessageReader<common::game::GameSessionCreated>,
    mut update_events: MessageReader<common::game::GameSessionUpdate>,
    existing_state: Option<Res<SnakeState>>,
    scene_setup: Res<SceneSetup>,
    scene_query: Query<Entity, With<SceneEntity>>,
    head_query: Query<Entity, With<SnakeHead>>,
    seg_query: Query<Entity, With<SnakeSegment>>,
    gem_query: Query<Entity, With<DiamondFood>>,
    title_query: Query<Entity, With<SnakeTitleAnnouncement>>,
    border_query: Query<Entity, With<SnakeBorder>>,
    mut stats_events: MessageWriter<BroadcastSnakeStatsEvent>,
) {
    let mut should_init: Option<bevy::asset::uuid::Uuid> = None;
    let mut other_game_started = false;

    for event in created_events.read() {
        if event.game_session.game_id == GAME_ID && event.game_session.state == GameState::InGame {
            should_init = Some(event.game_session.session_id);
        } else {
            other_game_started = true;
        }
    }
    for event in update_events.read() {
        if event.game_session.game_id == GAME_ID && event.game_session.state == GameState::InGame && existing_state.is_none() {
            should_init = Some(event.game_session.session_id);
        }
    }

    if other_game_started {
        for e in head_query
            .iter()
            .chain(seg_query.iter())
            .chain(gem_query.iter())
            .chain(title_query.iter())
            .chain(border_query.iter())
        {
            if let Ok(mut entity_cmds) = commands.get_entity(e) {
                entity_cmds.despawn();
            }
        }
        if existing_state.is_some() {
            commands.remove_resource::<SnakeState>();
        }
    }

    let Some(session_id) = should_init else {
        return;
    };

    for e in head_query
        .iter()
        .chain(seg_query.iter())
        .chain(gem_query.iter())
        .chain(title_query.iter())
    {
        if let Ok(mut entity_cmds) = commands.get_entity(e) {
            entity_cmds.despawn();
        }
    }

    let dim = scene_setup.scene.scene_dimension;
    let grid_w = ((dim.x as f32 / CELL_SIZE).floor() as i32).max(4);
    let grid_h = ((dim.y as f32 / CELL_SIZE).floor() as i32).max(4);
    let start = IVec2::new(grid_w / 2, grid_h / 2);

    let mut state = SnakeState {
        segments: vec![
            start,
            start + IVec2::new(-1, 0),
            start + IVec2::new(-2, 0),
        ],
        segment_colors: vec![
            (1.0, 1.0, 1.0),
            (0.6, 0.6, 0.6),
            (0.6, 0.6, 0.6),
        ],
        direction: SnakeDirection::Right,
        queued_direction: None,
        gem_position: IVec2::ZERO,
        gem_color: (0.0, 0.0, 0.0),
        gems_eaten: 0,
        pending_growth: 0,
        grid_w,
        grid_h,
        session_id,
        is_started: false,
        game_over: false,
        game_over_reset_timer: None,
    };

    state.gem_position = random_gem_position(&state);
    let gc = random_color();
    state.gem_color = gc;

    let scene_entity = scene_query.single().ok();
    spawn_head_entity(&mut commands, &state, scene_entity);
    spawn_snake_body_entity(&mut commands, &state, scene_entity);
    spawn_gem_entity(&mut commands, &state, scene_entity);
    spawn_snake_title_entity(&mut commands, &scene_setup, scene_entity);

    commands.insert_resource(state.clone());
    commands.insert_resource(SnakeMoveTimer::new(INITIAL_TICK_INTERVAL));

    stats_events.write(BroadcastSnakeStatsEvent {
        session_id,
        score: 0,
        length: 3,
        game_over: false,
    });

    info!("Snake game initialized: {}x{} grid, session {}", grid_w, grid_h, session_id);
}

pub fn handle_snake_game_over(
    mut game_over_events: MessageReader<SnakeGameOverEvent>,
) {
    for event in game_over_events.read() {
        info!("Snake game over event received. Final score: {}", event.final_score);
    }
}

pub fn cleanup_snake_game(
    mut commands: Commands,
    head_query: Query<Entity, With<SnakeHead>>,
    seg_query: Query<Entity, With<SnakeSegment>>,
    gem_query: Query<Entity, With<DiamondFood>>,
    title_query: Query<Entity, With<SnakeTitleAnnouncement>>,
    state: Option<Res<SnakeState>>,
    timer: Option<Res<SnakeMoveTimer>>,
) {
    for e in head_query.iter().chain(seg_query.iter()).chain(gem_query.iter()).chain(title_query.iter()) {
        if let Ok(mut entity_cmds) = commands.get_entity(e) {
            entity_cmds.despawn();
        }
    }
    if state.is_some() {
        commands.remove_resource::<SnakeState>();
    }
    if timer.is_some() {
        commands.remove_resource::<SnakeMoveTimer>();
    }
    info!("Snake game cleaned up");
}

pub fn save_snake_report(
    state: Option<Res<SnakeState>>,
) {
    let Some(state) = state else { return; };

    let session_id = state.session_id;
    let stats_dir = format!("stats/snake/{}", session_id);

    if let Err(e) = std::fs::create_dir_all(&stats_dir) {
        warn!("Failed to create snake stats directory {}: {}", stats_dir, e);
        return;
    }

    let mut text = String::new();
    text.push_str("# Snake Game Report\n\n");
    text.push_str(&format!("- **Session ID**: {}\n", session_id));
    text.push_str(&format!("- **Final Score (Gems Eaten)**: {}\n", state.gems_eaten));
    text.push_str(&format!("- **Final Length**: {}\n", state.segments.len()));
    text.push_str(&format!("- **Grid Dimensions**: {}x{}\n", state.grid_w, state.grid_h));
    text.push_str(&format!("- **Game Over State**: {}\n", state.game_over));
    text.push_str("\n---");

    let md_path = format!("{}/report.md", stats_dir);
    match std::fs::write(&md_path, &text) {
        Ok(_) => info!("Snake game report saved to {}", md_path),
        Err(e) => warn!("Failed to save snake markdown report {}: {}", md_path, e),
    }

    let json_path = format!("{}/report.json", stats_dir);
    match serde_json::to_string_pretty(&*state) {
        Ok(json) => match std::fs::write(&json_path, &json) {
            Ok(_) => info!("Snake game report (JSON) saved to {}", json_path),
            Err(e) => warn!("Failed to save snake JSON report {}: {}", json_path, e),
        },
        Err(e) => warn!("Failed to serialize snake state to JSON: {}", e),
    }
}

pub fn forward_snake_stats_to_network(
    mut events: MessageReader<BroadcastSnakeStatsEvent>,
    mut payload_writer: MessageWriter<BroadcastGameDataPayload>,
) {
    for event in events.read() {
        if let Ok(json) = serde_json::to_string(event) {
            payload_writer.write(BroadcastGameDataPayload {
                game_id: GAME_ID,
                session_id: event.session_id,
                event_tag: "snake_stats".to_string(),
                payload_json: json,
            });
        }
    }
}
