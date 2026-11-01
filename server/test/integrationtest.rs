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

    // Verify server update
    {
        let server_config = server_app.world().resource::<SceneConfiguration>();
        assert_eq!(server_config.scene_dimension, Vec2::new(42.0, 42.0));
        assert_eq!(server_config.y_difference, 10.0);
    }

    // Verify client 2 received broadcast
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
mod util;
use bevy::asset::uuid::Uuid;
use common::session::{FinishSessionEvent, PauseSessionEvent, ResumeSessionEvent, Session};
use common::state::{ServerState, SessionState};
use util::{
    ReceivedMessages, TEST_PORT_BASE, client_send, connect_client, create_test_client,
    create_test_game_server, create_test_server, flush_server_messages,
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
                .iter()
                .find(|m| matches!(m, NetworkMessage::Ping { timestamp } if *timestamp == ping_timestamp))
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
    flush_server_messages(&mut server_app); // Server updates and broadcasts

    std::thread::sleep(Duration::from_millis(150));
    client2_app.update(); // Client 2 receives broadcast

    // Verify server update
    {
        let server_config = server_app.world().resource::<ProjectorConfiguration>();
        assert_eq!(server_config.switched_on, true);
        assert_eq!(server_config.angle, 45.0);
    }

    // Verify client 2 received broadcast
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
// --- session channel ---------------------------------------------------------
//
// Session lifecycle logic is covered in-process by server-core/tests/session_menu_test.rs.
// These tests only check that the lifecycle crosses the network: OpenSession/CloseSession
// from a client, SessionCreated/SessionUpdate/ServerStateUpdate back to clients.

/// Step server and clients until every client has received a message matching `predicate`.
fn pump_until(
    server_app: &mut App,
    client_apps: &mut [&mut App],
    predicate: impl Fn(&NetworkMessage) -> bool,
) {
    for _ in 0..50 {
        server_app.update();
        for client_app in client_apps.iter_mut() {
            client_app.update();
        }
        let all_received = client_apps.iter().all(|c| {
            c.world().resource::<ReceivedMessages>().0.iter().any(&predicate)
        });
        if all_received {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("expected message never reached every client");
}

/// Connect a client and step until the server has accepted it.
fn connect_and_wait(server_app: &mut App, client_app: &mut App, port: u16) {
    let before = server_app.world().resource::<QuinnetServer>().endpoint().clients().len();
    connect_client(client_app, port);
    for _ in 0..50 {
        server_app.update();
        client_app.update();
        if server_app.world().resource::<QuinnetServer>().endpoint().clients().len() > before {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("client never connected");
}

/// Connect a client to a game server and open a Hunter session; returns its id.
fn open_hunter_session(server_app: &mut App, client_app: &mut App, port: u16) -> Uuid {
    connect_and_wait(server_app, client_app, port);

    let session_id = Uuid::new_v4();
    client_send(
        client_app,
        NetworkMessage::OpenSession {
            session_id,
            channel_name: hunter::common::GAME_NAME.to_string(),
        },
    );
    pump_until(server_app, &mut [client_app], |m| {
        matches!(m, NetworkMessage::SessionCreated(s) if s.session_id == session_id)
    });
    // The spawned Session also goes out as a SessionUpdate; wait so later updates are in order
    pump_until(server_app, &mut [client_app], |m| {
        matches!(m, NetworkMessage::SessionUpdate(s) if s.session_id == session_id)
    });
    session_id
}

fn clear_received(client_app: &mut App) {
    client_app.world_mut().resource_mut::<ReceivedMessages>().0.clear();
}

fn session_states(client_app: &App, session_id: Uuid) -> Vec<SessionState> {
    client_app
        .world()
        .resource::<ReceivedMessages>()
        .0
        .iter()
        .filter_map(|m| match m {
            NetworkMessage::SessionUpdate(s) if s.session_id == session_id => Some(s.state.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn test_open_session_creates_session_and_notifies_client() {
    let test_port = TEST_PORT_BASE + 15;
    let mut server_app = create_test_game_server(test_port);
    let mut client_app = create_test_client();

    let session_id = open_hunter_session(&mut server_app, &mut client_app, test_port);

    let created = client_app
        .world()
        .resource::<ReceivedMessages>()
        .0
        .iter()
        .find_map(|m| match m {
            NetworkMessage::SessionCreated(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(created.session_id, session_id);
    assert_eq!(created.game_id, hunter::common::GAME_ID);
    assert!(created.start_timestamp.is_some());

    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::ServerStateUpdate(ServerState::Session))
    });

    let mut sessions = server_app.world_mut().query::<&Session>();
    let ids: Vec<Uuid> = sessions.iter(server_app.world()).map(|s| s.session_id).collect();
    assert_eq!(ids, vec![session_id]);
}

#[test]
fn test_session_created_is_broadcast_to_all_clients() {
    let test_port = TEST_PORT_BASE + 16;
    let mut server_app = create_test_game_server(test_port);
    let mut client1_app = create_test_client();
    let mut client2_app = create_test_client();

    connect_and_wait(&mut server_app, &mut client2_app, test_port);

    let session_id = open_hunter_session(&mut server_app, &mut client1_app, test_port);

    pump_until(&mut server_app, &mut [&mut client2_app], |m| {
        matches!(m, NetworkMessage::SessionCreated(s) if s.session_id == session_id)
    });
}

#[test]
fn test_session_lifecycle_updates_reach_client() {
    let test_port = TEST_PORT_BASE + 17;
    let mut server_app = create_test_game_server(test_port);
    let mut client_app = create_test_client();

    let session_id = open_hunter_session(&mut server_app, &mut client_app, test_port);

    clear_received(&mut client_app);

    // Pause/resume/finish have no client message; they are raised server-side by games and the menu
    server_app.world_mut().write_message(PauseSessionEvent { session_id });
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::SessionUpdate(s) if s.state == SessionState::Paused)
    });

    server_app.world_mut().write_message(ResumeSessionEvent { session_id });
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::SessionUpdate(s) if s.state == SessionState::Active)
    });

    server_app.world_mut().write_message(FinishSessionEvent { session_id });
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::SessionUpdate(s) if s.state == SessionState::Finished)
    });

    // Each transition is sent exactly once
    let states = session_states(&client_app, session_id);
    assert_eq!(
        states,
        vec![SessionState::Paused, SessionState::Active, SessionState::Finished]
    );
}

