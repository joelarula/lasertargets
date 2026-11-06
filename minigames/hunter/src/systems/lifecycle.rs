use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::curator_core::{CuratorCommandsExt, CuratorEventEnvelope, WorkflowInstance, WorkflowStatus};
use common::game::{ExitGameEvent, GameSessionCreated, GameSessionUpdate};
use common::scene::SceneSetup;
use common::state::SessionState;

use crate::audit::{HunterAuditEvent, GameReport, HunterGameStats};
use crate::common::GAME_ID;
use crate::components::HunterEntity;
use crate::events::{BroadcastStatsUpdateEvent, SpawnHunterTargetEvent};
use crate::workflow::build_hunter_workflow;

/// A hunter session becoming active starts a round with a fresh audit log; a session of another
/// game, or exiting this one, ends it (saving the report)
pub fn follow_sessions(
    mut commands: Commands,
    mut created: MessageReader<GameSessionCreated>,
    mut updated: MessageReader<GameSessionUpdate>,
    mut exits: MessageReader<ExitGameEvent>,
    stats: Option<Res<HunterGameStats>>,
    entities: Query<Entity, With<HunterEntity>>,
    workflows: Query<&WorkflowInstance>,
    setup: Res<SceneSetup>,
    time: Res<Time>,
) {
    let current = stats.as_ref().map(|s| s.session_id);
    let is_active_hunter =
        |s: &common::game::GameSession| s.game_id == GAME_ID && s.state == SessionState::Active;
    let (mut start, mut end, mut new_workflow) = (None, false, false);

    for session in created.read().map(|e| &e.game_session) {
        start = is_active_hunter(session).then_some(session.session_id);
        new_workflow = start.is_some();
        end = start.is_none();
    }
    for session in updated.read().map(|e| &e.game_session) {
        if is_active_hunter(session) && current != Some(session.session_id) {
            start = Some(session.session_id);
        }
    }
    end |= exits.read().any(|e| Some(e.game_session_uuid) == current);

    if start.is_none() && !end {
        return;
    }
    let now = time.elapsed_secs_f64();
    if let Some(stats) = stats {
        finish_round(&mut commands, &stats, now, &entities);
    }
    let Some(session_id) = start else { return };

    let mut stats = HunterGameStats::new(session_id, now);
    stats.note_setup(now, &setup);
    commands.insert_resource(stats);
    info!("[Hunter] Round started for session {session_id}");

    let running = workflows
        .iter()
        .any(|w| w.session_id == Some(session_id) && w.status == WorkflowStatus::Running);
    if new_workflow && !running {
        commands.spawn_workflow_with_session(session_id, build_hunter_workflow(session_id));
    }
}

/// The server left the session: save the report and clear the round
pub fn end_round(
    mut commands: Commands,
    stats: Option<Res<HunterGameStats>>,
    entities: Query<Entity, With<HunterEntity>>,
    time: Res<Time>,
) {
    if let Some(stats) = stats {
        finish_round(&mut commands, &stats, time.elapsed_secs_f64(), &entities);
    }
}

fn finish_round(
    commands: &mut Commands,
    stats: &HunterGameStats,
    now: f64,
    entities: &Query<Entity, With<HunterEntity>>,
) {
    let report = GameReport::new(stats, now);
    match report.save() {
        Ok(dir) => info!("[Hunter] Report for session {} saved to {}", stats.session_id, dir.display()),
        Err(e) => warn!("[Hunter] Could not save report for session {}: {e}", stats.session_id),
    }
    for entity in entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<HunterGameStats>();
}

/// Logs scene, camera and projector changes so every later event can be read against the setup
/// that was in effect
pub fn audit_setup(mut stats: ResMut<HunterGameStats>, setup: Res<SceneSetup>, time: Res<Time>) {
    if setup.is_changed() {
        stats.note_setup(time.elapsed_secs_f64(), &setup);
    }
}

/// Logs pauses and resumes of the session
pub fn audit_pause(
    mut stats: ResMut<HunterGameStats>,
    session: Option<Res<State<SessionState>>>,
    mut last: Local<Option<(Uuid, SessionState)>>,
    time: Res<Time>,
) {
    let Some(state) = session.map(|s| s.get().clone()) else { return };
    let previous = last.replace((stats.session_id, state.clone()));
    let event = match (previous, &state) {
        (Some((id, SessionState::Active)), SessionState::Paused) if id == stats.session_id => HunterAuditEvent::Paused,
        (Some((id, SessionState::Paused)), SessionState::Active) if id == stats.session_id => HunterAuditEvent::Resumed,
        _ => return,
    };
    stats.record(time.elapsed_secs_f64(), event);
}

/// Sends the tally to terminals whenever it changes
pub fn broadcast_tally(
    stats: Res<HunterGameStats>,
    mut last: Local<Option<BroadcastStatsUpdateEvent>>,
    mut envelopes: MessageWriter<CuratorEventEnvelope>,
) {
    let update = BroadcastStatsUpdateEvent { session_id: stats.session_id, tally: stats.tally };
    if last.replace(update) != Some(update) {
        envelopes.write(CuratorEventEnvelope::from_payload(&update, Some(update.session_id)));
    }
}

/// Spawn requests arriving from terminals as curator envelopes
pub fn forward_spawn_requests(
    mut envelopes: MessageReader<CuratorEventEnvelope>,
    mut spawns: MessageWriter<SpawnHunterTargetEvent>,
) {
    spawns.write_batch(
        envelopes
            .read()
            .filter(|env| env.is::<SpawnHunterTargetEvent>())
            .filter_map(|env| env.downcast::<SpawnHunterTargetEvent>().ok()),
    );
}
