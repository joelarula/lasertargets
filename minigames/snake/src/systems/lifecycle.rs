use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::curator_core::CuratorEventEnvelope;
use common::game::{GameSessionCreated, GameSessionUpdate};
use common::scene::SceneSetup;
use common::state::SessionState;

use crate::common::SnakePhase;
use crate::components::SnakeEntity;
use crate::events::{BroadcastSnakeStatsEvent, SnakeGameOverEvent};
use crate::resources::{SnakeMoveTimer, SnakeRestartTimer};
use crate::rules::{Grid, SnakeState};
use crate::systems::render::{SceneRoot, spawn_announcement, spawn_parts};
use crate::types::GAME_ID;

/// Starts a game when a snake session becomes active (or is updated to active with no game
/// running), and stops it when a session of another game starts
pub fn follow_sessions(
    mut commands: Commands,
    mut created: MessageReader<GameSessionCreated>,
    mut updated: MessageReader<GameSessionUpdate>,
    running: Option<Res<SnakeState>>,
    scene_setup: Res<SceneSetup>,
    scene: SceneRoot,
    existing: Query<Entity, With<SnakeEntity>>,
    mut next_phase: ResMut<NextState<SnakePhase>>,
) {
    let is_active_snake = |session: &common::session::Session| {
        session.game_id == GAME_ID && session.state == SessionState::Active
    };
    let mut start = None;
    let mut stop = false;
    for session in created.read().map(|e| &e.game_session) {
        start = is_active_snake(session).then_some(session.session_id);
        stop = start.is_none();
    }
    for session in updated.read().map(|e| &e.game_session) {
        if is_active_snake(session) && running.is_none() {
            start = Some(session.session_id);
        }
    }

    match start {
        Some(session_id) => {
            for entity in &existing {
                commands.entity(entity).despawn();
            }
            start_game(&mut commands, session_id, &scene_setup, scene.map(|s| *s));
            next_phase.set(SnakePhase::Waiting);
        }
        None if stop => next_phase.set(SnakePhase::Off),
        None => {}
    }
}

fn start_game(
    commands: &mut Commands,
    session_id: Uuid,
    scene_setup: &SceneSetup,
    scene: Option<Entity>,
) {
    let state = SnakeState::new(session_id, Grid::fitting(scene_setup.scene.scene_dimension));
    spawn_parts(commands, &state, scene);
    spawn_announcement(
        commands,
        scene_setup,
        scene,
        "SNAKE",
        Color::srgb(0.2, 1.0, 0.4),
        "snake/title",
    );
    info!(
        "Snake game initialized: {} grid, session {}",
        state.grid.size, session_id
    );
    commands.insert_resource(SnakeMoveTimer::new(state.move_interval()));
    commands.insert_resource(state);
}

/// Ends the game when the server leaves the session
pub fn stop_game(mut next_phase: ResMut<NextState<SnakePhase>>) {
    next_phase.set(SnakePhase::Off);
}

/// [`SnakePhase::Off`]: everything the game spawned or inserted goes
pub fn clear_game(mut commands: Commands, entities: Query<Entity, With<SnakeEntity>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<SnakeState>();
    commands.remove_resource::<SnakeMoveTimer>();
    commands.remove_resource::<SnakeRestartTimer>();
}

/// [`SnakePhase::GameOver`]: GAME OVER text, the event pipelines react to, and the restart countdown
pub fn announce_game_over(
    mut commands: Commands,
    state: Res<SnakeState>,
    scene_setup: Res<SceneSetup>,
    scene: SceneRoot,
    mut game_over: MessageWriter<SnakeGameOverEvent>,
) {
    info!("★ [Snake] Game over! Score: {}", state.gems_eaten);
    spawn_announcement(
        &mut commands,
        &scene_setup,
        scene.map(|s| *s),
        "GAME OVER",
        Color::srgb(1.0, 0.2, 0.2),
        "snake/game_over",
    );
    game_over.write(SnakeGameOverEvent {
        session_id: state.session_id,
        final_score: state.gems_eaten,
    });
    commands.insert_resource(SnakeRestartTimer::default());
}

/// Sends score, length and game over to terminals whenever one of them changes
pub fn broadcast_stats(
    state: Res<SnakeState>,
    phase: Res<State<SnakePhase>>,
    mut last: Local<Option<BroadcastSnakeStatsEvent>>,
    mut envelopes: MessageWriter<CuratorEventEnvelope>,
) {
    let stats = BroadcastSnakeStatsEvent {
        session_id: state.session_id,
        score: state.gems_eaten,
        length: state.length(),
        game_over: *phase.get() == SnakePhase::GameOver,
    };
    if last.as_ref() != Some(&stats) {
        envelopes.write(CuratorEventEnvelope::from_payload(
            &stats,
            Some(stats.session_id),
        ));
        *last = Some(stats);
    }
}
