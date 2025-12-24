use bevy::prelude::*;
use bevy_quinnet::client::{
    QuinnetClientPlugin, QuinnetClient,
    certificate::CertificateVerificationMode,
    connection::ClientAddrConfiguration,
    ClientConnectionConfiguration,
};
use common::network::{NetworkMessage, SERVER_PORT};

/// Plugin that handles networking with the server
pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .add_systems(Startup, connect_to_server)
            .add_systems(Update, handle_server_messages);
    }
}

/// Resource to store server connection info
#[derive(Resource)]
struct ServerAddress(String);

impl Default for ServerAddress {
    fn default() -> Self {
        Self("127.0.0.1".to_string())
    }
}

/// Connect to the server on startup
fn connect_to_server(mut client: ResMut<QuinnetClient>) {
    let server_addr = "127.0.0.1"; // Default to localhost
    
    match client.open_connection(
        ClientConnectionConfiguration {
            addr_config: ClientAddrConfiguration::from_strings(
                format!("{}:{}", server_addr, SERVER_PORT).as_str(),
                "0.0.0.0:0"
            ).unwrap(),
            cert_mode: CertificateVerificationMode::SkipVerification,
            defaultables: Default::default(),
        },
    ) {
        Ok(_) => {
            info!("Connecting to server at {}:{}", server_addr, SERVER_PORT);
        }
        Err(e) => {
            error!("Failed to connect to server: {}", e);
        }
    }
}

/// Handle incoming messages from the server
fn handle_server_messages(mut client: ResMut<QuinnetClient>) {
    // Check connection status
    if let Some(connection) = client.get_connection_mut() {
        // Handle incoming messages
        while let Some(message) = connection.try_receive_message::<NetworkMessage>() {
            match message {
                NetworkMessage::Ping { timestamp } => {
                    info!("Received ping from server at timestamp {}", timestamp);
                    
                    // Send pong response
                    let pong = NetworkMessage::Pong { timestamp };
                    if let Err(e) = connection.send_message(pong) {
                        error!("Failed to send pong: {}", e);
                    } else {
                        info!("Sent pong response");
                    }
                }
                NetworkMessage::Pong { timestamp } => {
                    info!("Received pong at timestamp {}", timestamp);
                }
            }
        }
    }
}
