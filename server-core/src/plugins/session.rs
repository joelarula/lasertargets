use bevy::prelude::*;
use common::{
    session::{
        ChannelRegistry, CloseSessionEvent, ExitGameEvent, FinishGameEvent,
        FinishSessionEvent, GameSessionCreated, GameSessionUpdate,
        InitGameSessionEvent, InitSessionEvent, PauseGameEvent, PauseSessionEvent,
        ResumeGameEvent, ResumeSessionEvent, Session, SessionCreated,
        SessionUpdate, StartGameEvent, StartSessionEvent,
    },
    state::{ServerState, SessionState},
};
use curator_core::{AuditEvent, AuditLog, WorkflowCompletedEvent};

/// Plugin orchestrating session lifecycles, ECS state transitions, network notifications,
/// and Curator workflow audit integration.
pub struct SessionLifecyclePlugin;

/// Systems that apply session lifecycle events. Order after this set to observe their
/// spawns/despawns in the same frame.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionLifecycleSet;

impl Plugin for SessionLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_init_session,
                handle_start_session,
                handle_pause_session,
                handle_resume_session,
                handle_finish_session,
                handle_exit_session,
                handle_workflow_completed,
            )
                .in_set(SessionLifecycleSet),
        );
    }
}

/// Handles session creation from `InitSessionEvent` (and legacy `InitGameSessionEvent`).
fn handle_init_session(
    mut commands: Commands,
    mut next_session_state: ResMut<NextState<SessionState>>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    mut init_session_events: MessageReader<InitSessionEvent>,
    mut init_game_events: MessageReader<InitGameSessionEvent>,
    mut session_created: MessageWriter<SessionCreated>,
    mut game_session_created: MessageWriter<GameSessionCreated>,
    channel_registry: Res<ChannelRegistry>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut requests = Vec::new();

    for event in init_session_events.read() {
        requests.push((event.game_id, event.session_id, event.initial_state.clone()));
    }
    for event in init_game_events.read() {
        requests.push((event.game_id, event.game_session_uuid, event.initial_state.clone()));
    }

    for (app_id, session_uuid, initial_state) in requests {
        info!("[SessionLifecycle] Initializing session {} for app ID {}", session_uuid, app_id);

        let app = match channel_registry.get_app_by_id(app_id) {
            Some(app) => app,
            None => {
                warn!("[SessionLifecycle] App with ID {} not found in channel registry", app_id);
                continue;
            }
        };

        let mut new_session = Session {
            game_id: app_id,
            channel_name: app.name.clone(),
            session_id: session_uuid,
            name: app.name.clone(),
            state: initial_state,
            start_timestamp: None,
            end_timestamp: None,
        };

        new_session.start();
        let session = new_session.clone();
        commands.spawn(session.clone());

        next_server_state.set(ServerState::Session);
        next_session_state.set(SessionState::Active);

        // Notify canonical and legacy listeners
        session_created.write(SessionCreated {
            session: session.clone(),
        });
        game_session_created.write(GameSessionCreated {
            game_session: session.clone(),
        });

        if let Some(ref mut audit) = audit_log {
            audit.record(AuditEvent::PayloadDispatched {
                tag: "session:created".to_string(),
                payload: serde_json::json!({
                    "session_id": session_uuid,
                    "app_id": app_id,
                    "name": app.name,
                }),
                session_id: Some(session_uuid),
                timestamp_ms: 0,
            });
        }
    }
}

/// Handles session exit and teardown from `CloseSessionEvent` (and legacy `ExitGameEvent`).
fn handle_exit_session(
    mut commands: Commands,
    mut next_session_state: ResMut<NextState<SessionState>>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    sessions: Query<(Entity, &Session)>,
    mut close_session_events: MessageReader<CloseSessionEvent>,
    mut exit_game_events: MessageReader<ExitGameEvent>,
    mut broadcast_session_events: MessageWriter<SessionUpdate>,
    mut broadcast_game_events: MessageWriter<GameSessionUpdate>,
    init_session_events: MessageReader<InitSessionEvent>,
    init_game_events: MessageReader<InitGameSessionEvent>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut session_ids = Vec::new();
    for event in close_session_events.read() {
        session_ids.push(event.session_id);
    }
    for event in exit_game_events.read() {
        session_ids.push(event.game_session_uuid);
    }

    for session_id in session_ids {
        info!("[SessionLifecycle] Closing session: {}", session_id);
        if let Some((entity, current_session)) = sessions.iter().find(|(_, s)| s.session_id == session_id) {
            info!("[SessionLifecycle] Found Session entity, marking Finished and despawning");
            let mut session = current_session.clone();
            session.state = SessionState::Finished;

            broadcast_session_events.write(SessionUpdate { session: session.clone() });
            broadcast_game_events.write(GameSessionUpdate { game_session: session });
            commands.entity(entity).despawn();

            if let Some(ref mut audit) = audit_log {
                audit.record(AuditEvent::PayloadDispatched {
                    tag: "session:closed".to_string(),
                    payload: serde_json::json!({
                        "session_id": session_id,
                    }),
                    session_id: Some(session_id),
                    timestamp_ms: 0,
                });
            }
        } else {
            warn!("[SessionLifecycle] No Session entity found for session: {}", session_id);
        }

        if init_session_events.is_empty() && init_game_events.is_empty() {
            info!("[SessionLifecycle] Setting SessionState to Finished and ServerState to Menu");
            next_session_state.set(SessionState::Finished);
            next_server_state.set(ServerState::Menu);
        } else {
            info!("[SessionLifecycle] New session initializing in same frame; preserving Session state");
        }
    }
}

