use bevy::prelude::*;
use bevy_quinnet::server::endpoint::Endpoint;
use bevy_quinnet::server::{
    EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
    certificate::CertificateRetrievalMode,
};
use bevy_quinnet::shared::ClientId;
use common::actor::ActorLink;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::{NetworkMessage, SERVER_PORT};
use common::path::BroadcastScenePaths;
use common::scene::SceneSetup;
use common::session::{ChannelRegistry, CloseSessionEvent, InitSessionEvent, Session, SessionCreated, SessionUpdate};
use crate::plugins::session::SessionLifecycleSet;
use common::state::{CalibrationState, ServerInstanceId, ServerState};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};

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
            .add_message::<MousePositionEvent>()
            .add_message::<MouseButtonInputEvent>()
            .add_message::<KeyboardInputEvent>()
            .init_resource::<NetworkingConfiguration>()
            .add_systems(Startup, start_server)
            .add_systems(Update, receive_network_messages)
            .add_systems(Update, handle_config_messages)
            .add_systems(Update, handle_session_messages)
            .add_systems(Update, handle_input_messages)
            .add_systems(Update, broadcast_path_events)
            .add_systems(
                Update,
                send_ping_periodically.run_if(
                    bevy::time::common_conditions::on_timer(std::time::Duration::from_secs(10)),
                ),
            )
            .add_systems(Update, handle_lifecycle_messages)
            .add_systems(Update, handle_session_created)
            .add_systems(Update, send_session_updates)
            .add_systems(Update, send_closed_session_updates.after(SessionLifecycleSet))
            .add_systems(Update, broadcast_scene_setup_on_change)
            .add_systems(Update, broadcast_state_on_change)
            .add_systems(Update, broadcast_curator_prompts)
            .add_systems(Update, broadcast_curator_envelopes)
            .add_systems(Update, handle_curator_client_messages);
    }
}

/// Helper function to send a network message to a specific client and log any error.
fn send_network_message(
    endpoint: &mut Endpoint,
    client_id: ClientId,
    message: &NetworkMessage,
    error_context: &str,
) {
    debug!("Sending message {:?} to client {} ({})", message, client_id, error_context);
    match message.to_bytes() {
        Ok(payload) => {
            if let Err(e) = endpoint.send_payload(client_id, payload) {
                error!(
                    "Failed to send {} to client {}: {}",
                    error_context, client_id, e
                );
            }
        }
        Err(e) => {
            error!(
                "Failed to serialize {} for client {}: {}",
                error_context, client_id, e
            );
        }
    }
}

/// Helper function to broadcast a network message to all clients and log any error.
fn broadcast_network_message(endpoint: &mut Endpoint, message: &NetworkMessage, error_context: &str) {
    debug!("Broadcasting message {:?} ({})", message, error_context);
    match message.to_bytes() {
        Ok(payload) => {
            if let Err(e) = endpoint.broadcast_payload(payload) {
                error!("Failed to broadcast {}: {}", error_context, e);
            }
        }
        Err(e) => {
            error!("Failed to serialize {} for broadcast: {}", error_context, e);
        }
    }
}

