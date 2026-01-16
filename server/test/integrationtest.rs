#[test]
fn test_query_scene_setup() {
    let test_port = TEST_PORT_BASE + 12;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Client sends QuerySceneSetup
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

    // Verify response
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

    // Client 1 sends Update
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

    // Verify server update
    {
        let server_config = server_app.world().resource::<CameraConfiguration>();
        assert_eq!(server_config.locked_to_scene, true);
        assert_eq!(server_config.angle, 123.0);
    }

    // Verify client 2 received broadcast
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

    // Client 1 sends Update
    let mut new_config = SceneConfiguration::default();
    new_config.scene_dimension = UVec2::new(42, 42);
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

    // Verify server update
    {
        let server_config = server_app.world().resource::<SceneConfiguration>();
        assert_eq!(server_config.scene_dimension, UVec2::new(42, 42));
        assert_eq!(server_config.y_difference, 10.0);
    }

    // Verify client 2 received broadcast
    {
        let received = client2_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client 2 should receive broadcast");
        let found = received.0.iter().any(|m| {
            if let NetworkMessage::SceneConfigUpdate(received_config) = m {
                received_config.scene_dimension == UVec2::new(42, 42) && received_config.y_difference == 10.0
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
mod util;
use util::{
    ReceivedMessages, TEST_PORT_BASE, connect_client, create_test_client, create_test_server,
    flush_server_messages,
};

use bevy::prelude::*;
use bevy_quinnet::client::*;
use bevy_quinnet::server::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::NetworkMessage;
use std::time::Duration;

#[test]
fn test_client_sends_message_server_receives() {
    let test_port = TEST_PORT_BASE;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    std::thread::sleep(Duration::from_millis(100));

    // Connect client
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Update apps to process connection
    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    // Client sends a Pong message
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

    // Update to process message
    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);

    // Server receives and verifies the message
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

    // Connect client
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Update apps
    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    // Server sends a Ping message
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

    // Update to process message
    server_app.update();
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Client receives and verifies the message
    {
        let received_messages = client_app.world().resource::<ReceivedMessages>();
        assert!(
            !received_messages.0.is_empty(),
            "Client should have received a message"
        );
        let message = received_messages.0[0].clone();

        match message {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, test_timestamp, "Timestamp should match");
            }
            _ => panic!("Expected Ping message, got {:?}", message),
        }
    }
}

#[test]
fn test_bidirectional_message_exchange() {
    let test_port = TEST_PORT_BASE + 2;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    // Setup
    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    for _ in 0..10 {
        server_app.update();
        client_app.update();
        std::thread::sleep(Duration::from_millis(20));
    }

    // Step 1: Server sends Ping
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

    // Step 2: Client receives Ping and sends Pong
    {
        let client_received = {
            let received_messages = client_app.world().resource::<ReceivedMessages>();
            received_messages
                .0
                .get(0)
                .expect("Client should have received Ping")
                .clone()
        };

        match client_received {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, ping_timestamp);
                // Send Pong back
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

    // Step 3: Server receives Pong
    {
        let received_messages = server_app.world().resource::<ReceivedMessages>();
        let message = received_messages
            .0
            .get(0)
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

    // Client sends multiple messages
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

    // Server receives all messages in order
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

    // Setup server
    std::thread::sleep(Duration::from_millis(100));

    // Connect both clients
    connect_client(&mut client1, test_port);
    connect_client(&mut client2, test_port);
    std::thread::sleep(Duration::from_millis(300));

    for _ in 0..15 {
        server_app.update();
        client1.update();
        client2.update();
        std::thread::sleep(Duration::from_millis(30));
    }

    // Server broadcasts to all clients
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

    // Both clients receive the message
    {
        let received_messages = client1.world().resource::<ReceivedMessages>();
        assert!(!received_messages.0.is_empty());
        match received_messages.0[0] {
            NetworkMessage::Ping { timestamp } => assert_eq!(timestamp, broadcast_timestamp),
            _ => panic!("Expected Ping"),
        }
    }

    {
        let received_messages = client2.world().resource::<ReceivedMessages>();
        assert!(!received_messages.0.is_empty());
        match received_messages.0[0] {
            NetworkMessage::Ping { timestamp } => assert_eq!(timestamp, broadcast_timestamp),
            _ => panic!("Expected Ping"),
        }
    }
}

#[test]
fn test_message_serialization() {
    // Test Ping message
    let ping = NetworkMessage::Ping { timestamp: 12345 };
    let serialized = ping.to_bytes().expect("Should serialize");
    let deserialized: NetworkMessage =
        NetworkMessage::from_bytes(&serialized).expect("Should deserialize");

    match deserialized {
        NetworkMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
        _ => panic!("Wrong message type"),
    }

    // Test Pong message
    let pong = NetworkMessage::Pong { timestamp: 67890 };
    let serialized = pong.to_bytes().expect("Should serialize");
    let deserialized: NetworkMessage =
        NetworkMessage::from_bytes(&serialized).expect("Should deserialize");

    match deserialized {
        NetworkMessage::Pong { timestamp } => assert_eq!(timestamp, 67890),
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_server_starts_successfully() {
    let test_port = TEST_PORT_BASE + 5;
    let server_app = create_test_server(test_port);

    let server = server_app.world().resource::<QuinnetServer>();
    assert!(server.is_listening(), "Server should be listening");
}

#[test]
fn test_query_projector_config() {
    let test_port = TEST_PORT_BASE + 10;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Client sends Query
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
    flush_server_messages(&mut server_app); // Server handles query and sends response

    std::thread::sleep(Duration::from_millis(150));
    client_app.update(); // Client receives response

    // Verify response
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client should receive response");
        // The first message might be a Ping from periodic sender, so look for response
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

    // Client 1 sends Update
    let mut new_config = ProjectorConfiguration::default();
    new_config.enabled = true;
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
    flush_server_messages(&mut server_app); // Server updates and broadcasts

    std::thread::sleep(Duration::from_millis(150));
    client2_app.update(); // Client 2 receives broadcast

    // Verify server update
    {
        let server_config = server_app.world().resource::<ProjectorConfiguration>();
        assert_eq!(server_config.enabled, true);
        assert_eq!(server_config.angle, 45.0);
    }

    // Verify client 2 received broadcast
    {
        let received = client2_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client 2 should receive broadcast");
        let found = received.0.iter().any(|m| {
            if let NetworkMessage::ProjectorConfigUpdate(received_config) = m {
                received_config.enabled == true && received_config.angle == 45.0
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
fn test_init_game_session() {
    let test_port = TEST_PORT_BASE + 15;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Client initiates a game session
    let game_id = 1u16;
    let game_name = "Test Game";
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, game_name.to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(150));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(150));
    client_app.update();

    // Verify client receives GameSessionResponse
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .any(|m| matches!(m, NetworkMessage::GameSessionResponse(_)));
        assert!(found, "Expected GameSessionResponse");
    }
}

#[test]
fn test_start_game_session() {
    let test_port = TEST_PORT_BASE + 16;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 2u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Get game UUID from response
    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse received");
        }
    };

    // Clear messages and start game
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::StartGameSession(game_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify game session is updated
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| {
                if let NetworkMessage::GameSessionResponse(session) = m {
                    session.uuid == game_uuid && session.started
                } else {
                    false
                }
            });
        assert!(found.is_some(), "Expected started GameSession in response");
    }
}

#[test]
fn test_pause_game_session() {
    let test_port = TEST_PORT_BASE + 17;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 3u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Pause game
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::PauseGameSession(game_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify game session is paused
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| {
                if let NetworkMessage::GameSessionResponse(session) = m {
                    session.uuid == game_uuid && session.paused
                } else {
                    false
                }
            });
        assert!(found.is_some(), "Expected paused GameSession");
    }
}

#[test]
fn test_resume_game_session() {
    let test_port = TEST_PORT_BASE + 18;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 4u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Pause then resume
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::PauseGameSession(game_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);

    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::ResumeGameSession(game_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify game session is not paused
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| {
                if let NetworkMessage::GameSessionResponse(session) = m {
                    session.uuid == game_uuid && !session.paused
                } else {
                    false
                }
            });
        assert!(found.is_some(), "Expected resumed GameSession");
    }
}

#[test]
fn test_stop_game_session() {
    let test_port = TEST_PORT_BASE + 19;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 5u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Stop game
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::StopGameSession(game_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);

    // Verify game was despawned by checking it's no longer in active sessions
    {
        let mut game_sessions = server_app.world_mut().query::<&common::game::GameSession>();
        let found = game_sessions.iter(server_app.world()).find(|session| session.uuid == game_uuid);
        assert!(found.is_none(), "Game session should be despawned");
    }
}

#[test]
fn test_register_actor_success() {
    let test_port = TEST_PORT_BASE + 20;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 6u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Register actor
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::RegisterActor(game_uuid, "Player1".to_string(), vec!["Controller".to_string()])
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify ActorResponse received
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| matches!(m, NetworkMessage::ActorResponse(_)));
        assert!(found.is_some(), "Expected ActorResponse");
    }
}

#[test]
fn test_register_actor_game_not_found() {
    let test_port = TEST_PORT_BASE + 21;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Try to register actor for non-existent game
    let fake_game_uuid = bevy::asset::uuid::Uuid::new_v4();
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::RegisterActor(fake_game_uuid, "Player1".to_string(), vec!["Controller".to_string()])
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    flush_server_messages(&mut server_app);  // Extra flush to ensure message is sent over network
    std::thread::sleep(Duration::from_millis(200));
    client_app.update();

    // Verify ActorError received
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| matches!(m, NetworkMessage::ActorError(_)));
        assert!(found.is_some(), "Expected ActorError, but received: {:?}", received.0);
    }
}