#[test]
fn test_close_session_finishes_session_and_returns_to_menu() {
    let test_port = TEST_PORT_BASE + 18;
    let mut server_app = create_test_game_server(test_port);
    let mut client_app = create_test_client();

    let session_id = open_hunter_session(&mut server_app, &mut client_app, test_port);
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::ServerStateUpdate(ServerState::Session))
    });

    clear_received(&mut client_app);

    client_send(&mut client_app, NetworkMessage::CloseSession { session_id });
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::SessionUpdate(s) if s.session_id == session_id && s.state == SessionState::Finished)
    });
    pump_until(&mut server_app, &mut [&mut client_app], |m| {
        matches!(m, NetworkMessage::ServerStateUpdate(ServerState::Menu))
    });

    let mut sessions = server_app.world_mut().query::<&Session>();
    assert_eq!(sessions.iter(server_app.world()).count(), 0, "session entity should be despawned");
}

#[test]
fn test_curator_menu_workflow_lifecycle() {
    use bevy::app::ScheduleRunnerPlugin;
    use common::state::ServerState;

    let mut server_app = App::new();
    server_app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    server::add_common_server_plugins(&mut server_app);
    util::disable_dac(&mut server_app);
    server_app.add_plugins(hunter::common::HunterGamePlugin)
        .add_plugins(snake::common::SnakeGamePlugin);

    // Initial update enters ServerState::Menu
    server_app.update();

    // Verify ActiveMenuPrompt is populated
    let prompt_id = {
        let active_prompt = server_app.world().resource::<server::plugins::menu::ActiveMenuPrompt>();
        assert!(active_prompt.prompt_id.is_some(), "Menu prompt should be active");
        active_prompt.prompt_id.unwrap()
    };

    // Verify ServerMenuWorkflowMarker entity is spawned
    {
        let mut query = server_app.world_mut().query_filtered::<Entity, With<server::plugins::menu::ServerMenuWorkflowMarker>>();
        assert_eq!(query.iter(server_app.world()).count(), 1, "Should have 1 menu workflow instance");
    }

    // Verify AuditLog recorded menu open event
    {
        let audit = server_app.world().resource::<curator_core::AuditLog>();
        assert!(audit.len() > 0, "AuditLog should contain events");
    }

    // Simulate actor responding to the prompt choosing "hunter" (game ID 1)
    {
        let mut response_writer = server_app.world_mut().resource_mut::<bevy::ecs::message::Messages<curator_core::ActorPromptResponseEvent>>();
        response_writer.write(curator_core::ActorPromptResponseEvent {
            response: curator_core::ActorPromptResponse {
                prompt_id,
                actor_id: None,
                selected_choice_id: "101".to_string(),
                custom_payload: None,
            },
        });
    }

    // Run updates to process prompt response, init session, and state transitions
    for _ in 0..4 {
        server_app.update();
    }

    // Verify ServerState transitioned to InGame and GameSession was spawned
    {
        let server_state = server_app.world().resource::<State<ServerState>>();
        assert_eq!(*server_state.get(), ServerState::Session, "ServerState should be InGame after selection");

        let mut session_query = server_app.world_mut().query::<&common::game::GameSession>();
        let session = session_query.iter(server_app.world()).next();
        assert!(session.is_some(), "GameSession should be spawned");
        assert_eq!(session.unwrap().game_id, 101, "Game ID should match Hunter (101)");
    }

    // Verify ServerMenuWorkflowMarker was cleaned up on exiting Menu
    {
        let mut query = server_app.world_mut().query_filtered::<Entity, With<server::plugins::menu::ServerMenuWorkflowMarker>>();
        assert_eq!(query.iter(server_app.world()).count(), 0, "Menu workflow should be cleaned up on exiting Menu");
    }
}


