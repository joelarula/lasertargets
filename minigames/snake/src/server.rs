use bevy::prelude::*;
use common::calibration::GameKeyPressed;
use common::pipeline::plugin::{PathPipelineSet, ReactiveAppExt, TriggerTarget};
use common::state::{CalibrationState, ServerState, SessionState};

use crate::common::SnakePhase;
use crate::events::{
    ChangeSnakeDirectionEvent, SnakeGameOverEvent, SnakeGemEatenEvent, SnakeRestartEvent,
};
use crate::rules::SnakeState;
use crate::systems::SnakeSystemSet;
use crate::systems::input::{read_game_keys, read_gamepad, steer_snake};
use crate::systems::lifecycle::{
    announce_game_over, broadcast_stats, clear_game, follow_sessions, stop_game,
};
use crate::systems::movement::{move_snake, restart_after_game_over};
use crate::systems::render::{publish_snake_speed, sync_parts};

/// Server side of Snake: the authoritative game, its base shapes and the events pipelines react to
pub struct SnakeGameServerPlugin;

impl Plugin for SnakeGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<SnakePhase>()
            .add_message::<ChangeSnakeDirectionEvent>()
            .add_message::<GameKeyPressed>()
            .add_message::<SnakeGameOverEvent>()
            .add_message::<SnakeGemEatenEvent>()
            .add_message::<SnakeRestartEvent>();

        // Visuals: every snake entity names its pipeline (assets/pipelines/snake/); the game
        // only writes base shapes and raises events, the pipelines react
        app.embed_pipeline(include_str!(
            "../../../assets/pipelines/snake/head.pipeline.yaml"
        ))
        .embed_pipeline(include_str!(
            "../../../assets/pipelines/snake/body.pipeline.yaml"
        ))
        .embed_pipeline(include_str!(
            "../../../assets/pipelines/snake/gem.pipeline.yaml"
        ))
        .embed_pipeline(include_str!(
            "../../../assets/pipelines/snake/title.pipeline.yaml"
        ))
        .embed_pipeline(include_str!(
            "../../../assets/pipelines/snake/game_over.pipeline.yaml"
        ))
        .register_trigger_message::<SnakeGemEatenEvent>("snake.gem_eaten", |_| {
            Some((TriggerTarget::Broadcast, 1.0))
        })
        .register_trigger_message::<SnakeGameOverEvent>("snake.game_over", |_| {
            Some((TriggerTarget::Broadcast, 1.0))
        })
        .register_trigger_message::<SnakeRestartEvent>("snake.restart", |_| {
            Some((TriggerTarget::Broadcast, 1.0))
        });

        use SnakeSystemSet::*;
        app.configure_sets(
            Update,
            (Lifecycle, Input, Movement, Output)
                .chain()
                .before(PathPipelineSet::Bind),
        );

        app.add_systems(
            Update,
            (
                follow_sessions.in_set(Lifecycle),
                (
                    read_game_keys,
                    // Calibration owns the gamepad while it is on
                    read_gamepad.run_if(in_state(CalibrationState::Off)),
                    steer_snake.run_if(round_running.and(accepts_steering)),
                )
                    .chain()
                    .in_set(Input),
                (
                    move_snake.run_if(in_state(SnakePhase::Playing)),
                    restart_after_game_over.run_if(in_state(SnakePhase::GameOver)),
                )
                    .run_if(round_running)
                    .in_set(Movement),
                (
                    sync_parts.run_if(resource_exists_and_changed::<SnakeState>),
                    publish_snake_speed,
                    broadcast_stats,
                )
                    .run_if(resource_exists::<SnakeState>)
                    .in_set(Output),
            ),
        );

        app.add_systems(OnEnter(SnakePhase::GameOver), announce_game_over)
            .add_systems(OnEnter(SnakePhase::Off), clear_game)
            .add_systems(OnExit(ServerState::Session), stop_game);
    }
}

/// A round exists and its session is active (not paused or finished)
fn round_running(
    state: Option<Res<SnakeState>>,
    session: Option<Res<State<SessionState>>>,
) -> bool {
    state.is_some() && session.is_some_and(|s| *s.get() == SessionState::Active)
}

/// The snake can be turned before the first move and while it moves, not during GAME OVER
fn accepts_steering(phase: Res<State<SnakePhase>>) -> bool {
    matches!(phase.get(), SnakePhase::Waiting | SnakePhase::Playing)
}
