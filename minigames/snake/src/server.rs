use bevy::prelude::*;
use common::pipeline::plugin::{PathPipelineSet, ReactiveAppExt, TriggerTarget};
use common::pipeline::Signals;
use common::state::{SessionState, ServerState};

use crate::events::{BroadcastSnakeStatsEvent, ChangeSnakeDirectionEvent, SnakeGameOverEvent, SnakeGemEatenEvent};
use crate::systems::input::{handle_direction_input, handle_snake_gamepad_inputs};
use crate::systems::lifecycle::{
    cleanup_snake_game, forward_snake_stats_to_network, handle_snake_game_over, init_snake_game,
    save_snake_report,
};
use crate::systems::movement::snake_move_tick;
use crate::resources::SnakeMoveTimer;
use crate::systems::SnakeSystemSet;
use crate::types::{INITIAL_TICK_INTERVAL, MIN_TICK_INTERVAL};

pub struct SnakeGameServerPlugin;

impl Plugin for SnakeGameServerPlugin {
    fn build(&self, app: &mut App) {
        // Register events
        app.add_message::<ChangeSnakeDirectionEvent>();
        app.add_message::<BroadcastSnakeStatsEvent>();
        app.add_message::<SnakeGameOverEvent>();
        app.add_message::<SnakeGemEatenEvent>();

        // Visuals: every snake entity names its pipeline (assets/pipelines/snake/); the game
        // only writes base shapes and raises events, the pipelines react
        app.embed_pipeline(include_str!("../../../assets/pipelines/snake/head.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/snake/body.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/snake/gem.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/snake/title.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/snake/game_over.pipeline.yaml"))
            .register_trigger_message::<SnakeGemEatenEvent>("snake.gem_eaten", |_| Some((TriggerTarget::Broadcast, 1.0)))
            .register_trigger_message::<SnakeGameOverEvent>("snake.game_over", |_| Some((TriggerTarget::Broadcast, 1.0)))
            .add_systems(Update, publish_snake_speed.in_set(PathPipelineSet::Signals));

        // Configure system sets and ordering
        app.configure_sets(
            Update,
            (
                SnakeSystemSet::Input,
                SnakeSystemSet::Render,
                SnakeSystemSet::Lifecycle,
            ),
        );

        app.add_systems(
            Update,
            (
                (handle_snake_gamepad_inputs, handle_direction_input).in_set(SnakeSystemSet::Input),
                (
                    init_snake_game,
                    handle_snake_game_over,
                    forward_snake_stats_to_network,
                )
                    .in_set(SnakeSystemSet::Lifecycle),
            ),
        );

        app.add_systems(
            FixedUpdate,
            snake_move_tick.in_set(SnakeSystemSet::Movement),
        );

        app.add_systems(
            OnExit(ServerState::Session),
            (save_snake_report, cleanup_snake_game).chain(),
        );

        app.add_systems(OnExit(SessionState::Active), cleanup_snake_game);
    }
}

/// Publishes `snake.speed`: 0 at the starting pace, 1 at top speed
fn publish_snake_speed(timer: Option<Res<SnakeMoveTimer>>, signals: Option<ResMut<Signals>>) {
    let (Some(timer), Some(mut signals)) = (timer, signals) else { return };
    let interval = timer.timer.duration().as_secs_f32();
    let speed = (INITIAL_TICK_INTERVAL - interval) / (INITIAL_TICK_INTERVAL - MIN_TICK_INTERVAL);
    signals.set("snake.speed", speed.clamp(0.0, 1.0));
}
