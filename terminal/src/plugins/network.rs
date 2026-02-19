use common::state::{CalibrationState, GameState, ServerState};
use bevy::prelude::*;
use bevy_quinnet::client::connection::ClientSideConnection;
use bevy_quinnet::client::{certificate::CertificateVerificationMode, connection::ClientAddrConfiguration, ClientConnectionConfiguration, QuinnetClient, QuinnetClientPlugin};
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::network::{NetworkMessage, SERVER_PORT};
use common::state::TerminalState;
use common::toolbar::{Docking, ItemState, ToolbarItem};
use std::net::{IpAddr, Ipv6Addr};
use common::game::{GameSessionUpdate, GameSessionCreated};
use hunter::model::HunterGameStats;
use hunter::server::SpawnHunterTargetEvent;
use crate::plugins::path::{SpawnPathEvent, DespawnPathEvent, UpdatePathPositionEvent};

const CONN_BTN_NAME: &str = "connection_status";

/// Marker component for the connection status toolbar button
#[derive(Component)]
struct ConnectionButton;

#[derive(Resource, Debug, Clone)]
pub struct NetworkingConfiguration {
    pub ip: IpAddr,
    pub port: u16,
}



impl Default for NetworkingConfiguration {
    fn default() -> Self {
        Self {
            ip: IpAddr::V6(Ipv6Addr::LOCALHOST),
            port: SERVER_PORT,
        }
    }
}



pub struct NetworkPlugin;

#[derive(Resource, Default)]
struct ConnectionAttempt {
    in_flight: bool,
}

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .init_resource::<NetworkingConfiguration>()
            .init_resource::<ConnectionAttempt>()
            .add_message::<GameSessionCreated>()
            .add_message::<GameSessionUpdate>()
            .add_message::<SpawnHunterTargetEvent>()
            .add_message::<SpawnPathEvent>()
            .add_message::<DespawnPathEvent>()
            .add_message::<UpdatePathPositionEvent>()
            .init_resource::<ReconnectTimer>()
            .add_systems(Startup, start_client)
            .add_systems(Startup, register_connection_toolbar_button)
            .add_systems(Update, handle_client_connection_events)
            .add_systems(Update, reconnect_if_needed)
            .add_systems(Update, update_connection_toolbar_button)
            .add_systems(Update, receive_server_messages)
            .add_systems(Update, send_projector_config_updates)
            .add_systems(Update, send_camera_config_updates)
            .add_systems(Update, send_scene_config_updates);

    }
}


fn start_client(
    mut client: ResMut<QuinnetClient>,
    config: Res<NetworkingConfiguration>,
    mut attempt: ResMut<ConnectionAttempt>,
) {
    info!("Connecting to server...");
    match client.open_connection(ClientConnectionConfiguration {
        addr_config: ClientAddrConfiguration::from_ips(
            config.ip,
            config.port,
            IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            0,
        ),
        cert_mode: CertificateVerificationMode::SkipVerification,
        defaultables: Default::default(),
    }) {
        Ok(_) => {
            attempt.in_flight = true;
            info!("Connection initiated successfully");
        }
        Err(e) => warn!("Failed to initiate connection: {:?}", e),
    }
}

fn handle_client_connection_events(
    mut client: ResMut<QuinnetClient>,
    config: Res<NetworkingConfiguration>,
    current_state: Res<State<TerminalState>>,
    mut next_state: ResMut<NextState<TerminalState>>,
    mut attempt: ResMut<ConnectionAttempt>,
) {
    if client.is_connected() {
        if *current_state.get() != TerminalState::Connected {
            info!("Connected to server!");
            next_state.set(TerminalState::Connected);
            attempt.in_flight = false;

            if let Some(connection) = client.get_connection_mut() {
                let queries = [
                    NetworkMessage::QueryServerState,
                    NetworkMessage::QueryGameState,
                    NetworkMessage::QueryCalibrationState,
                    NetworkMessage::QueryGameSession,
                    NetworkMessage::QueryProjectorConfig,
                    NetworkMessage::QueryCameraConfig,
                    NetworkMessage::QuerySceneConfig,
                    NetworkMessage::QuerySceneSetup,
                ];

                for query in queries {
                    if let Err(e) = connection.send_payload(query.to_bytes().unwrap()) {
                        warn!("Failed to send {:?}: {e}", query);
                    }
                }
            }
        }
    } else {
        if *current_state.get() != TerminalState::Connecting {
            info!("Disconnected from server, attempting to reconnect...");
            next_state.set(TerminalState::Connecting);
        }
        attempt.in_flight = false;
    }
}


