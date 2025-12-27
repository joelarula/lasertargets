mod util;
use util::{
    ReceivedMessages, connect_client, create_test_client, create_test_server, flush_server_messages,
};

use bevy::prelude::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::NetworkMessage;
use std::time::Duration;

#[test]
fn test_query_projector_config() {
    let test_port = 7200;
    let mut server_app = create_test_server(test_port);
    let mut client_app = create_test_client();

    connect_client(&mut client_app, test_port);
    std::thread::sleep(Duration::from_millis(200));

    // Client sends Query
    {
        let mut client = client_app
            .world_mut()
            .resource_mut::<bevy_quinnet::client::QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::QueryProjectorConfig
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app); // Server handles query and sends response

    std::thread::sleep(Duration::from_millis(100));
    client_app.update(); // Client receives response

    // Verify response
    {
        let received = client_app.world().resource::<ReceivedMessages>();
        assert!(!received.0.is_empty(), "Client should receive response");
        match &received.0[0] {
            NetworkMessage::ProjectorConfigResponse(_) => {
                info!("Received projector config response")
            }
            _ => panic!("Expected ProjectorConfigResponse, got {:?}", received.0[0]),
        }
    }
}

#[test]
fn test_update_config_broadcast() {
    let test_port = 7201;
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
        let mut client = client1_app
            .world_mut()
            .resource_mut::<bevy_quinnet::client::QuinnetClient>();
        let connection = client.get_connection_mut().unwrap();
        let payload = NetworkMessage::UpdateProjectorConfig(new_config.clone())
            .to_bytes()
            .expect("Serialize");
        connection.send_payload(payload).unwrap();
    }

    client1_app.update();
    std::thread::sleep(Duration::from_millis(100));
    flush_server_messages(&mut server_app); // Server updates and broadcasts

    std::thread::sleep(Duration::from_millis(100));
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
        match &received.0[0] {
            NetworkMessage::UpdateProjectorConfig(received_config) => {
                assert_eq!(received_config.enabled, true);
                assert_eq!(received_config.angle, 45.0);
            }
            _ => panic!(
                "Expected UpdateProjectorConfig broadcast, got {:?}",
                received.0[0]
            ),
        }
    }
}
