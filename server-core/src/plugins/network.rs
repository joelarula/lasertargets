use bevy::prelude::*;
use bevy_quinnet::server::endpoint::Endpoint;
use bevy_quinnet::server::{
    EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
    certificate::CertificateRetrievalMode,
};
use bevy_quinnet::shared::ClientId;
use common::actor::Actor;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::game::{ExitGameEvent, GameSession, GameSessionCreated, InitGameSessionEvent};
use common::network::{NetworkMessage, SERVER_HOST, SERVER_PORT};
use common::scene::SceneSetup;
use common::state::{CalibrationState, SessionState, ServerInstanceId, ServerState};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use common::path::BroadcastScenePaths;
use crate::plugins::actor::{
    ActorLink, ActorRegistrationResultEvent, ActorUnregistrationResultEvent, GameActorUpdateEvent,
};

#[derive(Resource, Debug, Clone)]
pub struct NetworkingConfiguration {
    pub ip: IpAddr,
    pub port: u16,
}

impl Default for NetworkingConfiguration {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED), // Binds to 0.0.0.0 (all interfaces)
            port: SERVER_PORT,
        }
    }
}

#[derive(Message, Debug, Clone)]
pub struct FromClientMessage {
    pub client_id: ClientId,
    pub message: NetworkMessage,
}

pub use common::network::{KeyboardInputEvent, MouseButtonInputEvent, MousePositionEvent};

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FromClientMessage>()
            .add_message::<GameActorUpdateEvent>()
            .add_message::<ActorRegistrationResultEvent>()
            .add_message::<ActorUnregistrationResultEvent>()
            .add_message::<MousePositionEvent>()
            .add_message::<MouseButtonInputEvent>()
            .add_message::<KeyboardInputEvent>()
            .add_message::<BroadcastScenePaths>()
            .init_resource::<NetworkingConfiguration>()
            .add_systems(Startup, start_server)
            .add_systems(Update, receive_network_messages)
            .add_systems(Update, handle_config_messages)
                        .add_systems(Update, handle_game_session_messages)
            .add_systems(Update, handle_input_messages)
            .add_systems(Update, broadcast_path_events)
            .add_systems(Update, send_ping_periodically)
            .add_systems(Update, handle_lifecycle_messages)
            .add_systems(Update, handle_game_session_created)
            .add_systems(Update, send_game_session_updates)
            .add_systems(Update, handle_game_actor_update_event)
            .add_systems(Update, handle_actor_result_events)
            .add_systems(Update, broadcast_scene_setup_on_change)
            .add_systems(Update, broadcast_state_on_change)
            .add_systems(Update, broadcast_curator_prompts)
            .add_systems(Update, broadcast_curator_envelopes)
            .add_systems(Update, handle_curator_client_messages);
    }
}

fn broadcast_path_events(
    mut server: ResMut<QuinnetServer>,
    mut path_events: MessageReader<BroadcastScenePaths>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for event in path_events.read() {
        let message = NetworkMessage::BroadcastScenePaths(event.frame.paths.clone());
        if let Ok(payload) = message.to_bytes() {
            if let Err(e) = endpoint.broadcast_payload(payload) {
                error!("Failed to broadcast BroadcastScenePaths: {}", e);
            }
        }
    }
}

/// Start the Quinnet server on startup
fn start_server(mut server: ResMut<QuinnetServer>, config: Res<NetworkingConfiguration>) {
    if server.is_listening() {
        info!("Server is already listening, skipping startup");
        return;
    }
    match server.start_endpoint(ServerEndpointConfiguration {
        addr_config: EndpointAddrConfiguration::from_ip(config.ip, config.port),
        cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
            server_hostname: "localhost".to_string(),
        },
        defaultables: Default::default(),
    }) {
        Ok(_) => {
            info!("Server started on {}:{}", SERVER_HOST, SERVER_PORT);
        }
        Err(e) => {
            error!("Failed to start server: {}", e);
            return;
        }
    }
}

