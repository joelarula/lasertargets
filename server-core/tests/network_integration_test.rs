use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy_quinnet::client::certificate::*;
use bevy_quinnet::client::connection::*;
use bevy_quinnet::client::*;
use bevy_quinnet::server::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::NetworkMessage;
use server_core::add_common_server_plugins;
use server_core::plugins::network::{FromClientMessage, NetworkingConfiguration};
use server_core::plugins::projector::ProjectorDacController;

pub const TEST_PORT_BASE: u16 = 7200;

#[derive(Resource, Debug, Default)]
pub struct ReceivedMessages(pub Vec<NetworkMessage>);

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

pub fn handle_server_messages_system(
    mut messages: MessageReader<FromClientMessage>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    for msg in messages.read() {
        received_messages.0.push(msg.message.clone());
    }
}

pub fn disable_dac(app: &mut App) {
    app.insert_resource(ProjectorDacController {
        enabled: false,
        ..Default::default()
    });
}

pub fn create_test_server(port: u16) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    add_common_server_plugins(&mut app);
    disable_dac(&mut app);

    app.insert_resource(NetworkingConfiguration {
        ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port,
    })
    .insert_resource(ReceivedMessages::default())
    .add_systems(Update, handle_server_messages_system);

    app.update();
    app
}

pub fn create_test_client() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(QuinnetClientPlugin::default())
        .insert_resource(ReceivedMessages::default())
        .add_systems(Update, handle_client_messages_system);
    app
}

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

pub fn flush_server_messages(app: &mut App) {
    app.update();
    app.update();
    app.update();
}

#[test]
fn test_server_starts_successfully() {
    let test_port = TEST_PORT_BASE + 5;
    let server_app = create_test_server(test_port);

    let server = server_app.world().resource::<QuinnetServer>();
    assert!(server.is_listening(), "Server should be listening");
}

#[test]
fn test_client_sends_message_server_receives() {
    let test_port = TEST_PORT_BASE;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    let test_timestamp = 42424242u64;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client
            .get_connection_mut()
            .expect("Client should be connected");
        let payload = NetworkMessage::Pong {
            timestamp: test_timestamp,
        }
        .to_bytes()
        .expect("Should serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);

    {
        let received_messages = server_app.world().resource::<ReceivedMessages>();
        assert_eq!(
            received_messages.0.len(),
            1,
            "Server should receive exactly one message"
        );
        let message = received_messages.0[0].clone();

        match message {
            NetworkMessage::Pong { timestamp } => {
                assert_eq!(timestamp, test_timestamp, "Timestamp should match");
            }
            _ => panic!("Expected Pong message, got {:?}", message),
        }
    }
}

#[test]
fn test_server_sends_message_client_receives() {
    let test_port = TEST_PORT_BASE + 1;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    let test_timestamp = 99999999u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server
            .get_endpoint_mut()
            .expect("Server should have endpoint");

        let clients = endpoint.clients();
        assert!(!clients.is_empty(), "Should have connected clients");

        let payload = NetworkMessage::Ping {
            timestamp: test_timestamp,
        }
        .to_bytes()
        .expect("Should serialize");
        for client_id in clients {
            endpoint.send_payload(client_id, payload.clone()).unwrap();
        }
    }

    server_app.update();
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    {
        let received_messages = client_app.world().resource::<ReceivedMessages>();
        assert!(
            !received_messages.0.is_empty(),
            "Client should have received a message"
        );
        assert!(
            received_messages.0.iter().any(|m| matches!(m, NetworkMessage::Ping { timestamp } if *timestamp == test_timestamp)),
            "Expected Ping with timestamp {} in {:?}",
            test_timestamp,
            received_messages.0
        );
    }
}