/// Handles starting an active session.
fn handle_start_session(
    mut sessions: Query<&mut Session>,
    mut next_state: ResMut<NextState<SessionState>>,
    mut start_session_events: MessageReader<StartSessionEvent>,
    mut start_game_events: MessageReader<StartGameEvent>,
    mut broadcast_session_events: MessageWriter<SessionUpdate>,
    mut broadcast_game_events: MessageWriter<GameSessionUpdate>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut session_ids = Vec::new();
    for e in start_session_events.read() {
        session_ids.push(e.session_id);
    }
    for e in start_game_events.read() {
        session_ids.push(e.game_session_uuid);
    }

    for session_id in session_ids {
        if let Some(mut session) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            next_state.set(SessionState::Active);
            session.start();
            broadcast_session_events.write(SessionUpdate { session: session.clone() });
            broadcast_game_events.write(GameSessionUpdate { game_session: session.clone() });

            if let Some(ref mut audit) = audit_log {
                audit.record(AuditEvent::PayloadDispatched {
                    tag: "session:started".to_string(),
                    payload: serde_json::json!({ "session_id": session_id }),
                    session_id: Some(session_id),
                    timestamp_ms: 0,
                });
            }
        }
    }
}

/// Handles pausing an active session.
fn handle_pause_session(
    mut sessions: Query<&mut Session>,
    mut next_state: ResMut<NextState<SessionState>>,
    mut pause_session_events: MessageReader<PauseSessionEvent>,
    mut pause_game_events: MessageReader<PauseGameEvent>,
    mut broadcast_session_events: MessageWriter<SessionUpdate>,
    mut broadcast_game_events: MessageWriter<GameSessionUpdate>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut session_ids = Vec::new();
    for e in pause_session_events.read() {
        session_ids.push(e.session_id);
    }
    for e in pause_game_events.read() {
        session_ids.push(e.game_session_uuid);
    }

    for session_id in session_ids {
        if let Some(mut session) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            next_state.set(SessionState::Paused);
            session.pause();
            broadcast_session_events.write(SessionUpdate { session: session.clone() });
            broadcast_game_events.write(GameSessionUpdate { game_session: session.clone() });

            if let Some(ref mut audit) = audit_log {
                audit.record(AuditEvent::PayloadDispatched {
                    tag: "session:paused".to_string(),
                    payload: serde_json::json!({ "session_id": session_id }),
                    session_id: Some(session_id),
                    timestamp_ms: 0,
                });
            }
        }
    }
}

/// Handles resuming a paused session.
fn handle_resume_session(
    mut sessions: Query<&mut Session>,
    mut next_state: ResMut<NextState<SessionState>>,
    mut resume_session_events: MessageReader<ResumeSessionEvent>,
    mut resume_game_events: MessageReader<ResumeGameEvent>,
    mut broadcast_session_events: MessageWriter<SessionUpdate>,
    mut broadcast_game_events: MessageWriter<GameSessionUpdate>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut session_ids = Vec::new();
    for e in resume_session_events.read() {
        session_ids.push(e.session_id);
    }
    for e in resume_game_events.read() {
        session_ids.push(e.game_session_uuid);
    }

    for session_id in session_ids {
        if let Some(mut session) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            next_state.set(SessionState::Active);
            session.resume();
            broadcast_session_events.write(SessionUpdate { session: session.clone() });
            broadcast_game_events.write(GameSessionUpdate { game_session: session.clone() });

            if let Some(ref mut audit) = audit_log {
                audit.record(AuditEvent::PayloadDispatched {
                    tag: "session:resumed".to_string(),
                    payload: serde_json::json!({ "session_id": session_id }),
                    session_id: Some(session_id),
                    timestamp_ms: 0,
                });
            }
        }
    }
}

/// Handles finishing an active session without necessarily closing the channel.
fn handle_finish_session(
    mut sessions: Query<&mut Session>,
    mut finish_session_events: MessageReader<FinishSessionEvent>,
    mut finish_game_events: MessageReader<FinishGameEvent>,
    mut broadcast_session_events: MessageWriter<SessionUpdate>,
    mut broadcast_game_events: MessageWriter<GameSessionUpdate>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    let mut session_ids = Vec::new();
    for e in finish_session_events.read() {
        session_ids.push(e.session_id);
    }
    for e in finish_game_events.read() {
        session_ids.push(e.game_session_uuid);
    }

    for session_id in session_ids {
        if let Some(mut session) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            session.stop();
            broadcast_session_events.write(SessionUpdate { session: session.clone() });
            broadcast_game_events.write(GameSessionUpdate { game_session: session.clone() });

            if let Some(ref mut audit) = audit_log {
                audit.record(AuditEvent::PayloadDispatched {
                    tag: "session:finished".to_string(),
                    payload: serde_json::json!({ "session_id": session_id }),
                    session_id: Some(session_id),
                    timestamp_ms: 0,
                });
            }
        }
    }
}

/// Records completed Curator workflow events and logs audit trails.
fn handle_workflow_completed(
    mut completed_events: MessageReader<WorkflowCompletedEvent>,
    mut audit_log: Option<ResMut<AuditLog>>,
) {
    for event in completed_events.read() {
        info!(
            "[SessionLifecycle] Curator workflow completed for session {:?}: {:?}",
            event.session_id, event.status
        );

        if let Some(ref mut audit) = audit_log {
            audit.record(AuditEvent::PayloadDispatched {
                tag: "workflow:completed".to_string(),
                payload: serde_json::json!({
                    "workflow_id": format!("{:?}", event.workflow_id),
                    "session_id": event.session_id,
                    "status": format!("{:?}", event.status),
                }),
                session_id: event.session_id,
                timestamp_ms: 0,
            });
        }
    }
}
