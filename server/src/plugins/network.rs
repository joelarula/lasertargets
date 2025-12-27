use bevy::prelude::*;
use bevy_quinnet::server::{
    EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
    certificate::CertificateRetrievalMode,
};
use bevy_quinnet::shared::ClientId;
use common::network::{NetworkMessage, SERVER_HOST, SERVER_PORT};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::Duration;

#[derive(Resource, Debug, Clone)]
pub struct NetworkingConfiguration {
    pub ip: IpAddr,
    pub port: u16,
}

impl Default for NetworkingConfiguration {
    fn default() -> Self {
        Self {
            ip: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
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
            .add_systems(Update, (handle_server_events, send_ping_periodically));
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
) {
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };
    if let Some(channel_id) = endpoint.default_channel() {
        for client_id in endpoint.clients() {
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

    // Serialize the message to bytes using serde_json
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
