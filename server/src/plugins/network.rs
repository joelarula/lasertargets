use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy_quinnet::server::endpoint::Endpoint;
use bevy_quinnet::server::{
    EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
    certificate::CertificateRetrievalMode,
};
use bevy_quinnet::shared::ClientId;
use common::actor::{Actor, ActorMetaData};
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::game::{ExitGameEvent, FinishGameEvent, GameSession, GameSessionCreated, GameSessionUpdate, InitGameSessionEvent, PauseGameEvent, ResumeGameEvent, StartGameEvent};
use common::network::{NetworkMessage, SERVER_HOST, SERVER_PORT};
use common::scene::SceneSetup;
use common::state::{CalibrationState, GameState, ServerState};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv6Addr};
use hunter::model::{BroadcastStatsUpdateEvent, HunterClickEvent};
use hunter::server::SpawnHunterTargetEvent;

use crate::plugins::actor::{
    ActorLink, ActorRegistrationResultEvent, ActorUnregistrationResultEvent, GameActorUpdateEvent,
    RegisterActorEvent, UnregisterActorEvent,
};

#[derive(Resource, Debug, Clone)]
pub struct NetworkingConfiguration {
    pub ip: IpAddr,
    pub port: u16,
}

impl Default for NetworkingConfiguration {
    fn default() -> Self {
        Self {
            ip: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: SERVER_PORT,
        }
    }
}

#[derive(Message, Debug, Clone)]
pub struct FromClientMessage {
    pub client_id: ClientId,
    pub message: NetworkMessage,
}

/// Event for mouse position updates from clients
#[derive(Message, Debug, Clone)]
pub struct MousePositionEvent {
    pub client_id: u64,
    pub position: Option<Vec3>,
}

