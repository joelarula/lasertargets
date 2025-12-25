use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use bevy_quinnet::{
    server::{
        QuinnetServerPlugin, QuinnetServer,
        certificate::CertificateRetrievalMode,
        EndpointAddrConfiguration, ServerEndpointConfiguration,
    },
    client::{
        QuinnetClientPlugin, QuinnetClient,
        certificate::CertificateVerificationMode,
        ClientAddrConfiguration, ClientConnectionConfiguration,
    },
};
use common::network::NetworkMessage;
use std::time::Duration;
use std::net::Ipv6Addr;

const TEST_PORT_BASE: u16 = 6100; // Different from default to avoid conflicts

/// Helper to create a test server app
fn create_test_server() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(QuinnetServerPlugin::default());
    app
}

/// Helper to create a test client app
fn create_test_client() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(QuinnetClientPlugin::default());
    app
}

/// Helper to start server on a specific port
fn start_server(server_app: &mut App, port: u16) {
    let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
    server.start_endpoint(
        ServerEndpointConfiguration {
            addr_config: EndpointAddrConfiguration::from_ip(
                Ipv6Addr::LOCALHOST,
                port
            ),
            cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "localhost".to_string(),
            },
            defaultables: Default::default(),
        }
    ).expect("Server should start");
}

/// Helper to connect client to server
fn connect_client(client_app: &mut App, port: u16) {
    let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
    client.open_connection(
        ClientConnectionConfiguration {
            addr_config: ClientAddrConfiguration::from_string(
                format!("127.0.0.1:{}", port).as_str()
            ).unwrap(),
            cert_mode: CertificateVerificationMode::SkipVerification,
            defaultables: Default::default(),
        },
    ).expect("Client should connect");
}

#[test]
fn test_client_sends_message_server_receives() {
    let mut server_app = create_test_server();
    let mut client_app = create_test_client();
    let test_port = TEST_PORT_BASE;
    
    // Start server
    start_server(&mut server_app, test_port);
    std::thread::sleep(Duration::from_millis(100));
    
    // Connect client
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    
    // Update apps to process connection
    server_app.update();
    client_app.update();
    std::thread::sleep(Duration::from_millis(50));
    
    // Client sends a Pong message
    let test_timestamp = 42424242u64;
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().expect("Client should be connected");
        connection.send_message(NetworkMessage::Pong { timestamp: test_timestamp })
            .expect("Should send message");
    }
    
    // Update to process message
    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    server_app.update();
    
    // Server receives and verifies the message
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().expect("Server should have endpoint");
        
        let clients = endpoint.clients();
        assert_eq!(clients.len(), 1, "Should have exactly one connected client");
        
        let client_id = clients[0];
        let message = endpoint.try_receive_message::<NetworkMessage>(client_id)
            .expect("Should receive message from client");
        
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
    let mut server_app = create_test_server();
    let mut client_app = create_test_client();
    let test_port = TEST_PORT_BASE + 1;
    
    // Start server
    start_server(&mut server_app, test_port);
    std::thread::sleep(Duration::from_millis(100));
    
    // Connect client
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    
    // Update apps
    server_app.update();
    client_app.update();
    std::thread::sleep(Duration::from_millis(50));
    
    // Server sends a Ping message
    let test_timestamp = 99999999u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().expect("Server should have endpoint");
        
        let clients = endpoint.clients();
        assert!(!clients.is_empty(), "Should have connected clients");
        
        for client_id in clients {
            endpoint.send_message(client_id, NetworkMessage::Ping { timestamp: test_timestamp })
                .expect("Should send message");
        }
    }
    
    // Update to process message
    server_app.update();
    std::thread::sleep(Duration::from_millis(100));
    client_app.update();
    
    // Client receives and verifies the message
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().expect("Client should be connected");
        
        let message = connection.try_receive_message::<NetworkMessage>()
            .expect("Should receive message from server");
        
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
    let mut server_app = create_test_server();
    let mut client_app = create_test_client();
    let test_port = TEST_PORT_BASE + 2;
    
    // Setup
    start_server(&mut server_app, test_port);
    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    server_app.update();
    client_app.update();
    
    // Step 1: Server sends Ping
    let ping_timestamp = 111111u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let client_id = endpoint.clients()[0];
        endpoint.send_message(client_id, NetworkMessage::Ping { timestamp: ping_timestamp })
            .expect("Server should send Ping");
    }
    
    server_app.update();
    std::thread::sleep(Duration::from_millis(50));
    client_app.update();
    
    // Step 2: Client receives Ping and sends Pong
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        
        let message = connection.try_receive_message::<NetworkMessage>()
            .expect("Client should receive Ping");
        
        match message {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, ping_timestamp);
                // Send Pong back
                connection.send_message(NetworkMessage::Pong { timestamp })
                    .expect("Client should send Pong");
            }
            _ => panic!("Expected Ping"),
        }
    }
    
    client_app.update();
    std::thread::sleep(Duration::from_millis(50));
    server_app.update();
    
    // Step 3: Server receives Pong
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let client_id = endpoint.clients()[0];
        
        let message = endpoint.try_receive_message::<NetworkMessage>(client_id)
            .expect("Server should receive Pong");
        
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
    let mut server_app = create_test_server();
    let mut client_app = create_test_client();
    let test_port = TEST_PORT_BASE + 3;
    
    start_server(&mut server_app, test_port);
    std::thread::sleep(Duration::from_millis(100));
    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));
    server_app.update();
    client_app.update();
    
    // Client sends multiple messages
    let timestamps = vec![1000u64, 2000u64, 3000u64, 4000u64];
    {
        let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        
        for &timestamp in &timestamps {
            connection.send_message(NetworkMessage::Pong { timestamp })
                .expect("Should send message");
        }
    }
    
    client_app.update();
    std::thread::sleep(Duration::from_millis(150));
    server_app.update();
    
    // Server receives all messages in order
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let client_id = endpoint.clients()[0];
        
        let mut received_timestamps = Vec::new();
        while let Some(message) = endpoint.try_receive_message::<NetworkMessage>(client_id) {
            match message {
                NetworkMessage::Pong { timestamp } => {
                    received_timestamps.push(timestamp);
                }
                _ => panic!("Expected Pong messages"),
            }
        }
        
        assert_eq!(received_timestamps.len(), timestamps.len(), "Should receive all messages");
        assert_eq!(received_timestamps, timestamps, "Messages should be received in order");
    }
}

