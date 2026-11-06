use bevy::prelude::*;
use common::pipeline::plugin::ReactiveAppExt;
use common::state::{ServerState, SessionState};

use crate::audit::HunterGameStats;
use crate::events::{HunterClickEvent, SpawnHunterTargetEvent};
use crate::resources::HunterTargetSelection;
use crate::systems::HunterSystemSet;
use crate::systems::collision::resolve_shots;
use crate::systems::input::{read_gamepad, read_mouse};
use crate::systems::lifecycle::{
    audit_pause, audit_setup, broadcast_tally, end_round, follow_sessions, forward_spawn_requests,
};
use crate::systems::movement::{release_escaped, rise_balloons};
use crate::systems::spawn::{lift_spawn_immunity, spawn_targets};
use crate::workflow::handle_hunter_phase_events;

/// Server side of Hunter: the authoritative round and its audit log
pub struct HunterGameServerPlugin;

impl Plugin for HunterGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HunterTargetSelection>()
            .add_message::<SpawnHunterTargetEvent>()
            .add_message::<HunterClickEvent>()
            .embed_pipeline(include_str!("../../../assets/pipelines/hunter/ripple.pipeline.yaml"))
            .embed_pipeline(include_str!("../../../assets/pipelines/hunter/title.pipeline.yaml"));

        use HunterSystemSet::*;
        app.configure_sets(Update, (Lifecycle, Input, Spawn, Collision, Output).chain());
        app.configure_sets(Update, (Input, Spawn, Collision).run_if(round_running));
        app.configure_sets(Update, Output.run_if(resource_exists::<HunterGameStats>));

        app.add_systems(
            Update,
            (
                (
                    follow_sessions,
                    (audit_setup, audit_pause).run_if(resource_exists::<HunterGameStats>),
                    handle_hunter_phase_events,
                )
                    .chain()
                    .in_set(Lifecycle),
                (
                    // Shares the pad with calibration: see gamepad::Btn::is_for_game
                    read_gamepad,
                    read_mouse,
                    forward_spawn_requests,
                )
                    .in_set(Input),
                (spawn_targets, lift_spawn_immunity).chain().in_set(Spawn),
                (resolve_shots, release_escaped).chain().in_set(Collision),
                broadcast_tally.in_set(Output),
            ),
        );
        app.add_systems(FixedUpdate, rise_balloons.run_if(round_running));
        app.add_systems(OnExit(ServerState::Session), end_round);
    }
}

/// A round exists and its session is active (not paused or finished)
fn round_running(stats: Option<Res<HunterGameStats>>, session: Option<Res<State<SessionState>>>) -> bool {
    stats.is_some() && session.is_some_and(|s| *s.get() == SessionState::Active)
}
