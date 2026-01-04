use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_quinnet::client::certificate::*;
use bevy_quinnet::client::connection::*;
use bevy_quinnet::client::*;
use common::network::NetworkMessage;
use server::plugins::network::{FromClientMessage, NetworkingConfiguration};
use std::net::{IpAddr, Ipv4Addr};

pub const TEST_PORT_BASE: u16 = 7100; // Different from default to avoid conflicts

/// Resource to store messages received during tests.
#[derive(Resource, Debug, Default)]
pub struct ReceivedMessages(pub Vec<NetworkMessage>);

/// System to handle incoming client messages and store them.
pub fn handle_client_messages_system(
    mut client: ResMut<QuinnetClient>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    if let Some(connection) = client.get_connection_mut() {
        if let Some(channel_id) = connection.default_channel() {
            while let Some(bytes) = connection.try_receive_payload(channel_id) {
                match NetworkMessage::from_bytes(&bytes) {
                    Ok(msg) => received_messages.0.push(msg),
                    Err(e) => error!("Failed to deserialize client message: {}", e),
                }
            }
        }
    }
}

/// System to handle incoming server messages and store them.
pub fn handle_server_messages_system(
    mut messages: MessageReader<FromClientMessage>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    for msg in messages.read() {
        received_messages.0.push(msg.message.clone());
    }
}

/// Helper to create a test server app
pub fn create_test_server(port: u16) -> App {
    let mut app = App::new();
    
    // Add minimal plugins
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    
    // Add common server plugins (includes StatesPlugin)
    server::add_common_server_plugins(&mut app);
    
    app.insert_resource(NetworkingConfiguration {
        ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port,
    })
    .insert_resource(ReceivedMessages::default())
    .add_systems(Update, handle_server_messages_system);

    // Trigger startup systems to start the server
    app.update();

    app
}

/// Helper to flush all pending server messages into ReceivedMessages.
/// Needs multiple updates to process the full message chain:
/// 1. NetworkingPlugin reads from network and writes internal messages
/// 2. Game/Actor plugins process internal messages and write responses
/// 3. NetworkingPlugin sends responses back over the network
pub fn flush_server_messages(app: &mut App) {
    app.update();
    app.update();
    app.update();
}

/// Helper to create a test client app
pub fn create_test_client() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(QuinnetClientPlugin::default())
        .insert_resource(ReceivedMessages::default())
        .add_systems(Update, handle_client_messages_system);
    app
}

/// Helper to connect a client to a server on a specific port
pub fn connect_client(client_app: &mut App, port: u16) {
    let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
    client
        .open_connection(ClientConnectionConfiguration {
            addr_config: ClientAddrConfiguration::from_strings(
                format!("127.0.0.1:{}", port).as_str(),
                "0.0.0.0:0",
            )
            .unwrap(),
            cert_mode: CertificateVerificationMode::SkipVerification,
            defaultables: Default::default(),
        })
        .expect("Client should connect");
}
