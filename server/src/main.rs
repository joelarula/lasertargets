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
    let Some(endpoint) = server.get_endpoint_mut() else {
        return;
    };

    // Handle incoming messages from all clients on the default channel
    for client_id in endpoint.clients() {
        while let Some(message) = endpoint.try_receive_message::<NetworkMessage>(client_id) {
            info!("Received message from client {}: {:?}", client_id, message);
            
            match message {
                NetworkMessage::Pong { timestamp } => {
                    info!("Received pong from client {} at timestamp {}", client_id, timestamp);
                }
                _ => {}
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

    // Broadcast ping to all connected clients
    for client_id in endpoint.clients() {
        match endpoint.send_message(client_id, message.clone()) {
            Ok(_) => {
                info!("Sent ping to client {} at timestamp {}", client_id, timestamp);
            }
            Err(e) => {
                error!("Failed to send ping to client {}: {}", client_id, e);
            }
        }
    }
}
