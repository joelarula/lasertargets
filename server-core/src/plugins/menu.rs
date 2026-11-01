use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::session::{ChannelRegistry, CloseSessionEvent, InitSessionEvent, Session};
use common::state::{SessionState, ServerState};
use curator_core::{
    ActorPrompt, ActorPromptResponseEvent, ActorTarget, AuditEvent, AuditLog,
    CuratorCommandsExt, CuratorEventEnvelope, CuratorStep, PromptChoice,
};

/// Marker component for the active server menu workflow entity.
#[derive(Component, Debug)]
pub struct ServerMenuWorkflowMarker;

/// Resource tracking the active prompt ID issued for the server menu.
#[derive(Resource, Debug, Default, Clone)]
pub struct ActiveMenuPrompt {
    pub prompt_id: Option<Uuid>,
}

/// Plugin orchestrating the Server Menu using Curator Core workflow logic.
pub struct ServerMenuWorkflowPlugin;

impl Plugin for ServerMenuWorkflowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveMenuPrompt>()
            .add_systems(Update, handle_gamepad_game_switcher)
            .add_systems(OnEnter(ServerState::Menu), spawn_menu_workflow)
            .add_systems(OnExit(ServerState::Menu), cleanup_menu_workflow)
            .add_systems(
                Update,
                (
                    handle_menu_prompt_response,
                    handle_menu_event_envelope,
                )
                    .run_if(in_state(ServerState::Menu)),
            );
    }
}

/// Spawns a managed Curator workflow when entering ServerState::Menu.
fn spawn_menu_workflow(
    mut commands: Commands,
    channel_registry: Res<ChannelRegistry>,
    mut active_prompt: ResMut<ActiveMenuPrompt>,
    mut audit_log: ResMut<AuditLog>,
) {
    info!("[ServerMenuWorkflow] Initializing Server Menu Curator workflow");

    let choices: Vec<PromptChoice> = channel_registry
        .all_apps()
        .iter()
        .map(|game| PromptChoice::new(game.id.to_string(), game.name.clone()))
        .collect();

    let prompt = ActorPrompt::new("Main Menu", "Select a game mode to launch")
        .with_choices(choices)
        .with_target(ActorTarget::All);

    let prompt_id = prompt.prompt_id;
    active_prompt.prompt_id = Some(prompt_id);

    let workflow_step = CuratorStep::prompt(prompt);
    commands
        .spawn_workflow(workflow_step)
        .insert(ServerMenuWorkflowMarker);

    let game_names: Vec<String> = channel_registry
        .all_apps()
        .iter()
        .map(|g| g.name.clone())
        .collect();

    audit_log.record(AuditEvent::PayloadDispatched {
        tag: "server:menu:opened".to_string(),
        payload: serde_json::json!({
            "available_games": game_names,
            "prompt_id": prompt_id,
        }),
        session_id: None,
        timestamp_ms: 0,
    });
}

/// Despawns the server menu workflow and cleans up resources on exiting ServerState::Menu.
fn cleanup_menu_workflow(
    mut commands: Commands,
    workflows: Query<Entity, With<ServerMenuWorkflowMarker>>,
    mut active_prompt: ResMut<ActiveMenuPrompt>,
    mut audit_log: ResMut<AuditLog>,
) {
    info!("[ServerMenuWorkflow] Cleaning up Server Menu Curator workflow");
    for entity in workflows.iter() {
        commands.entity(entity).despawn();
    }
    active_prompt.prompt_id = None;

    audit_log.record(AuditEvent::PayloadDispatched {
        tag: "server:menu:closed".to_string(),
        payload: serde_json::json!({}),
        session_id: None,
        timestamp_ms: 0,
    });
}

/// Handles actor responses to the active server menu prompt.
fn handle_menu_prompt_response(
    mut response_events: MessageReader<ActorPromptResponseEvent>,
    active_prompt: Res<ActiveMenuPrompt>,
    channel_registry: Res<ChannelRegistry>,
    mut init_session_events: MessageWriter<InitSessionEvent>,
    mut audit_log: ResMut<AuditLog>,
) {
    let Some(active_prompt_id) = active_prompt.prompt_id else {
        return;
    };

    for event in response_events.read() {
        if event.response.prompt_id != active_prompt_id {
            continue;
        }

        let choice = &event.response.selected_choice_id;
        info!("[ServerMenuWorkflow] Received prompt selection: {}", choice);

        let game_opt = choice
            .parse::<u16>()
            .ok()
            .and_then(|id| channel_registry.get_app_by_id(id))
            .or_else(|| {
                channel_registry
                    .all_apps()
                    .into_iter()
                    .find(|g| g.name.eq_ignore_ascii_case(choice))
            });

        if let Some(game) = game_opt {
            info!(
                "[ServerMenuWorkflow] Launching game '{}' (ID {}) via workflow selection",
                game.name, game.id
            );
            audit_log.record(AuditEvent::PayloadDispatched {
                tag: "server:menu:game_selected".to_string(),
                payload: serde_json::json!({
                    "game_id": game.id,
                    "game_name": game.name,
                    "actor_id": event.response.actor_id,
                }),
                session_id: None,
                timestamp_ms: 0,
            });

            init_session_events.write(InitSessionEvent {
                game_id: game.id,
                session_id: Uuid::new_v4(),
                initial_state: SessionState::Active,
            });
        } else {
            warn!("[ServerMenuWorkflow] Invalid game selection: '{}'", choice);
        }
    }
}

