use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy_quinnet::{
    server::{
        certificate::CertificateRetrievalMode,
        QuinnetServerPlugin, QuinnetServer,
        EndpointAddrConfiguration, ServerEndpointConfiguration,
    },
};
use common::network::{NetworkMessage, SERVER_HOST, SERVER_PORT};
use std::time::Duration;
use std::net::Ipv6Addr;

fn main() {
    App::new()
        // Use MinimalPlugins for headless server (no rendering, no input, no windowing)
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0), // 60 FPS
        )))
        .add_plugins(LogPlugin::default())
        // Add Quinnet server plugin for networking
        .add_plugins(QuinnetServerPlugin::default())
        // Add our server systems
        .add_systems(Startup, start_server)
        .add_systems(Update, (handle_server_events, send_ping_periodically))
        .run();
}

/// Start the Quinnet server on startup
fn start_server(mut server: ResMut<QuinnetServer>) {
    match server.start_endpoint(
        ServerEndpointConfiguration {
            addr_config: EndpointAddrConfiguration::from_ip(
                Ipv6Addr::UNSPECIFIED, 
                SERVER_PORT
            ),
            cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "localhost".to_string(),
            },
            defaultables: Default::default(),
        }
    ) {
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
fn handle_server_events(mut server: ResMut<QuinnetServer>) {
    let Some(endpoint) = server.get_endpoint_mut() else { return; };
        if let Some(channel_id) = endpoint.default_channel() {
            for client_id in endpoint.clients() {
                while let Some(bytes) = endpoint.try_receive_payload(client_id, channel_id) {
                    match NetworkMessage::from_bytes(&bytes) {
                        Ok(message) => {
                            info!("Received message from client {}: {:?}", client_id, message);
                            match message {
                                NetworkMessage::Pong { timestamp } => {
                                    info!("Received pong from client {} at timestamp {}", client_id, timestamp);
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize message from client {}: {}", client_id, e);
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