/// Broadcast path events to all clients
fn broadcast_path_events(
    mut server: ResMut<QuinnetServer>,
    mut path_events: MessageReader<BroadcastScenePaths>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for event in path_events.read() {
        let message = NetworkMessage::BroadcastScenePaths(event.frame.clone());
        broadcast_network_message(endpoint, &message, "BroadcastScenePaths");
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
            info!("Server started on {}:{}", config.ip, config.port);
        }
        Err(e) => {
            error!("Failed to start server: {}", e);
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
                broadcast_network_message(endpoint, &reply, "ReceiverModeUpdate");
            }
            NetworkMessage::Pong { timestamp } => {
                info!("Received pong from client {} at timestamp {}", msg.client_id, timestamp);
            }
            NetworkMessage::QueryProjectorConfig => {
                let message = NetworkMessage::ProjectorConfigUpdate(projector_config.clone());
                send_network_message(endpoint, msg.client_id, &message, "ProjectorConfigResponse");
            }
            NetworkMessage::QueryCameraConfig => {
                let message = NetworkMessage::CameraConfigUpdate(camera_config.clone());
                send_network_message(endpoint, msg.client_id, &message, "CameraConfigUpdate");
            }
            NetworkMessage::QuerySceneConfig => {
                let message = NetworkMessage::SceneConfigUpdate(scene_config.clone());
                send_network_message(endpoint, msg.client_id, &message, "SceneConfigUpdate");
            }
            NetworkMessage::QuerySceneSetup => {
                let message = NetworkMessage::SceneSetupUpdate(scene_setup.clone());
                send_network_message(endpoint, msg.client_id, &message, "SceneSetupResponse");
            }
            NetworkMessage::QueryServerState => {
                if let Some(uuid) = instance_id.0 {
                    let info_msg = NetworkMessage::ServerInfo { instance_id: uuid };
                    send_network_message(endpoint, msg.client_id, &info_msg, "ServerInfoResponse");
                }

                let message = NetworkMessage::ServerStateUpdate(current_state.get().clone());
                send_network_message(endpoint, msg.client_id, &message, "ServerStateResponse");
            }

            NetworkMessage::QueryCalibrationState => {
                let message = NetworkMessage::CalibrationStateUpdate(current_calibration_state.get().clone());
                send_network_message(endpoint, msg.client_id, &message, "CalibrationStateResponse");
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
                    // Authoritative state change is broadcast by broadcast_state_on_change
                } else {
                    debug!("Ignoring UpdateCalibrationState({:?}) from client {}: transition already pending or unchanged", new_state, msg.client_id);
                }
            }
            _ => {}
        }
    }
}

