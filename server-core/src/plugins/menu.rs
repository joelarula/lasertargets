use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use common::game::{GameRegistry, InitGameSessionEvent};
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
    game_registry: Res<GameRegistry>,
    mut active_prompt: ResMut<ActiveMenuPrompt>,
    mut audit_log: ResMut<AuditLog>,
) {
    info!("[ServerMenuWorkflow] Initializing Server Menu Curator workflow");

    let choices: Vec<PromptChoice> = game_registry
        .all_games()
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

    let game_names: Vec<String> = game_registry
        .all_games()
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
    game_registry: Res<GameRegistry>,
    mut init_game_events: MessageWriter<InitGameSessionEvent>,
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
            .and_then(|id| game_registry.get_game_by_id(id))
            .or_else(|| {
                game_registry
                    .all_games()
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

            init_game_events.write(InitGameSessionEvent {
                game_id: game.id,
                game_session_uuid: Uuid::new_v4(),
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
    game_registry: Res<GameRegistry>,
    mut init_game_events: MessageWriter<InitGameSessionEvent>,
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
                            game_registry
                                .all_games()
                                .into_iter()
                                .find(|g| g.name.eq_ignore_ascii_case(name))
                                .map(|g| g.id)
                        })
                });

            if let Some(game_id) = game_id_opt {
                if let Some(game) = game_registry.get_game_by_id(game_id) {
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

                    init_game_events.write(InitGameSessionEvent {
                        game_id: game.id,
                        game_session_uuid: Uuid::new_v4(),
                        initial_state: SessionState::Active,
                    });
                }
            }
        }
    }
}