#[test]
fn test_unregister_actor_success() {
    let test_port = TEST_PORT_BASE + 22;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 7u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Register actor
    let actor_uuid = {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::RegisterActor(game_uuid, "Player1".to_string(), vec!["Controller".to_string()])
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
        bevy::asset::uuid::Uuid::new_v4()
    };

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Get registered actor UUID
    let registered_actor_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::ActorResponse(meta)) = received.0.iter().find(|m| matches!(m, NetworkMessage::ActorResponse(_))) {
            meta.actors.get(0).map(|a| a.uuid).unwrap_or(actor_uuid)
        } else {
            actor_uuid
        }
    };

    // Unregister actor
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UnregisterActor(game_uuid, registered_actor_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify ActorResponse received (with empty actors list)
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| matches!(m, NetworkMessage::ActorResponse(_)));
        assert!(found.is_some(), "Expected ActorResponse on unregister");
    }
}

#[test]
fn test_unregister_nonexistent_actor() {
    let test_port = TEST_PORT_BASE + 23;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Init game
    let game_id = 8u16;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::InitGameSession(game_id, "Test Game".to_string())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    let game_uuid = {
        let received = client_app.world().resource::<ReceivedMessages>();
        if let Some(NetworkMessage::GameSessionResponse(session)) = received.0.iter().find(|m| matches!(m, NetworkMessage::GameSessionResponse(_))) {
            session.uuid
        } else {
            panic!("No GameSessionResponse");
        }
    };

    // Try to unregister non-existent actor
    let fake_actor_uuid = bevy::asset::uuid::Uuid::new_v4();
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UnregisterActor(game_uuid, fake_actor_uuid)
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app);
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();

    // Verify ActorError received
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        let found = received
            .0
            .iter()
            .find(|m| {
                if let NetworkMessage::ActorError(msg) = m {
                    msg.contains("not found")
                } else {
                    false
                }
            });
        assert!(found.is_some(), "Expected ActorError for non-existent actor");
    }
}