fn register_connection_toolbar_button(mut commands: Commands) {
    commands.spawn((
        ToolbarItem {
            name: CONN_BTN_NAME.to_string(),
            order: 0,
            text: Some("Connection".to_string()),
            icon: Some("\u{f057}".to_string()), // times-circle icon for disconnected/connecting
            state: ItemState::Disabled,
            docking: Docking::Right,
            button_size: 36.0,
            ..default()
        },
        ConnectionButton,
    ));
}

fn update_connection_toolbar_button(
    terminal_state: Res<State<TerminalState>>,
    mut button_query: Query<&mut ToolbarItem, With<ConnectionButton>>,
) {
    if terminal_state.is_changed() {
        let (icon, state) = match *terminal_state.get() {
            TerminalState::Connected => ("\u{f058}".to_string(), ItemState::On), // check-circle icon for connected
            TerminalState::Connecting => ("\u{f057}".to_string(), ItemState::Disabled), // times-circle icon for connecting
        };
        
        if let Ok(mut item) = button_query.single_mut() {
            item.state = state;
            item.icon = Some(icon);
        }
    }
}

fn receive_server_messages(
    mut commands: Commands,
    mut client: ResMut<QuinnetClient>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut camera_config: ResMut<CameraConfiguration>,
    mut scene_config: ResMut<SceneConfiguration>,
    mut scene_setup: ResMut<common::scene::SceneSetup>,
    mut hunter_stats: Option<ResMut<HunterGameStats>>,
    mut game_session_created_writer: MessageWriter<GameSessionCreated>,
    mut game_session_update_writer: MessageWriter<GameSessionUpdate>,
    mut spawn_hunter_target_writer: MessageWriter<SpawnHunterTargetEvent>,
    mut spawn_path_writer: MessageWriter<SpawnPathEvent>,
    mut despawn_path_writer: MessageWriter<DespawnPathEvent>,
    mut update_path_position_writer: MessageWriter<UpdatePathPositionEvent>,
    mut next_server_state: ResMut<NextState<ServerState>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
) {
    if !client.is_connected() {
        return;
    }

    if let Some(mut connection) = client.get_connection_mut() {
        if let Some(channel_id) = connection.default_channel() {
            let mut messages = Vec::new();
            while let Some(bytes) = connection.try_receive_payload(channel_id) {
                messages.push(bytes);
            }
            drop(connection); // Release mutable borrow

            for bytes in messages {
                match NetworkMessage::from_bytes(&bytes) {
                    Ok(msg) => {
                        info!("[Network] Incoming message: {:?}", msg);
                        match msg {
                            NetworkMessage::ProjectorConfigUpdate(config)  => {
                                if *projector_config != config {
                                    *projector_config.bypass_change_detection() = config;
                                    projector_config.set_changed();
                                    info!("Updated projector configuration from server");
                                }
                            }
                            NetworkMessage::CameraConfigUpdate(config) => {
                                if *camera_config != config {
                                    *camera_config.bypass_change_detection() = config;
                                    camera_config.set_changed();
                                    info!("Updated camera configuration from server");
                                }
                            }
                            NetworkMessage::SceneConfigUpdate(config) => {
                                if *scene_config != config {
                                    *scene_config.bypass_change_detection() = config;
                                    scene_config.set_changed();
                                    info!("Updated scene configuration from server");
                                }
                            }
                            NetworkMessage::SceneSetupUpdate(setup) => {
                                if *scene_setup != setup {
                                    *scene_setup.bypass_change_detection() = setup.clone();
                                    scene_setup.set_changed();
                                    info!("Updated SceneSetup from server");
                                }
                                if *camera_config != setup.camera {
                                    *camera_config.bypass_change_detection() = setup.camera;
                                    camera_config.set_changed();
                                    info!("Updated camera configuration from SceneSetup");
                                }
                                if *projector_config != setup.projector {
                                    *projector_config.bypass_change_detection() = setup.projector;
                                    projector_config.set_changed();
                                    info!("Updated projector configuration from SceneSetup");
                                }
                                if *scene_config != setup.scene {
                                    *scene_config.bypass_change_detection() = setup.scene;
                                    scene_config.set_changed();
                                    info!("Updated scene configuration from SceneSetup");
                                }
                            }
                            NetworkMessage::GameSessionCreated(session) => {
                                info!("Received GameSessionCreated from server: {:?}", session);
                                game_session_created_writer.write(GameSessionCreated { game_session: session });
                            }
                            NetworkMessage::GameSessionUpdate(session) => {
                                info!("Received GameSessionUpdate from server: {:?}", session);
                                game_session_update_writer.write(GameSessionUpdate { game_session: session });
                            }
                            NetworkMessage::ServerStateUpdate(server_state) => {
                                info!("Received ServerStateUpdate from server: {:?}", server_state);
                                next_server_state.set(server_state);
                            }
                            NetworkMessage::GameStateUpdate(game_state) => {
                                info!("Received GameStateUpdate from server: {:?}", game_state);
                                next_game_state.set(game_state);
                            }
                            NetworkMessage::CalibrationStateUpdate(calibration_state) => {
                                info!("Received CalibrationStateUpdate from server: {:?}", calibration_state);
                                next_calibration_state.set(calibration_state);
                            }
                            NetworkMessage::ExitGameSession(uuid) => {
                                info!("Received ExitGameSession from server for session: {:?}", uuid);
                                
                            }
                            NetworkMessage::Ping { timestamp } => {
                                info!("Received Ping from server: timestamp={}", timestamp);
                                if let Some(mut connection) = client.get_connection_mut() {
                                    let pong = NetworkMessage::Pong { timestamp };
                                    if let Err(e) = connection.send_payload(pong.to_bytes().unwrap()) {
                                        warn!("Failed to send Pong: {e}");
                                    } else {
                                        info!("Sent Pong in response to Ping");
                                    }
                                }
                            }
                            NetworkMessage::SpawnHunterTarget(target, position) => {
                                info!("Received SpawnHunterTarget from server: target={:?}, position={:?}", target, position);
                                spawn_hunter_target_writer.write(SpawnHunterTargetEvent { target, position });
                            }
                            NetworkMessage::SpawnPath(uuid, path, position) => {
                                info!("Received SpawnPath from server: uuid={}, position={:?}", uuid, position);
                                spawn_path_writer.write(SpawnPathEvent { uuid, path, position });
                            }
                            NetworkMessage::DespawnPath(uuid) => {
                                info!("Received DespawnPath from server: uuid={}", uuid);
                                despawn_path_writer.write(DespawnPathEvent { uuid });
                            }
                            NetworkMessage::UpdatePathPosition(uuid, position) => {
                                debug!("Received UpdatePathPosition from server: uuid={}, position={:?}", uuid, position);
                                update_path_position_writer.write(UpdatePathPositionEvent { uuid, position });
                            }
                            NetworkMessage::HunterStatsUpdate { session_id, targets_spawned, targets_popped, score } => {
                                if let Some(mut stats) = hunter_stats.as_mut() {
                                    if stats.session_id != session_id {
                                        **stats = HunterGameStats {
                                            session_id,
                                            targets_spawned,
                                            targets_popped,
                                            score,
                                            target_events: Vec::new(),
                                            game_start_time: 0.0,
                                        };
                                    } else {
                                        stats.targets_spawned = targets_spawned;
                                        stats.targets_popped = targets_popped;
                                        stats.score = score;
                                    }
                                } else {
                                    commands.insert_resource(HunterGameStats {
                                        session_id,
                                        targets_spawned,
                                        targets_popped,
                                        score,
                                        target_events: Vec::new(),
                                        game_start_time: 0.0,
                                    });
                                }
                            }
                            _ => {
                                info!("Received unhandled message: {:?}", msg);
                            }
                        }
                    }
                    Err(e) => warn!("Failed to deserialize server message: {:?}", e),
                }
            }
        }
    }
}


