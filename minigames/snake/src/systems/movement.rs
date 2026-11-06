use std::time::Duration;

use bevy::prelude::*;

use crate::common::SnakePhase;
use crate::events::{SnakeGemEatenEvent, SnakeRestartEvent};
use crate::resources::{SnakeMoveTimer, SnakeRestartTimer};
use crate::rules::{SnakeState, StepOutcome};

/// Moves the snake whenever the move timer runs out (while playing)
pub fn move_snake(
    time: Res<Time>,
    mut timer: ResMut<SnakeMoveTimer>,
    mut state: ResMut<SnakeState>,
    mut next_phase: ResMut<NextState<SnakePhase>>,
    mut gem_events: MessageWriter<SnakeGemEatenEvent>,
) {
    if !timer.tick(time.delta()).just_finished() {
        return;
    }
    match state.step() {
        StepOutcome::Moved => {}
        StepOutcome::AteGem => {
            timer.set_duration(Duration::from_secs_f32(state.move_interval()));
            gem_events.write(SnakeGemEatenEvent {
                session_id: state.session_id,
                gems_eaten: state.gems_eaten,
            });
            info!(
                "Snake ate gem! Score: {}, length: {} (+{}), interval: {:.3}s",
                state.gems_eaten,
                state.length(),
                state.pending_growth,
                state.move_interval()
            );
        }
        StepOutcome::Died => next_phase.set(SnakePhase::GameOver),
    }
}

/// After GAME OVER has shown, starts the next round right away (moving, no wait for input)
pub fn restart_after_game_over(
    time: Res<Time>,
    mut restart: ResMut<SnakeRestartTimer>,
    mut timer: ResMut<SnakeMoveTimer>,
    mut state: ResMut<SnakeState>,
    mut next_phase: ResMut<NextState<SnakePhase>>,
    mut restarted: MessageWriter<SnakeRestartEvent>,
) {
    if !restart.tick(time.delta()).just_finished() {
        return;
    }
    info!("★ [Snake] Starting the next round after GAME OVER");
    state.restart();
    *timer = SnakeMoveTimer::new(state.move_interval());
    restarted.write(SnakeRestartEvent {
        session_id: state.session_id,
    });
    next_phase.set(SnakePhase::Playing);
}