/// Receive network messages and forward them to the message system
fn receive_network_messages(
    mut server: ResMut<QuinnetServer>,
    mut message_writer: MessageWriter<FromClientMessage>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    if let Some(channel_id) = endpoint.default_channel() {
        let clients = endpoint.clients();
        for client_id in clients {
            while let Some(bytes) = endpoint.try_receive_payload(client_id, channel_id) {
                match NetworkMessage::from_bytes(&bytes) {
                    Ok(message) => {
                        info!("Received message from client {}: {:?}", client_id, message);
                        message_writer.write(FromClientMessage {
                            client_id,
                            message,
                        });
                    }
                    Err(e) => {
                        error!(
                            "Failed to deserialize message from client {}: {}",
                            client_id, e
                        );
                    }
                }
            }
        }
    }
}

/// Handle configuration query and update messages
fn handle_config_messages(
    mut server: ResMut<QuinnetServer>,
    mut messages: MessageReader<FromClientMessage>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut camera_config: ResMut<CameraConfiguration>,
    mut scene_config: ResMut<SceneConfiguration>,
    scene_setup: Res<SceneSetup>,
    instance_id: Res<ServerInstanceId>,
    current_state: Res<State<ServerState>>,
    _current_session_state: Res<State<SessionState>>,
    current_calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
    mut next_server_state: ResMut<NextState<ServerState>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::SetReceiverMode { active, source_name } => {
                let target_state = if *active { ServerState::Receiver } else { ServerState::Menu };
                if current_state.get() != &target_state {
                    next_server_state.set(target_state.clone());
                    info!("Server state changed to {:?} (Receiver Mode: {}) by client {}", target_state, active, msg.client_id);
                }
                let reply = NetworkMessage::ReceiverModeUpdate { active: *active, source_name: source_name.clone() };
                if let Ok(payload) = reply.to_bytes() {
                    let _ = endpoint.broadcast_payload(payload);
                }
            }
            NetworkMessage::Pong { timestamp } => {
                info!("Received pong from client {} at timestamp {}", msg.client_id, timestamp);
            }
            NetworkMessage::QueryProjectorConfig => {
                let message = NetworkMessage::ProjectorConfigUpdate(projector_config.clone());
                let payload = message.to_bytes().expect("Serialize ProjectorConfigResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "ProjectorConfigResponse");
            }
            NetworkMessage::QueryCameraConfig => {
                let message = NetworkMessage::CameraConfigUpdate(camera_config.clone());
                let payload = message.to_bytes().expect("Serialize CameraConfigUpdate");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "CameraConfigUpdate");
            }
            NetworkMessage::QuerySceneConfig => {
                let message = NetworkMessage::SceneConfigUpdate(scene_config.clone());
                let payload = message.to_bytes().expect("Serialize SceneConfigUpdate");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "SceneConfigUpdate");
            }
            NetworkMessage::QuerySceneSetup => {
                let message = NetworkMessage::SceneSetupUpdate(scene_setup.clone());
                let payload = message.to_bytes().expect("Serialize SceneSetupResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "SceneSetupResponse");
            }
            NetworkMessage::QueryServerState => {
                // Send both ServerState and InstanceInfo
                if let Some(uuid) = instance_id.0 {
                    let info_msg = NetworkMessage::ServerInfo { instance_id: uuid };
                    let info_payload = info_msg.to_bytes().expect("Serialize ServerInfo");
                    send_payload_and_log_error(endpoint, msg.client_id, info_payload, "ServerInfoResponse");
                }

                let message = NetworkMessage::ServerStateUpdate(current_state.get().clone());
                let payload = message.to_bytes().expect("Serialize ServerStateResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "ServerStateResponse");
            }

            NetworkMessage::QueryCalibrationState => {
                let message = NetworkMessage::CalibrationStateUpdate(current_calibration_state.get().clone());
                let payload = message.to_bytes().expect("Serialize CalibrationStateResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "CalibrationStateResponse");
            }
            NetworkMessage::UpdateProjectorConfig(new_config) => {
                *projector_config = new_config.clone();
                info!("Projector configuration updated by client {}", msg.client_id);
            }
            NetworkMessage::UpdateCameraConfig(new_config) => {
                *camera_config = new_config.clone();
                info!("Camera configuration updated by client {}", msg.client_id);
            }
            NetworkMessage::UpdateSceneConfig(new_config) => {
                *scene_config = new_config.clone();
                info!("Scene configuration updated by client {}", msg.client_id);
            }
            NetworkMessage::UpdateCalibrationState(new_state) => {
                if !matches!(*next_calibration_state, NextState::Pending(_)) && current_calibration_state.get() != new_state {
                    next_calibration_state.set(new_state.clone());
                    info!("Calibration state updated by client {}: {:?}", msg.client_id, new_state);

                    let message = NetworkMessage::CalibrationStateUpdate(new_state.clone());
                    if let Ok(payload) = message.to_bytes() {
                        if let Err(e) = endpoint.broadcast_payload(payload) {
                            error!("Failed to broadcast CalibrationStateUpdate: {:?}", e);
                        }
                    }
                } else {
                    debug!("Ignoring UpdateCalibrationState({:?}) from client {}: transition already pending or unchanged", new_state, msg.client_id);
                }
            }
            _ => {}
        }
    }
}