/// Event for keyboard input from clients
#[derive(Message, Debug, Clone)]
pub struct KeyboardInputEvent {
    pub client_id: u64,
    pub key: String,
    pub pressed: bool,
}

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FromClientMessage>()
            .add_message::<GameActorUpdateEvent>()
            .add_message::<ActorRegistrationResultEvent>()
            .add_message::<ActorUnregistrationResultEvent>()
            .add_message::<MousePositionEvent>()
            .add_message::<KeyboardInputEvent>()
            .add_message::<SpawnHunterTargetEvent>()
            .add_message::<HunterClickEvent>()
            .add_message::<BroadcastStatsUpdateEvent>()
            .init_resource::<NetworkingConfiguration>()
            .add_systems(Startup, start_server)
            .add_systems(Update, receive_network_messages)
            .add_systems(Update, handle_config_messages)
            .add_systems(Update, handle_actor_messages)
            .add_systems(Update, handle_game_session_messages)
            .add_systems(Update, handle_input_messages)
            .add_systems(Update, handle_hunter_target_messages)
            .add_systems(Update, broadcast_hunter_events)
            .add_systems(Update, send_ping_periodically)
            .add_systems(Update, handle_game_session_created)
            .add_systems(Update, send_game_session_updates)
            .add_systems(Update, handle_game_actor_update_event)
            .add_systems(Update, handle_actor_result_events)
            .add_systems(Update, broadcast_scene_setup_on_change)
            .add_systems(Update, broadcast_state_on_change);
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
    current_state: Res<State<ServerState>>,
    current_game_state: Res<State<GameState>>,
    current_calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    
    for msg in messages.read() {
        match &msg.message {
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
                let message = NetworkMessage::ServerStateUpdate(current_state.get().clone());
                let payload = message.to_bytes().expect("Serialize ServerStateResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "ServerStateResponse");
            }
            NetworkMessage::QueryGameState => {
                let message = NetworkMessage::GameStateUpdate(current_game_state.get().clone());
                let payload = message.to_bytes().expect("Serialize GameStateResponse");
                send_payload_and_log_error(endpoint, msg.client_id, payload, "GameStateResponse");
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
                next_calibration_state.set(new_state.clone());
                info!("Calibration state updated by client {}: {:?}", msg.client_id, new_state);

                let message = NetworkMessage::CalibrationStateUpdate(new_state.clone());
                if let Ok(payload) = message.to_bytes() {
                    if let Err(e) = endpoint.broadcast_payload(payload) {
                        error!("Failed to broadcast CalibrationStateUpdate: {:?}", e);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Handle actor registration and unregistration messages
fn handle_actor_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut register_actor_events: MessageWriter<RegisterActorEvent>,
    mut unregister_actor_events: MessageWriter<UnregisterActorEvent>,
) {
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::RegisterActor(game, name, roles) => {
                let actor = Actor {
                    uuid: Uuid::new_v4(),
                    name: name.clone(),
                    roles: roles.clone(),
                };
                register_actor_events.write(RegisterActorEvent {
                    client_id: msg.client_id,
                    game_session_id: *game,
                    actor,
                });
            }
            NetworkMessage::UnregisterActor(game_uuid, actor_uuid) => {
                unregister_actor_events.write(UnregisterActorEvent {
                    client_id: msg.client_id,
                    actor_uuid: *actor_uuid,
                    game_uuid: *game_uuid,
                });
            }
            _ => {}
        }
    }
}

/// Handle game session management messages
fn handle_game_session_messages(
    mut server: ResMut<QuinnetServer>,
    mut messages: MessageReader<FromClientMessage>,
    mut start_game_session_events: MessageWriter<StartGameEvent>,
    mut init_game_session_events: MessageWriter<InitGameSessionEvent>,
    mut pause_game_events: MessageWriter<PauseGameEvent>,
    mut resume_game_events: MessageWriter<ResumeGameEvent>,
    mut finish_game_events: MessageWriter<FinishGameEvent>,
    mut exit_game_events: MessageWriter<ExitGameEvent>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    mut game_sessions: Query<&mut GameSession>,
    mut broadcast_events: MessageWriter<GameSessionUpdate>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::QueryGameSession => {
                if let Some(session) = game_sessions.iter().next() {
                    let message = NetworkMessage::GameSessionUpdate(session.clone());
                    if let Ok(payload) = message.to_bytes() {
                        send_payload_and_log_error(endpoint, msg.client_id, payload, "GameSessionResponse");
                    }
                } else {
                    debug!("No active game session to return to client {}", msg.client_id);
                }
            }
            NetworkMessage::InitGameSession(session_uuid, game_id, initial_state) => {
                info!("Received InitGameSession for game ID: {} with initial state: {:?}", game_id, initial_state);
                init_game_session_events.write(InitGameSessionEvent {
                    game_id: *game_id,
                    game_session_uuid: *session_uuid,
                    initial_state: initial_state.clone(),
                });

            }
            NetworkMessage::StartGameSession(uuid) => {
                info!("Received StartGameSession for: {}", uuid);
                start_game_session_events.write(StartGameEvent {
                    game_session_uuid: *uuid,
                });
            }
            NetworkMessage::PauseGameSession(uuid) => {
                info!("Received PauseGame for: {}", uuid);
                pause_game_events.write(PauseGameEvent {
                    game_session_uuid: *uuid,
                });
            }
            NetworkMessage::ResumeGameSession(uuid) => {
                info!("Received ResumeGame for: {}", uuid);
                resume_game_events.write(ResumeGameEvent {
                    game_session_uuid: *uuid,
                });
            }
            NetworkMessage::FinishGameSession(uuid) => {
                info!("Received FinishGameSession for: {}", uuid);
                finish_game_events.write(FinishGameEvent {
                    game_session_uuid: *uuid,
                });
            }
            NetworkMessage::ExitGameSession(uuid) => {
                info!("Received ExitGameSession for: {}", uuid);
                // Set server state to Menu and session to Finished
                exit_game_events.write(ExitGameEvent {
                    game_session_uuid: *uuid,
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
    mut keyboard_input_events: MessageWriter<KeyboardInputEvent>,
    mut hunter_click_events: MessageWriter<HunterClickEvent>,
    game_sessions: Query<&GameSession>,
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
                // Forward mouse clicks to hunter game if session is active
                if button == "Left" && *pressed {
                    if let Some(position) = position {
                        // Check if there's an active hunter game session (game_id 101)
                        for session in game_sessions.iter() {
                            if session.game_id == 101 { // Hunter game ID
                                hunter_click_events.write(HunterClickEvent {
                                    session_id: session.session_id,
                                    click_position: *position,
                                });
                                break;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Handle hunter target spawn messages
fn handle_hunter_target_messages(
    mut messages: MessageReader<FromClientMessage>,
    mut spawn_hunter_target_events: MessageWriter<SpawnHunterTargetEvent>,
) {
    for msg in messages.read() {
        match &msg.message {
            NetworkMessage::SpawnHunterTarget(target, position) => {
                info!("Received SpawnHunterTarget message from client {}", msg.client_id);
                info!("Received SpawnHunterTarget from client {}: target={:?}, position={:?}", msg.client_id, target, position);
                spawn_hunter_target_events.write(SpawnHunterTargetEvent {
                    target: target.clone(),
                    position: *position,
                });
            }
            _ => {}
        }
    }
}

/// Broadcast hunter game events to all clients
fn broadcast_hunter_events(
    mut server: ResMut<QuinnetServer>,
    mut stats_events: MessageReader<BroadcastStatsUpdateEvent>,
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    
    // Broadcast stats update events
    for event in stats_events.read() {
        let message = NetworkMessage::HunterStatsUpdate {
            session_id: event.session_id,
            targets_spawned: event.targets_spawned,
            targets_popped: event.targets_popped,
            score: event.score,
        };
        
        if let Ok(payload) = message.to_bytes() {
            if let Err(e) = endpoint.broadcast_payload(payload) {
                error!("Failed to broadcast stats update: {:?}", e);
            }
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
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    // Check and broadcast CameraConfiguration changes
    if camera_config.is_changed() {
        info!("CameraConfiguration changed, broadcasting update: {:?}", camera_config);
        let message = NetworkMessage::CameraConfigUpdate(camera_config.clone());
        let payload = message.to_bytes().expect("Serialize CameraConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast CameraConfiguration update: {}", e);
        }
    }

    // Check and broadcast ProjectorConfiguration changes
    if projector_config.is_changed() {
        info!("ProjectorConfiguration changed, broadcasting update: {:?}", projector_config);
        let message = NetworkMessage::ProjectorConfigUpdate(projector_config.clone());
        let payload = message.to_bytes().expect("Serialize ProjectorConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast ProjectorConfiguration update: {}", e);
        }
    }

    // Check and broadcast SceneConfiguration changes
    if scene_configuration.is_changed() {
        info!("SceneConfiguration changed, broadcasting update: {:?}", scene_configuration);
        let message = NetworkMessage::SceneConfigUpdate(scene_configuration.clone());
        let payload = message.to_bytes().expect("Serialize SceneConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast SceneConfiguration update: {}", e);
        }
    }

    // Check and broadcast SceneSetup changes (if still desired, as it aggregates the above)
    if scene_setup.is_changed() {
        info!("SceneSetup changed, broadcasting update: {:?}", scene_setup);
        let message = NetworkMessage::SceneSetupUpdate(scene_setup.clone());
        let payload = message.to_bytes().expect("Serialize SceneSetupResponse");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast SceneSetup: {}", e);
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
            let message = NetworkMessage::GameSessionUpdate(game_session.clone());
            let payload = message.to_bytes().expect("Serialize GameSessionResponse");
            broadcast_payload_and_log_error(
                endpoint,
                payload,
                "GameSessionResponse update (broadcast)",
            );
            continue;
        }

        let message = NetworkMessage::GameSessionUpdate(game_session.clone());
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
            let payload = NetworkMessage::ActorResponse(ActorMetaData {
                game_session_uuid: update.game_uuid,
                actors,
            })
            .to_bytes()
            .expect("Serialize ActorResponse");

            send_payload_and_log_error(endpoint, client_id, payload, "ActorResponse");
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
        let message = NetworkMessage::GameSessionCreated(created.game_session.clone());
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
        let payload = match &result.result {
            Ok(actor) => NetworkMessage::ActorResponse(ActorMetaData {
                game_session_uuid: result.game_uuid,
                actors: vec![actor.clone()],
            })
            .to_bytes()
            .expect("Serialize ActorResponse"),
            Err(msg) => NetworkMessage::ActorError(msg.clone())
                .to_bytes()
                .expect("Serialize ActorError"),
        };

        send_payload_and_log_error(
            endpoint,
            result.client_id,
            payload,
            "Actor registration result",
        );
    }

    // Unregistration results
    for result in unreg_results.read() {
        let payload = match &result.result {
            Ok(actor) => NetworkMessage::ActorResponse(ActorMetaData {
                game_session_uuid: result.game_uuid,
                actors: vec![actor.clone()],
            })
            .to_bytes()
            .expect("Serialize ActorResponse"),
            Err(msg) => NetworkMessage::ActorError(msg.clone())
                .to_bytes()
                .expect("Serialize ActorError"),
        };

        send_payload_and_log_error(
            endpoint,
            result.client_id,
            payload,
            "Actor unregistration result",
        );
    }
}

/// Broadcasts ServerState and GameState to all clients when they change.
fn broadcast_state_on_change(
    mut server: ResMut<QuinnetServer>,
    server_state: Res<State<ServerState>>,
    game_state: Res<State<GameState>>,
) {

    let Some(endpoint) = server.get_endpoint_mut() else {
        info!("[broadcast_state_on_change] No endpoint available");
        return;
    };

    if server_state.is_changed() {
        info!("[broadcast_state_on_change] ServerState changed, broadcasting ServerStateUpdate: {:?}", server_state.get());
        let msg = NetworkMessage::ServerStateUpdate(server_state.get().clone());
        let payload = msg.to_bytes().expect("Serialize ServerStateResponse");
        let _ = endpoint.broadcast_payload(payload);
    }

    if game_state.is_changed() {
        info!("[broadcast_state_on_change] GameState changed, broadcasting GameStateUpdate: {:?}", game_state.get());
        let msg = NetworkMessage::GameStateUpdate(game_state.get().clone());
        let payload = msg.to_bytes().expect("Serialize GameStateResponse");
        let _ = endpoint.broadcast_payload(payload);
    }
}