#[test]
fn test_curator_menu_network_prompt_selection() {
    use bevy::app::ScheduleRunnerPlugin;
    use common::state::ServerState;

    let mut server_app = App::new();
    server_app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    server::add_common_server_plugins(&mut server_app);
    util::disable_dac(&mut server_app);
    server_app.add_plugins(hunter::common::HunterGamePlugin)
        .add_plugins(snake::common::SnakeGamePlugin);

    // Initial update enters ServerState::Menu and spawns workflow
    server_app.update();

    let prompt_id = {
        let active_prompt = server_app.world().resource::<server::plugins::menu::ActiveMenuPrompt>();
        active_prompt.prompt_id.expect("Active prompt")
    };

    // Simulate client sending NetworkMessage::CuratorPromptResponse
    {
        let mut msg_writer = server_app.world_mut().resource_mut::<bevy::ecs::message::Messages<server::plugins::network::FromClientMessage>>();
        msg_writer.write(server::plugins::network::FromClientMessage {
            client_id: 42,
            message: NetworkMessage::CuratorPromptResponse(curator_core::ActorPromptResponse {
                prompt_id,
                actor_id: None,
                selected_choice_id: "101".to_string(),
                custom_payload: None,
            }),
        });
    }

    // Process network message -> prompt response event -> menu handler -> session init -> state transition
    for _ in 0..4 {
        server_app.update();
    }

    // Verify ServerState transitioned to InGame and Hunter session created
    let server_state = server_app.world().resource::<State<ServerState>>();
    assert_eq!(*server_state.get(), ServerState::Session);

    let mut session_query = server_app.world_mut().query::<&common::game::GameSession>();
    let session = session_query.iter(server_app.world()).next().expect("Game session");
    assert_eq!(session.game_id, 101);
}