/// Handle session channel messages (OpenSession / CloseSession)
fn handle_game_session_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut init_game_session_events: MessageWriter<InitGameSessionEvent>,
    mut exit_game_events: MessageWriter<ExitGameEvent>,
) {

    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::OpenSession { session_id, channel_name } => {
                info!("Received OpenSession for channel '{}' (session: {})", channel_name, session_id);
                let game_id = match channel_name.to_lowercase().as_str() {
                    "hunter" => 101,
                    "snake" => 102,
                    _ => 101,
                };
                init_game_session_events.write(InitGameSessionEvent {
                    game_id,
                    game_session_uuid: *session_id,
                    initial_state: common::state::SessionState::Paused,
                });
            }
            NetworkMessage::CloseSession { session_id } => {
                info!("Received CloseSession for session: {}", session_id);
                exit_game_events.write(ExitGameEvent {
                    game_session_uuid: *session_id,
                });
            }
            _ => {}
        }
    }
}

/// Handle input messages (mouse and keyboard)
fn handle_input_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut mouse_position_events: MessageWriter<MousePositionEvent>,
    mut mouse_button_events: MessageWriter<MouseButtonInputEvent>,
    mut keyboard_input_events: MessageWriter<KeyboardInputEvent>,
) {
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::UpdateMousePosition(position) => {
                mouse_position_events.write(MousePositionEvent {
                    client_id: msg.client_id,
                    position: *position,
                });
            }
            NetworkMessage::KeyboardInput { key, pressed } => {
                keyboard_input_events.write(KeyboardInputEvent {
                    client_id: msg.client_id,
                    key: key.clone(),
                    pressed: *pressed,
                });
            }
            NetworkMessage::MouseButtonInput { button, pressed, position } => {
                mouse_button_events.write(MouseButtonInputEvent {
                    client_id: msg.client_id,
                    button: button.clone(),
                    pressed: *pressed,
                    position: *position,
                });
            }
            _ => {}
        }
    }
}



/// Send periodic ping messages to all connected clients
fn send_ping_periodically(
    mut server: ResMut<QuinnetServer>,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
) {
    // Initialize timer on first run
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(10.0, TimerMode::Repeating));
    }

    let timer = timer.as_mut().unwrap();
    timer.tick(time.delta());

    if !timer.just_finished() {
        return;
    }

    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    // Get current timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let message = NetworkMessage::Ping { timestamp };

    // Serialize the message to bytes using bincode
    let payload = match message.to_bytes() {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to serialize ping message: {}", e);
            return;
        }
    };

    // Broadcast ping to all connected clients
    match endpoint.broadcast_payload(payload) {
        Ok(_) => {
            debug!("Sent ping to all clients at timestamp {}", timestamp);
        }
        Err(e) => {
            error!("Failed to broadcast ping: {}", e);
        }
    }
}

