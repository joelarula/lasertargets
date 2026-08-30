use bevy::prelude::*;
use common::state::ServerState;

use crate::events::{BroadcastStatsUpdateEvent, HunterClickEvent};
pub use crate::components::HunterTitleAnnouncement;
pub use crate::events::SpawnHunterTargetEvent;
pub use crate::resources::HunterTargetSelection;
use crate::systems::collision::handle_hunter_clicks;
use crate::systems::input::handle_hunter_gamepad_inputs;
use crate::systems::lifecycle::{
    forward_hunter_stats_to_network, handle_incoming_hunter_payloads, hunter_session_is_running,
    reset_hunter_on_new_session, reset_hunter_session, save_hunter_report,
};
use crate::systems::movement::{check_balloon_out_of_bounds, update_balloon_positions};
use crate::systems::render::{
    animate_hunter_shot_ripples, animate_hunter_title_announcement,
    spawn_hunter_title_on_session_start,
};
use crate::systems::spawn::{spawn_hunter_targets, update_target_spawn_immunity};
use crate::systems::HunterSystemSet;

pub struct HunterGameServerPlugin;

impl Plugin for HunterGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HunterTargetSelection>();
        app.add_message::<SpawnHunterTargetEvent>();
        app.add_message::<HunterClickEvent>();
        app.add_message::<BroadcastStatsUpdateEvent>();

        // System sets configuration
        app.configure_sets(
            Update,
            (
                HunterSystemSet::Input,
                HunterSystemSet::Spawn,
                HunterSystemSet::Collision,
                HunterSystemSet::Render,
                HunterSystemSet::Lifecycle,
            )
                .run_if(in_state(ServerState::InGame))
                .run_if(hunter_session_is_running),
        );

        app.add_systems(
            Update,
            (
                handle_hunter_gamepad_inputs.in_set(HunterSystemSet::Input),
                (spawn_hunter_targets, update_target_spawn_immunity).in_set(HunterSystemSet::Spawn),
                handle_hunter_clicks.in_set(HunterSystemSet::Collision),
                (animate_hunter_shot_ripples, check_balloon_out_of_bounds).in_set(HunterSystemSet::Render),
                (forward_hunter_stats_to_network, handle_incoming_hunter_payloads).in_set(HunterSystemSet::Lifecycle),
            ),
        );

        app.add_systems(
            FixedUpdate,
            update_balloon_positions
                .in_set(HunterSystemSet::Movement)
                .run_if(in_state(ServerState::InGame))
                .run_if(hunter_session_is_running),
        );

        app.add_systems(
            OnExit(ServerState::InGame),
            (save_hunter_report, reset_hunter_session).chain(),
        );

        app.add_systems(
            Update,
            (
                reset_hunter_on_new_session,
                spawn_hunter_title_on_session_start,
                animate_hunter_title_announcement,
            ),
        );
    }
}