fn send_projector_config_updates(
    projector_config: Res<ProjectorConfiguration>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    if projector_config.is_changed() && *terminal_state.get() == TerminalState::Connected {
        if let Some(connection) = client.get_connection_mut() {
            let payload = NetworkMessage::UpdateProjectorConfig(projector_config.clone())
                .to_bytes()
                .expect("Failed to serialize projector config");
            
            send_payload_and_log_error(connection, payload, "projector config update");
        }
    }
}

fn send_camera_config_updates(
    camera_config: Res<CameraConfiguration>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    if camera_config.is_changed() && *terminal_state.get() == TerminalState::Connected {
        if let Some(connection) = client.get_connection_mut() {
            let payload = NetworkMessage::UpdateCameraConfig(camera_config.clone())
                .to_bytes()
                .expect("Failed to serialize camera config");
            
            send_payload_and_log_error(connection, payload, "camera config update");
        }
    }
}

fn send_scene_config_updates(
    scene_config: Res<SceneConfiguration>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    if scene_config.is_changed() && *terminal_state.get() == TerminalState::Connected {
        if let Some(connection) = client.get_connection_mut() {
            let payload = NetworkMessage::UpdateSceneConfig(scene_config.clone())
                .to_bytes()
                .expect("Failed to serialize scene config");
            
            send_payload_and_log_error(connection, payload, "scene config update");
        }
    }
}