/// Broadcasts SceneSetup and individual configurations to all connected clients when they change.
fn broadcast_scene_setup_on_change(
    mut server: ResMut<QuinnetServer>,
    scene_setup: Res<SceneSetup>,
    camera_config: Res<CameraConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
    scene_configuration: Res<SceneConfiguration>,
) {
    let mut changed = false;

    // Check and broadcast CameraConfiguration changes
    if camera_config.is_changed() {
        if let Some(endpoint) = server.get_endpoint_mut() {
            let message = NetworkMessage::CameraConfigUpdate(camera_config.clone());
            if let Ok(payload) = message.to_bytes() {
                let _ = endpoint.broadcast_payload(payload);
            }
        }
        changed = true;
    }

    // Check and broadcast ProjectorConfiguration changes
    if projector_config.is_changed() {
        if let Some(endpoint) = server.get_endpoint_mut() {
            let message = NetworkMessage::ProjectorConfigUpdate(projector_config.clone());
            if let Ok(payload) = message.to_bytes() {
                let _ = endpoint.broadcast_payload(payload);
            }
        }
        changed = true;
    }

    // Check and broadcast SceneConfiguration changes
    if scene_configuration.is_changed() {
        if let Some(endpoint) = server.get_endpoint_mut() {
            let message = NetworkMessage::SceneConfigUpdate(scene_configuration.clone());
            if let Ok(payload) = message.to_bytes() {
                let _ = endpoint.broadcast_payload(payload);
            }
        }
        changed = true;
    }

    // Check and broadcast SceneSetup changes
    if scene_setup.is_changed() || changed {
        if let Some(endpoint) = server.get_endpoint_mut() {
            let message = NetworkMessage::SceneSetupUpdate(scene_setup.clone());
            if let Ok(payload) = message.to_bytes() {
                let _ = endpoint.broadcast_payload(payload);
            }
        }
    }
}

/// Helper function to send a payload and log an error if it fails.
fn send_payload_and_log_error(
    endpoint: &mut Endpoint,
    client_id: ClientId,
    payload: Vec<u8>,
    error_context: &str,
) {
    let msg_name = match NetworkMessage::from_bytes(&payload) {
        Ok(msg) => format!("{:?}", msg),
        Err(_) => "<unknown message>".to_string(),
    };
    info!("Sending message '{}' to client {} ({})", msg_name, client_id, error_context);
    if let Err(e) = endpoint.send_payload(client_id, payload) {
        error!(
            "Failed to send {} to client {}: {}",
            error_context, client_id, e
        );
    }
}

/// Helper function to broadcast a payload and log an error if it fails.
fn broadcast_payload_and_log_error(endpoint: &mut Endpoint, payload: Vec<u8>, error_context: &str) {
    let msg_name = match NetworkMessage::from_bytes(&payload) {
        Ok(msg) => format!("{:?}", msg),
        Err(_) => "<unknown message>".to_string(),
    };
    info!("Broadcasting message '{}' to all clients ({})", msg_name, error_context);
    if let Err(e) = endpoint.broadcast_payload(payload) {
        error!("Failed to broadcast {}: {}", error_context, e);
    }
}

