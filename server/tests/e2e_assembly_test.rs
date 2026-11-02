use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy_quinnet::client::certificate::*;
use bevy_quinnet::client::connection::*;
use bevy_quinnet::client::*;
use bevy_quinnet::server::*;
use common::network::NetworkMessage;
use common::session::{FinishSessionEvent, PauseSessionEvent, ResumeSessionEvent, Session};
use common::state::{ServerState, SessionState};
use server::curator_core;
use server::plugins::network::{FromClientMessage, NetworkingConfiguration};
use server::plugins::projector::ProjectorDacController;

const TEST_PORT_BASE: u16 = 7300;

#[derive(Resource, Debug, Default)]
struct ReceivedMessages(pub Vec<NetworkMessage>);

fn handle_client_messages_system(
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

fn handle_server_messages_system(
    mut messages: MessageReader<FromClientMessage>,
    mut received_messages: ResMut<ReceivedMessages>,
) {
    for msg in messages.read() {
        received_messages.0.push(msg.message.clone());
    }
}

fn disable_dac(app: &mut App) {
    app.insert_resource(ProjectorDacController {
        enabled: false,
        ..Default::default()
    });
}

/// Creates a test server instance using the real assembled server app with minigames.
fn create_test_game_server(port: u16) -> App {
    let mut app = server::create_server_app(ScheduleRunnerPlugin::run_once());
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

fn create_test_client() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_plugins(QuinnetClientPlugin::default())
        .insert_resource(ReceivedMessages::default())
        .add_systems(Update, handle_client_messages_system);
    app
}

fn client_send(client_app: &mut App, message: NetworkMessage) {
    let mut client = client_app.world_mut().resource_mut::<QuinnetClient>();
    let connection = client.get_connection_mut().expect("Client should be connected");
    connection
        .send_payload(message.to_bytes().expect("Serialize"))
        .expect("Send");
}

fn connect_client(client_app: &mut App, port: u16) {
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
fn test_curator_menu_network_prompt_selection() {
    let test_port = TEST_PORT_BASE + 19;
    let mut server_app = create_test_game_server(test_port);

    let prompt_id = {
        let active_prompt = server_app.world().resource::<server::plugins::menu::ActiveMenuPrompt>();
        active_prompt.prompt_id.expect("Active prompt")
    };

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

    for _ in 0..4 {
        server_app.update();
    }

    let server_state = server_app.world().resource::<State<ServerState>>();
    assert_eq!(*server_state.get(), ServerState::Session);

    let mut session_query = server_app.world_mut().query::<&common::game::GameSession>();
    let session = session_query.iter(server_app.world()).next().expect("Game session");
    assert_eq!(session.game_id, 101);
}