/// Handle session channel messages (OpenSession / CloseSession)
fn handle_session_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut init_session_events: MessageWriter<InitSessionEvent>,
    mut close_session_events: MessageWriter<CloseSessionEvent>,
    channel_registry: Option<Res<ChannelRegistry>>,
) {
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::OpenSession { session_id, channel_name } => {
                info!("Received OpenSession for channel '{}' (session: {})", channel_name, session_id);
                let game_id = channel_registry
                    .as_ref()
                    .and_then(|reg| reg.get_app_by_name(channel_name))
                    .map(|app| app.id)
                    .unwrap_or_else(|| {
                        match channel_name.to_lowercase().as_str() {
                            "snake" => 102,
                            _ => 101,
                        }
                    });
                init_session_events.write(InitSessionEvent {
                    game_id,
                    session_id: *session_id,
                    initial_state: common::state::SessionState::Paused,
                });
            }
            NetworkMessage::CloseSession { session_id } => {
                info!("Received CloseSession for session: {}", session_id);
                close_session_events.write(CloseSessionEvent {
                    session_id: *session_id,
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
fn send_ping_periodically(mut server: ResMut<QuinnetServer>) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let message = NetworkMessage::Ping { timestamp };
    broadcast_network_message(endpoint, &message, "Ping");
}

/// Broadcasts SceneSetup and individual configurations to all connected clients when they change.
fn broadcast_scene_setup_on_change(
    mut server: ResMut<QuinnetServer>,
    scene_setup: Res<SceneSetup>,
    camera_config: Res<CameraConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
    scene_configuration: Res<SceneConfiguration>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    let mut changed = false;

    // Check and broadcast CameraConfiguration changes
    if camera_config.is_changed() {
        let message = NetworkMessage::CameraConfigUpdate(camera_config.clone());
        broadcast_network_message(endpoint, &message, "CameraConfigUpdate");
        changed = true;
    }

    // Check and broadcast ProjectorConfiguration changes
    if projector_config.is_changed() {
        let message = NetworkMessage::ProjectorConfigUpdate(projector_config.clone());
        broadcast_network_message(endpoint, &message, "ProjectorConfigUpdate");
        changed = true;
    }

    // Check and broadcast SceneConfiguration changes
    if scene_configuration.is_changed() {
        let message = NetworkMessage::SceneConfigUpdate(scene_configuration.clone());
        broadcast_network_message(endpoint, &message, "SceneConfigUpdate");
        changed = true;
    }

    // Check and broadcast SceneSetup changes
    if scene_setup.is_changed() || changed {
        let message = NetworkMessage::SceneSetupUpdate(scene_setup.clone());
        broadcast_network_message(endpoint, &message, "SceneSetupUpdate");
    }
}

fn send_session_updates(
    mut server: ResMut<QuinnetServer>,
    sessions: Query<(&Session, Option<&Children>), Changed<Session>>,
    actor_links: Query<&ActorLink>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for (session, maybe_children) in sessions.iter() {
        info!("Session changed: {:?}", session);

        // Collect clients that own actors parented to this session
        let mut recipients: HashSet<ClientId> = HashSet::new();
        if let Some(children) = maybe_children {
            recipients.reserve(children.len());
            for child in children.iter() {
                if let Ok(link) = actor_links.get(child) {
                    recipients.insert(link.client_id);
                }
            }
        }

        let message = NetworkMessage::SessionUpdate(session.clone().into());

        // If no actors are registered yet, broadcast to all clients
        if recipients.is_empty() {
            broadcast_network_message(endpoint, &message, "SessionUpdate (broadcast)");
        } else {
            // Send only to clients that have actors in this session
            for client_id in recipients {
                send_network_message(endpoint, client_id, &message, "SessionUpdate");
            }
        }
    }
}

/// Broadcast the final SessionUpdate of sessions that were despawned this frame.
/// Live sessions are covered by `send_session_updates` via `Changed<Session>`; a closed
/// session's entity is gone before that can fire, so its update is forwarded from the message.
fn send_closed_session_updates(
    mut server: ResMut<QuinnetServer>,
    mut updates: MessageReader<SessionUpdate>,
    sessions: Query<&Session>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for update in updates.read() {
        let id = update.session.session_id;
        if sessions.iter().any(|s| s.session_id == id) {
            continue;
        }
        let message = NetworkMessage::SessionUpdate(update.session.clone());
        broadcast_network_message(endpoint, &message, "SessionUpdate (closed)");
    }
}

fn handle_session_created(
    mut server: ResMut<QuinnetServer>,
    mut sessions: MessageReader<SessionCreated>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for created in sessions.read() {
        info!("[Network] Received SessionCreated: {:?}", created.session);
        let message = NetworkMessage::SessionCreated(created.session.clone().into());
        let clients = endpoint.clients();
        info!("[Network] Broadcasting SessionCreated to clients: {:?}", clients);
        broadcast_network_message(endpoint, &message, "SessionCreated");
    }
}

/// Broadcasts ServerState and CalibrationState to all clients when they change.
fn broadcast_state_on_change(
    mut server: ResMut<QuinnetServer>,
    server_state: Res<State<ServerState>>,
    calibration_state: Res<State<CalibrationState>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    if server_state.is_changed() {
        info!(
            "[broadcast_state_on_change] ServerState changed, broadcasting ServerStateUpdate: {:?}",
            server_state.get()
        );
        let msg = NetworkMessage::ServerStateUpdate(server_state.get().clone());
        broadcast_network_message(endpoint, &msg, "ServerStateUpdate");
    }

    if calibration_state.is_changed() {
        info!(
            "[broadcast_state_on_change] CalibrationState changed, broadcasting CalibrationStateUpdate: {:?}",
            calibration_state.get()
        );
        let msg = NetworkMessage::CalibrationStateUpdate(calibration_state.get().clone());
        broadcast_network_message(endpoint, &msg, "CalibrationStateUpdate");
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
        info!(
            "[NetworkingPlugin] Broadcasting CuratorPrompt '{}' (id: {})",
            event.prompt.title, event.prompt.prompt_id
        );
        broadcast_network_message(endpoint, &msg, "CuratorPrompt");
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
        broadcast_network_message(endpoint, &msg, "CuratorEnvelope");
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