fn send_game_session_updates(
    mut server: ResMut<QuinnetServer>,
    game_sessions: Query<(&GameSession, Option<&Children>), Changed<GameSession>>,
    actor_links: Query<&ActorLink>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (game_session, maybe_children) in game_sessions.iter() {
        info!("GameSession changed: {:?}", game_session);

        // Collect clients that own actors parented to this game session
        let mut recipients: HashSet<ClientId> = HashSet::new();
        if let Some(children) = maybe_children {
            recipients.reserve(children.len());
            for child in children.iter() {
                if let Ok(link) = actor_links.get(child) {
                    recipients.insert(link.client_id);
                }
            }
        }

        // If no actors are registered yet, broadcast to all clients
        if recipients.is_empty() {
            let message = NetworkMessage::SessionUpdate(game_session.clone().into());
            let payload = message.to_bytes().expect("Serialize GameSessionResponse");
            broadcast_payload_and_log_error(
                endpoint,
                payload,
                "GameSessionResponse update (broadcast)",
            );
            continue;
        }

        let message = NetworkMessage::SessionUpdate(game_session.clone().into());
        let payload = message.to_bytes().expect("Serialize GameSessionResponse");

        // Send only to clients that have actors in this session
        for client_id in recipients {
            send_payload_and_log_error(
                endpoint,
                client_id,
                payload.clone(),
                "GameSessionResponse update",
            );
        }
    }
}

fn handle_game_actor_update_event(
    mut server: ResMut<QuinnetServer>,
    mut updates: MessageReader<GameActorUpdateEvent>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for update in updates.read() {
        let mut per_client: HashMap<ClientId, Vec<Actor>> = HashMap::new();
        for link in &update.actors {
            per_client
                .entry(link.client_id)
                .or_default()
                .push(link.actor.clone());
        }

        for (client_id, actors) in per_client {
            let env = common::curator_core::CuratorEventEnvelope::new(
                "actor_response",
                serde_json::to_value(&common::actor::ActorMetaData {
                    game_session_uuid: update.game_uuid,
                    actors,
                }).unwrap_or_default(),
            ).with_session(update.game_uuid);
            if let Ok(payload) = NetworkMessage::CuratorEnvelope(env).to_bytes() {
                send_payload_and_log_error(endpoint, client_id, payload, "CuratorEnvelope(actor_response)");
            }
        }
    }
}

fn handle_game_session_created(
    mut server: ResMut<QuinnetServer>,
    mut game_sessions: MessageReader<GameSessionCreated>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for created in game_sessions.read() {
        info!("[Network] Received GameSessionCreated: {:?}", created.game_session);
        let message = NetworkMessage::SessionCreated(created.game_session.clone().into());
        let payload = message
            .to_bytes()
            .expect("Serialize GameSessionResponse (created)");

        // Log all connected clients before broadcasting
        let clients = endpoint.clients();
        info!("[Network] Broadcasting GameSessionCreated to clients: {:?}", clients);
        let result = endpoint.broadcast_payload(payload);
        match result {
            Ok(_) => info!("[Network] Broadcast GameSessionCreated succeeded"),
            Err(e) => error!("[Network] Broadcast GameSessionCreated failed: {}", e),
        }
    }
}

fn handle_actor_result_events(
    mut server: ResMut<QuinnetServer>,
    mut reg_results: MessageReader<ActorRegistrationResultEvent>,
    mut unreg_results: MessageReader<ActorUnregistrationResultEvent>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    // Registration results
    for result in reg_results.read() {
        let (tag, val) = match &result.result {
            Ok(actor) => (
                "actor_response",
                serde_json::to_value(&common::actor::ActorMetaData {
                    game_session_uuid: result.game_uuid,
                    actors: vec![actor.clone()],
                }).unwrap_or_default(),
            ),
            Err(msg) => ("actor_error", serde_json::Value::String(msg.clone())),
        };
        let env = common::curator_core::CuratorEventEnvelope::new(tag, val).with_session(result.game_uuid);
        if let Ok(payload) = NetworkMessage::CuratorEnvelope(env).to_bytes() {
            send_payload_and_log_error(endpoint, result.client_id, payload, tag);
        }
    }

    // Unregistration results
    for result in unreg_results.read() {
        let (tag, val) = match &result.result {
            Ok(actor) => (
                "actor_response",
                serde_json::to_value(&common::actor::ActorMetaData {
                    game_session_uuid: result.game_uuid,
                    actors: vec![actor.clone()],
                }).unwrap_or_default(),
            ),
            Err(msg) => ("actor_error", serde_json::Value::String(msg.clone())),
        };
        let env = common::curator_core::CuratorEventEnvelope::new(tag, val).with_session(result.game_uuid);
        if let Ok(payload) = NetworkMessage::CuratorEnvelope(env).to_bytes() {
            send_payload_and_log_error(endpoint, result.client_id, payload, tag);
        }
    }
}