#[test]
fn test_bidirectional_message_exchange() {
    let test_port = TEST_PORT_BASE + 2;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    let ping_timestamp = 111111u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let client_id = endpoint.clients()[0];
        let payload = NetworkMessage::Ping {
            timestamp: ping_timestamp,
        }
        .to_bytes()
        .expect("Should serialize");
        endpoint.send_payload(client_id, payload).unwrap();
    }

    server_app.update();
    std::thread::sleep(Duration::from_millis(50));
    client_app.update();

    {
        let client_received = {
            let received_messages = client_app.world().resource::<ReceivedMessages>();
            received_messages
                .0
                .iter()
                .find(|m| matches!(m, NetworkMessage::Ping { timestamp } if *timestamp == ping_timestamp))
                .expect("Client should have received Ping")
                .clone()
        };

        match client_received {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, ping_timestamp);
                let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
                let connection = client.get_connection_mut().unwrap();
                let payload = NetworkMessage::Pong { timestamp }
                    .to_bytes()
                    .expect("Should serialize");
                connection.send_payload(payload).unwrap();
            }
            _ => panic!("Expected Ping"),
        }
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(50));
    flush_server_messages(&mut server_app);

    {
        let received_messages = server_app.world().resource::<ReceivedMessages>();
        let message = received_messages
            .0
            .iter()
            .find(|m| matches!(m, NetworkMessage::Pong { .. }))
            .expect("Server should have received Pong")
            .clone();

        match message {
            NetworkMessage::Pong { timestamp } => {
                assert_eq!(timestamp, ping_timestamp, "Pong should echo Ping timestamp");
            }
            _ => panic!("Expected Pong"),
        }
    }
}

#[test]
fn test_multiple_messages_in_sequence() {
    let test_port = TEST_PORT_BASE + 3;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    let timestamps = vec![1000u64, 2000u64, 3000u64, 4000u64];
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();

        for &timestamp in &timestamps {
            let payload = NetworkMessage::Pong { timestamp }
                .to_bytes()
                .expect("Should serialize");
            connection.send_payload(payload).unwrap();
        }
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);

    {
        let received_messages = server_app.world().resource::<ReceivedMessages>();
        assert_eq!(
            received_messages.0.len(),
            timestamps.len(),
            "Should receive all messages"
        );

        for (i, &timestamp) in timestamps.iter().enumerate() {
            match received_messages.0[i] {
                NetworkMessage::Pong {
                    timestamp: received_ts,
                } => {
                    assert_eq!(received_ts, timestamp);
                }
                _ => panic!("Expected Pong messages"),
            }
        }
    }
}

#[test]
fn test_server_broadcasts_to_multiple_clients() {
    let test_port = TEST_PORT_BASE + 4;
    let mut server_app = create_test_server(test_port);
    let mut client1 = create_test_client();
    let mut client2 = create_test_client();

    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client1, test_port);
    connect_client(&mut client2, test_port);
    std::thread::sleep(Duration::from_millis(300));

    for _ in 0..15 {
        server_app.update();
        client1.update();
        client2.update();
        std::thread::sleep(Duration::from_millis(30));
    }

    let broadcast_timestamp = 777777u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let clients = endpoint.clients();

        assert_eq!(clients.len(), 2, "Should have 2 connected clients");

        let payload = NetworkMessage::Ping {
            timestamp: broadcast_timestamp,
        }
        .to_bytes()
        .expect("Should serialize");
        for client_id in clients {
            endpoint.send_payload(client_id, payload.clone()).unwrap();
        }
    }

    server_app.update();
    std::thread::sleep(Duration::from_millis(100));
    client1.update();
    client2.update();

    {
        let received_messages = client1.world().resource::<ReceivedMessages>();
        assert!(
            received_messages.0.iter().any(|m| matches!(m, NetworkMessage::Ping { timestamp } if *timestamp == broadcast_timestamp)),
            "Expected broadcast Ping in {:?}",
            received_messages.0
        );
    }

    {
        let received_messages = client2.world().resource::<ReceivedMessages>();
        assert!(
            received_messages.0.iter().any(|m| matches!(m, NetworkMessage::Ping { timestamp } if *timestamp == broadcast_timestamp)),
            "Expected broadcast Ping in {:?}",
            received_messages.0
        );
    }
}

#[test]
fn test_query_projector_config() {
    let test_port = TEST_PORT_BASE + 10;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::QueryProjectorConfig
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);

    std::thread::sleep(Duration::from_millis(150));
    client_app.update();

    {
        let received = client_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client should receive response");
        let found = received
            .0
            .iter()
            .any(|m| matches!(m, NetworkMessage::ProjectorConfigUpdate(_)));
        assert!(
            found,
            "Expected ProjectorConfigResponse in {:?}",
            received.0
        );
    }
}

