use bevy::prelude::*;
use common::state::{GameState, ServerState};

use crate::events::{BroadcastSnakeStatsEvent, ChangeSnakeDirectionEvent, SnakeGameOverEvent};
use crate::systems::input::{handle_direction_input, handle_snake_gamepad_inputs};
use crate::systems::lifecycle::{
    cleanup_snake_game, forward_snake_stats_to_network, handle_snake_game_over, init_snake_game,
    save_snake_report,
};
use crate::systems::movement::snake_move_tick;
use crate::systems::render::animate_snake_title_announcement;
use crate::systems::SnakeSystemSet;

pub struct SnakeGameServerPlugin;

impl Plugin for SnakeGameServerPlugin {
    fn build(&self, app: &mut App) {
        // Register events
        app.add_message::<ChangeSnakeDirectionEvent>();
        app.add_message::<BroadcastSnakeStatsEvent>();
        app.add_message::<SnakeGameOverEvent>();

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
                animate_snake_title_announcement.in_set(SnakeSystemSet::Render),
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
            OnExit(ServerState::InGame),
            (save_snake_report, cleanup_snake_game).chain(),
        );

        app.add_systems(OnExit(GameState::InGame), cleanup_snake_game);
    }
}