/// Handles dynamic CuratorEventEnvelope messages targeting the server menu.
fn handle_menu_event_envelope(
    mut envelopes: MessageReader<CuratorEventEnvelope>,
    channel_registry: Res<ChannelRegistry>,
    mut init_session_events: MessageWriter<InitSessionEvent>,
    mut audit_log: ResMut<AuditLog>,
) {
    for env in envelopes.read() {
        if env.tag == "menu:select_game" || env.tag == "menu:launch_game" {
            let game_id_opt = env
                .payload
                .get("game_id")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .or_else(|| {
                    env.payload
                    .get("game_name")
                    .and_then(|v| v.as_str())
                    .and_then(|name| {
                        channel_registry
                            .all_apps()
                            .into_iter()
                            .find(|g| g.name.eq_ignore_ascii_case(name))
                            .map(|g| g.id)
                    })
                });

            if let Some(game_id) = game_id_opt {
                if let Some(game) = channel_registry.get_app_by_id(game_id) {
                    info!(
                        "[ServerMenuWorkflow] Received launch envelope for '{}'",
                        game.name
                    );
                    audit_log.record(AuditEvent::PayloadDispatched {
                        tag: "server:menu:envelope_launch".to_string(),
                        payload: serde_json::json!({
                            "game_id": game.id,
                            "game_name": game.name,
                        }),
                        session_id: None,
                        timestamp_ms: 0,
                    });

                    init_session_events.write(InitSessionEvent {
                        game_id: game.id,
                        session_id: Uuid::new_v4(),
                        initial_state: SessionState::Active,
                    });
                }
            }
        }
    }
}

/// Handles cycling between Calibration/Menu -> Game A -> Game B -> Calibration/Menu via Gamepad (Btn::West).
fn handle_gamepad_game_switcher(
    mut button_events: MessageReader<gamepad::GamepadButtonEvent>,
    sessions: Query<&Session>,
    next_game_state: Option<Res<NextState<SessionState>>>,
    next_server_state: Res<NextState<ServerState>>,
    channel_registry: Option<Res<ChannelRegistry>>,
    mut init_events: MessageWriter<InitSessionEvent>,
    mut close_events: MessageWriter<CloseSessionEvent>,
) {
    for event in button_events.read() {
        if event.button != gamepad::Btn::West || !event.pressed {
            continue;
        }

        if matches!(*next_server_state, NextState::Pending(_)) {
            return;
        }
        if let Some(ref gs) = next_game_state {
            if matches!(**gs, NextState::Pending(_)) {
                return;
            }
        }

        let available_games = channel_registry
            .as_ref()
            .map(|r| r.all_apps())
            .unwrap_or_default();

        if available_games.is_empty() {
            info!("★ Gamepad Menu Switcher: No registered apps available");
            return;
        }

        let active_session = sessions.iter().find(|s| s.state == SessionState::Active);
        if let Some(current_session) = active_session {
            let current_game_id = current_session.game_id;
            let current_uuid = current_session.session_id;

            info!("★ Gamepad Menu Switcher: Exiting current app ID {} (session {})", current_game_id, current_uuid);
            close_events.write(CloseSessionEvent {
                session_id: current_uuid,
            });

            if let Some(curr_idx) = available_games.iter().position(|g| g.id == current_game_id) {
                if curr_idx + 1 < available_games.len() {
                    let next_game = available_games[curr_idx + 1];
                    let new_uuid = Uuid::new_v4();
                    info!("🎮 APP STARTED: {} (ID {}) [session {}]", next_game.name, next_game.id, new_uuid);
                    init_events.write(InitSessionEvent {
                        game_id: next_game.id,
                        session_id: new_uuid,
                        initial_state: SessionState::Active,
                    });
                } else {
                    info!("★ Gamepad Menu Switcher: Switched back to Calibration / Main Menu");
                }
            } else {
                info!("★ Gamepad Menu Switcher: Switched back to Calibration / Main Menu");
            }
        } else {
            let first_game = available_games[0];
            let new_uuid = Uuid::new_v4();
            info!("🎮 APP STARTED: {} (ID {}) [session {}]", first_game.name, first_game.id, new_uuid);
            init_events.write(InitSessionEvent {
                game_id: first_game.id,
                session_id: new_uuid,
                initial_state: SessionState::Active,
            });
        }
    }
}
