use bevy::prelude::*;
use bevy_quinnet::server::{
    certificate::CertificateRetrievalMode, EndpointAddrConfiguration, QuinnetServer,
    ServerEndpointConfiguration,
};
use bevy_quinnet::server::endpoint::Endpoint;
use bevy_quinnet::shared::ClientId;
use common::actor::ActorMetaData;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::{NetworkMessage, SERVER_HOST, SERVER_PORT};
use common::scene::SceneSetup;
use std::net::{IpAddr, Ipv6Addr};

use crate::plugins::actor::ActorRegistry;

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

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FromClientMessage>()
            .init_resource::<NetworkingConfiguration>()
            .add_systems(Startup, start_server)
            .add_systems(Update, (handle_server_events, send_ping_periodically))
            .add_systems(Update, broadcast_scene_setup_on_change);
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

/// Handle incoming client connections and messages
fn handle_server_events(
    mut server: ResMut<QuinnetServer>,
    mut message_writer: MessageWriter<FromClientMessage>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut camera_config: ResMut<CameraConfiguration>,
    mut scene_config: ResMut<SceneConfiguration>,
    mut scene_setup: ResMut<SceneSetup>,
    mut registry: ResMut<ActorRegistry>,
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
                            message: message.clone(),
                        });
                        match message {
                            NetworkMessage::Pong { timestamp } => {
                                info!(
                                    "Received pong from client {} at timestamp {}",
                                    client_id, timestamp
                                );
                            }
                            // Query handlers
                            NetworkMessage::QueryProjectorConfig => {
                                let message = NetworkMessage::ProjectorConfigResponse(projector_config.clone());
                                let payload = message.to_bytes().expect("Serialize ProjectorConfigResponse");
                                send_payload_and_log_error(
                                    endpoint,
                                    client_id,
                                    payload,
                                    "ProjectorConfigResponse",
                                );
                            }
                            NetworkMessage::QueryCameraConfig => {
                                let message = NetworkMessage::CameraConfigResponse(camera_config.clone());
                                let payload = message.to_bytes().expect("Serialize CameraConfigResponse");
                                send_payload_and_log_error(
                                    endpoint,
                                    client_id,
                                    payload,
                                    "CameraConfigResponse",
                                );
                            }
                            NetworkMessage::QuerySceneConfig => {
                                let message = NetworkMessage::SceneConfigResponse(scene_config.clone());
                                let payload = message.to_bytes().expect("Serialize SceneConfigResponse");
                                send_payload_and_log_error(
                                    endpoint,
                                    client_id,
                                    payload,
                                    "SceneConfigResponse",
                                );
                            }

                            NetworkMessage::QuerySceneSetup => {
                                let message = NetworkMessage::SceneSetupResponse(scene_setup.clone());
                                let payload = message.to_bytes().expect("Serialize SceneSetupResponse");
                                send_payload_and_log_error(
                                    endpoint,
                                    client_id,
                                    payload,
                                    "SceneSetupResponse",
                                );
                            }
                        
                            NetworkMessage::UpdateProjectorConfig(new_config) => {
                                *projector_config = new_config.clone();
                                info!("Projector configuration updated by client {}", client_id);
                                let payload = NetworkMessage::UpdateProjectorConfig(new_config)
                                    .to_bytes()
                                    .expect("Serialize");
                                broadcast_payload_and_log_error(
                                    endpoint,
                                    payload,
                                    "UpdateProjectorConfig",
                                );
                            }
                            NetworkMessage::UpdateCameraConfig(new_config) => {
                                *camera_config = new_config.clone();
                                info!("Camera configuration updated by client {}", client_id);
                                let payload = NetworkMessage::UpdateCameraConfig(new_config)
                                    .to_bytes()
                                    .expect("Serialize");
                                broadcast_payload_and_log_error(
                                    endpoint,
                                    payload,
                                    "UpdateCameraConfig",
                                );
                            }
                            NetworkMessage::UpdateSceneConfig(new_config) => {
                                *scene_config = new_config.clone();
                                info!("Scene configuration updated by client {}", client_id);
                                let payload = NetworkMessage::UpdateSceneConfig(new_config)
                                    .to_bytes()
                                    .expect("Serialize");
                                broadcast_payload_and_log_error(
                                    endpoint,
                                    payload,
                                    "UpdateSceneConfig",
                                );
                            }
                            NetworkMessage::QueryActor => {
                  
                                let actors = registry.get_actors(client_id).to_vec();
                                let meta = ActorMetaData { actors };
                                let payload = NetworkMessage::ActorResponse(meta)
                                    .to_bytes()
                                    .expect("Serialize");
                                send_payload_and_log_error(
                                    endpoint,
                                    client_id,
                                    payload,
                                    "ActorResponse",
                                );
                            }

                            NetworkMessage::RegisterActor(meta) => {
                                for actor in meta.actors {
                                    registry.register_actor(client_id, actor);
                                }
                            }

                            NetworkMessage::UnregisterActor(meta) => {
                                for actor in meta.actors {
                                    registry.unregister_actor(client_id, actor.uuid);
                                }
                            }
                            _ => {}
                        }
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

/// Send periodic ping messages to all connected clients
fn send_ping_periodically(
    mut server: ResMut<QuinnetServer>,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
) {
    // Initialize timer on first run
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(2.0, TimerMode::Repeating));
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
            info!("Sent ping to all clients at timestamp {}", timestamp);
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
        info!("CameraConfiguration changed, broadcasting update.");
        let message = NetworkMessage::UpdateCameraConfig(camera_config.clone());
        let payload = message.to_bytes().expect("Serialize CameraConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast CameraConfiguration update: {}", e);
        }
    }

    // Check and broadcast ProjectorConfiguration changes
    if projector_config.is_changed() {
        info!("ProjectorConfiguration changed, broadcasting update.");
        let message = NetworkMessage::UpdateProjectorConfig(projector_config.clone());
        let payload = message.to_bytes().expect("Serialize ProjectorConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast ProjectorConfiguration update: {}", e);
        }
    }

    // Check and broadcast SceneConfiguration changes
    if scene_configuration.is_changed() {
        info!("SceneConfiguration changed, broadcasting update.");
        let message = NetworkMessage::UpdateSceneConfig(scene_configuration.clone());
        let payload = message.to_bytes().expect("Serialize SceneConfig");
        if let Err(e) = endpoint.broadcast_payload(payload) {
            error!("Failed to broadcast SceneConfiguration update: {}", e);
        }
    }

    // Check and broadcast SceneSetup changes (if still desired, as it aggregates the above)
    if scene_setup.is_changed() {
        info!("SceneSetup changed, broadcasting update.");
        let message = NetworkMessage::SceneSetupResponse(scene_setup.clone());
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
    if let Err(e) = endpoint.send_payload(client_id, payload) {
        error!("Failed to send {} to client {}: {}", error_context, client_id, e);
    }
}

/// Helper function to broadcast a payload and log an error if it fails.
fn broadcast_payload_and_log_error(
    endpoint: &mut Endpoint,
    payload: Vec<u8>,
    error_context: &str,
) {
    if let Err(e) = endpoint.broadcast_payload(payload) {
        error!("Failed to broadcast {}: {}", error_context, e);
    }
}