/// Broadcasts ServerState and CalibrationState to all clients when they change.
fn broadcast_state_on_change(
    mut server: ResMut<QuinnetServer>,
    server_state: Res<State<ServerState>>,
    _session_state: Res<State<SessionState>>,
    calibration_state: Res<State<CalibrationState>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    if server_state.is_changed() {
        info!("[broadcast_state_on_change] ServerState changed, broadcasting ServerStateUpdate: {:?}", server_state.get());
        let msg = NetworkMessage::ServerStateUpdate(server_state.get().clone());
        if let Ok(payload) = msg.to_bytes() {
            let _ = endpoint.broadcast_payload(payload);
        }
    }



    if calibration_state.is_changed() {
        info!("[broadcast_state_on_change] CalibrationState changed, broadcasting CalibrationStateUpdate: {:?}", calibration_state.get());
        let msg = NetworkMessage::CalibrationStateUpdate(calibration_state.get().clone());
        if let Ok(payload) = msg.to_bytes() {
            let _ = endpoint.broadcast_payload(payload);
        }
    }
}



/// Handle server lifecycle messages (Shutdown)
fn handle_lifecycle_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut exit_writer: MessageWriter<bevy::app::AppExit>,
) {
    for msg in messages.read() {
        if matches!(msg.message, NetworkMessage::ShutdownServer) {
            info!("Received ShutdownServer from client {}. Exiting...", msg.client_id);
            exit_writer.write(bevy::app::AppExit::Success);
        }
    }
}

fn broadcast_curator_prompts(
    mut server: ResMut<QuinnetServer>,
    mut prompt_requests: MessageReader<curator_core::ActorPromptRequestEvent>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    for event in prompt_requests.read() {
        let msg = NetworkMessage::CuratorPrompt(event.prompt.clone());
        if let Ok(payload) = msg.to_bytes() {
            info!(
                "[NetworkingPlugin] Broadcasting CuratorPrompt '{}' (id: {})",
                event.prompt.title, event.prompt.prompt_id
            );
            let _ = endpoint.broadcast_payload(payload);
        }
    }
}

fn broadcast_curator_envelopes(
    mut server: ResMut<QuinnetServer>,
    mut envelopes: MessageReader<curator_core::CuratorEventEnvelope>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    for env in envelopes.read() {
        let msg = NetworkMessage::CuratorEnvelope(env.clone());
        if let Ok(payload) = msg.to_bytes() {
            let _ = endpoint.broadcast_payload(payload);
        }
    }
}

fn handle_curator_client_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut prompt_responses: MessageWriter<curator_core::ActorPromptResponseEvent>,
    mut envelopes: MessageWriter<curator_core::CuratorEventEnvelope>,
) {
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::CuratorPromptResponse(resp) => {
                info!(
                    "[NetworkingPlugin] Received CuratorPromptResponse for prompt {}",
                    resp.prompt_id
                );
                prompt_responses.write(curator_core::ActorPromptResponseEvent {
                    response: resp.clone(),
                });
            }
            NetworkMessage::CuratorEnvelope(env) => {
                info!(
                    "[NetworkingPlugin] Received CuratorEnvelope tag '{}'",
                    env.tag
                );
                envelopes.write(env.clone());
            }
            _ => {}
        }
    }
}