#[test]
fn test_server_broadcasts_to_multiple_clients() {
    let mut server_app = create_test_server();
    let mut client1 = create_test_client();
    let mut client2 = create_test_client();
    let test_port = TEST_PORT_BASE + 4;
    
    // Setup server
    start_server(&mut server_app, test_port);
    std::thread::sleep(Duration::from_millis(100));
    
    // Connect both clients
    connect_client(&mut client1, test_port);
    connect_client(&mut client2, test_port);
    std::thread::sleep(Duration::from_millis(300));
    
    server_app.update();
    client1.update();
    client2.update();
    
    // Server broadcasts to all clients
    let broadcast_timestamp = 777777u64;
    {
        let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
        let endpoint = server.get_endpoint_mut().unwrap();
        let clients = endpoint.clients();
        
        assert_eq!(clients.len(), 2, "Should have 2 connected clients");
        
        for client_id in clients {
            endpoint.send_message(client_id, NetworkMessage::Ping { timestamp: broadcast_timestamp })
                .expect("Should broadcast message");
        }
    }
    
    server_app.update();
    std::thread::sleep(Duration::from_millis(100));
    client1.update();
    client2.update();
    
    // Both clients receive the message
    {
        let mut client = client1.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let message = connection.try_receive_message::<NetworkMessage>()
            .expect("Client 1 should receive message");
        
        match message {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, broadcast_timestamp);
            }
            _ => panic!("Expected Ping"),
        }
    }
    
    {
        let mut client = client2.world_mut().resource_mut::<QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let message = connection.try_receive_message::<NetworkMessage>()
            .expect("Client 2 should receive message");
        
        match message {
            NetworkMessage::Ping { timestamp } => {
                assert_eq!(timestamp, broadcast_timestamp);
            }
            _ => panic!("Expected Ping"),
        }
    }
}

#[test]
fn test_message_serialization() {
    // Test Ping message
    let ping = NetworkMessage::Ping { timestamp: 12345 };
    let serialized = bincode::serialize(&ping).expect("Should serialize");
    let deserialized: NetworkMessage = bincode::deserialize(&serialized).expect("Should deserialize");
    
    match deserialized {
        NetworkMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
        _ => panic!("Wrong message type"),
    }
    
    // Test Pong message
    let pong = NetworkMessage::Pong { timestamp: 67890 };
    let serialized = bincode::serialize(&pong).expect("Should serialize");
    let deserialized: NetworkMessage = bincode::deserialize(&serialized).expect("Should deserialize");
    
    match deserialized {
        NetworkMessage::Pong { timestamp } => assert_eq!(timestamp, 67890),
        _ => panic!("Wrong message type"),
    }
}

#[test]
fn test_server_starts_successfully() {
    let mut server_app = create_test_server();
    let test_port = TEST_PORT_BASE + 5;
    
    let mut server = server_app.world_mut().resource_mut::<QuinnetServer>();
    let result = server.start_endpoint(
        ServerEndpointConfiguration {
            addr_config: EndpointAddrConfiguration::from_ip(
                Ipv6Addr::LOCALHOST,
                test_port
            ),
            cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "localhost".to_string(),
            },
            defaultables: Default::default(),
        }
    );
    
    assert!(result.is_ok(), "Server should start successfully");
    assert!(server.is_listening(), "Server should be listening");
}