#[test]
fn test_update_config_broadcast() {
    let test_port = TEST_PORT_BASE + 11;
    let mut server_app = create_test_server(test_port);
    let mut client1_app = create_test_client();
    let mut client2_app = create_test_client();

    connect_client(&mut client1_app, test_port);
    connect_client(&mut client2_app, test_port);
    std::thread::sleep(Duration::from_millis(300));

    let mut new_config = ProjectorConfiguration::default();
    new_config.switched_on = true;
    new_config.angle = 45.0;

    {
        let mut client = client1_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UpdateProjectorConfig(new_config.clone())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client1_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);

    std::thread::sleep(Duration::from_millis(150));
    client2_app.update();

    {
        let server_config = server_app.world().resource::<ProjectorConfiguration>();
        assert_eq!(server_config.switched_on, true);
        assert_eq!(server_config.angle, 45.0);
    }

    {
        let received = client2_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client 2 should receive broadcast");
        let found = received.0.iter().any(|m| {
            if let NetworkMessage::ProjectorConfigUpdate(received_config) = m {
                received_config.switched_on == true && received_config.angle == 45.0
            } else {
                false
            }
        });
        assert!(
            found,
            "Expected ProjectorConfigUpdate broadcast in {:?}",
            received.0
        );
    }
}

#[test]
fn test_query_scene_setup() {
    let test_port = TEST_PORT_BASE + 12;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::QuerySceneSetup
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(150));
    client_app.update();

    {
        let received = client_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client should receive response");
        let found = received
            .0
            .iter()
            .any(|m| matches!(m, NetworkMessage::SceneSetupUpdate(_)));
        assert!(
            found,
            "Expected SceneSetupResponse in {:?}",
            received.0
        );
    }
}

#[test]
fn test_update_camera_config_broadcast() {
    let test_port = TEST_PORT_BASE + 13;
    let mut server_app = create_test_server(test_port);
    let mut client1_app = create_test_client();
    let mut client2_app = create_test_client();

    connect_client(&mut client1_app, test_port);
    connect_client(&mut client2_app, test_port);
    std::thread::sleep(Duration::from_millis(300));

    let mut new_config = CameraConfiguration::default();
    new_config.locked_to_scene = true;
    new_config.angle = 123.0;

    {
        let mut client = client1_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UpdateCameraConfig(new_config.clone())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client1_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(150));
    client2_app.update();

    {
        let server_config = server_app.world().resource::<CameraConfiguration>();
        assert_eq!(server_config.locked_to_scene, true);
        assert_eq!(server_config.angle, 123.0);
    }

    {
        let received = client2_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client 2 should receive broadcast");
        let found = received.0.iter().any(|m| {
            if let NetworkMessage::CameraConfigUpdate(received_config) = m {
                received_config.locked_to_scene == true && received_config.angle == 123.0
            } else {
                false
            }
        });
        assert!(
            found,
            "Expected CameraConfigUpdate broadcast in {:?}",
            received.0
        );
    }
}

#[test]
fn test_update_scene_config_broadcast() {
    let test_port = TEST_PORT_BASE + 14;
    let mut server_app = create_test_server(test_port);
    let mut client1_app = create_test_client();
    let mut client2_app = create_test_client();

    connect_client(&mut client1_app, test_port);
    connect_client(&mut client2_app, test_port);
    std::thread::sleep(Duration::from_millis(300));

    let mut new_config = SceneConfiguration::default();
    new_config.scene_dimension = Vec2::new(42.0, 42.0);
    new_config.y_difference = 10.0;

    {
        let mut client = client1_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UpdateSceneConfig(new_config.clone())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client1_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(150));
    client2_app.update();

    {
        let server_config = server_app.world().resource::<SceneConfiguration>();
        assert_eq!(server_config.scene_dimension, Vec2::new(42.0, 42.0));
        assert_eq!(server_config.y_difference, 10.0);
    }

    {
        let received = client2_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client 2 should receive broadcast");
        let found = received.0.iter().any(|m| {
            if let NetworkMessage::SceneConfigUpdate(received_config) = m {
                received_config.scene_dimension == Vec2::new(42.0, 42.0) && received_config.y_difference == 10.0
            } else {
                false
            }
        });
        assert!(
            found,
            "Expected SceneConfigUpdate broadcast in {:?}",
            received.0
        );
    }
}