/// Helper function to send a payload and log an error if it fails.
fn send_payload_and_log_error(
    connection: &mut ClientSideConnection,
    payload: Vec<u8>,
    error_context: &str,
) {
    if let Err(e) = connection.send_payload(payload) {
        warn!("Failed to send {}: {:?}", error_context, e);
    } else {
        debug!("Sent {}", error_context);
    }
}


    /// Resource for reconnection timer
    #[derive(Resource, Default)]
    struct ReconnectTimer(Timer);

    /// System to periodically attempt reconnection if not connected
    fn reconnect_if_needed(
        mut client: ResMut<QuinnetClient>,
        config: Res<NetworkingConfiguration>,
        mut timer: ResMut<ReconnectTimer>,
        time: Res<Time>,
        current_state: Res<State<TerminalState>>,
        mut attempt: ResMut<ConnectionAttempt>,
    ) {
        // Only try to reconnect if not connected
        if !client.is_connected() {
            // Initialize timer if needed
            if timer.0.duration().as_secs_f32() == 0.0 {
                timer.0 = Timer::from_seconds(2.0, TimerMode::Repeating);
            }
            timer.0.tick(time.delta());
            if timer.0.just_finished()
                && *current_state.get() == TerminalState::Connecting
                && !attempt.in_flight
            {
                match client.open_connection(ClientConnectionConfiguration {
                    addr_config: ClientAddrConfiguration::from_ips(
                        config.ip,
                        config.port,
                        IpAddr::V6(Ipv6Addr::UNSPECIFIED),
                        0,
                    ),
                    cert_mode: CertificateVerificationMode::SkipVerification,
                    defaultables: Default::default(),
                }) {
                    Ok(_) => {
                        attempt.in_flight = true;
                        info!("[Reconnect] Reconnection attempt initiated");
                    }
                    Err(e) => debug!("[Reconnect] Reconnection attempt failed: {:?}", e),
                }
            }
        } else {
            // Reset timer if connected
            timer.0 = Timer::from_seconds(0.0, TimerMode::Once);
            attempt.in_flight = false;
        }
    }