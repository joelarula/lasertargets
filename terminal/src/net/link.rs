//! The connection to the server: connects and keeps reconnecting, hands every incoming message
//! to the other plugins as [`ServerMessage`], and sends what they write as `ToServer`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use bevy::prelude::*;
use bevy_quinnet::client::certificate::CertificateVerificationMode;
use bevy_quinnet::client::connection::{ClientAddrConfiguration, ConnectionLocalId};
use bevy_quinnet::client::{ClientConnectionConfiguration, QuinnetClient, QuinnetClientPlugin};
use bevy_quinnet::shared::QuinnetSyncPreUpdate;
use common::curator_core::CuratorEventEnvelope;
use common::network::{NetworkMessage, SERVER_PORT, ToServer};
use common::state::{CalibrationState, ServerState, TerminalState};
use terminal_ui::{Docking, ItemState, ToolbarItem, ToolbarPressed};

const SERVER_HOST: &str = "lasertargets.local";
const FALLBACK_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 110));
const RECONNECT_SECONDS: f32 = 3.0;

/// A message from the server; each plugin picks out the variants it cares about
#[derive(Message, Debug, Clone)]
pub struct ServerMessage(pub NetworkMessage);

/// Where the server is: `LASERTARGETS_IP` or `SERVER_IP` if set, else `lasertargets.local`,
/// else a fixed LAN address
#[derive(Resource, Debug, Clone)]
struct ServerAddress {
    ip: IpAddr,
    port: u16,
}

impl Default for ServerAddress {
    fn default() -> Self {
        let from_env = std::env::var("LASERTARGETS_IP")
            .or_else(|_| std::env::var("SERVER_IP"))
            .ok()
            .and_then(|ip| ip.parse().ok());
        let from_dns = || {
            (SERVER_HOST, SERVER_PORT)
                .to_socket_addrs()
                .ok()?
                .find(|addr| addr.is_ipv4())
                .map(|addr| addr.ip())
        };
        let ip = from_env.or_else(from_dns).unwrap_or(FALLBACK_IP);
        Self { ip, port: SERVER_PORT }
    }
}

/// Retries the connection every few seconds while it is down
#[derive(Resource)]
struct Reconnect {
    timer: Timer,
    /// The latest attempt, closed before the next so attempts do not pile up
    attempt: Option<ConnectionLocalId>,
}

impl Default for Reconnect {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(RECONNECT_SECONDS, TimerMode::Repeating), attempt: None }
    }
}

#[derive(Component)]
struct ConnectionButton;

pub struct LinkPlugin;

impl Plugin for LinkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .init_state::<TerminalState>()
            .init_state::<ServerState>()
            .init_state::<CalibrationState>()
            .init_resource::<ServerAddress>()
            .init_resource::<Reconnect>()
            .add_message::<ServerMessage>()
            .add_message::<ToServer>()
            .add_systems(Startup, (connect, spawn_buttons))
            // Right after quinnet syncs, so every Update system sees this frame's messages
            .add_systems(PreUpdate, receive.after(QuinnetSyncPreUpdate))
            .add_systems(
                Update,
                (track_connection, reconnect, show_connection.run_if(state_changed::<TerminalState>)).chain(),
            )
            .add_systems(PostUpdate, send);
    }
}

fn open_connection(client: &mut QuinnetClient, address: &ServerAddress, reconnect: &mut Reconnect) {
    if let Some(attempt) = reconnect.attempt.take() {
        let _ = client.close_connection(attempt);
    }
    let config = ClientConnectionConfiguration {
        addr_config: ClientAddrConfiguration::from_ips(address.ip, address.port, IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
        cert_mode: CertificateVerificationMode::SkipVerification,
        defaultables: Default::default(),
    };
    match client.open_connection(config) {
        Ok(id) => {
            reconnect.attempt = Some(id);
            info!("Connecting to server at {}:{} (attempt {id})", address.ip, address.port);
        }
        Err(e) => warn!("Failed to open a connection to {}:{}: {e:?}", address.ip, address.port),
    }
}

fn connect(mut client: ResMut<QuinnetClient>, address: Res<ServerAddress>, mut reconnect: ResMut<Reconnect>) {
    open_connection(&mut client, &address, &mut reconnect);
}

/// Follows the connection with `TerminalState`. A lost connection passes through `Disconnected`
/// for a frame so plugins can clean up on `OnEnter(Connecting)` before the reconnect.
fn track_connection(
    client: Res<QuinnetClient>,
    state: Res<State<TerminalState>>,
    mut next: ResMut<NextState<TerminalState>>,
    mut server_state: ResMut<NextState<ServerState>>,
) {
    match (client.is_connected(), state.get()) {
        (true, TerminalState::Connected) | (false, TerminalState::Connecting) => {}
        (true, _) => {
            info!("Connected to server");
            next.set(TerminalState::Connected);
        }
        (false, TerminalState::Connected) => {
            info!("Disconnected from server");
            next.set(TerminalState::Disconnected);
            server_state.set(ServerState::Menu);
        }
        (false, TerminalState::Disconnected) => next.set(TerminalState::Connecting),
    }
}

fn reconnect(
    mut client: ResMut<QuinnetClient>,
    address: Res<ServerAddress>,
    mut reconnect: ResMut<Reconnect>,
    state: Res<State<TerminalState>>,
    time: Res<Time>,
) {
    if *state.get() != TerminalState::Connecting {
        reconnect.timer.reset();
        return;
    }
    if reconnect.timer.tick(time.delta()).just_finished() {
        open_connection(&mut client, &address, &mut reconnect);
    }
}

fn spawn_buttons(mut commands: Commands) {
    commands.spawn((
        ConnectionButton,
        ToolbarItem::new("connection_status").icon("\u{f0ec}").text("Disconnected").dock(Docking::Right),
    ));
    commands
        .spawn(ToolbarItem::new("shutdown_server").icon("\u{f011}").text("Shutdown").order(99).dock(Docking::Right))
        .observe(|_: On<ToolbarPressed>, mut to_server: MessageWriter<ToServer>| {
            info!("Asking the server to shut down");
            to_server.write(ToServer(NetworkMessage::ShutdownServer));
        });
}

fn show_connection(
    state: Res<State<TerminalState>>,
    address: Res<ServerAddress>,
    mut button: Single<&mut ToolbarItem, With<ConnectionButton>>,
) {
    let connected = *state.get() == TerminalState::Connected;
    let ServerAddress { ip, port } = *address;
    button.state = if connected { ItemState::On } else { ItemState::Off };
    button.text = Some(if connected {
        format!("Connected to {SERVER_HOST} ({ip}:{port})")
    } else {
        format!("Disconnected ({ip}:{port})")
    });
}

/// Answers pings, bridges curator envelopes into the local curator, and passes every message on
fn receive(
    mut client: ResMut<QuinnetClient>,
    mut messages: MessageWriter<ServerMessage>,
    mut envelopes: MessageWriter<CuratorEventEnvelope>,
    mut to_server: MessageWriter<ToServer>,
) {
    let Some(connection) = client.get_connection_mut() else { return };
    let Some(channel) = connection.default_channel() else { return };
    while let Some(bytes) = connection.try_receive_payload(channel) {
        let message = match NetworkMessage::from_bytes(&bytes) {
            Ok(message) => message,
            Err(e) => {
                warn!("Failed to decode a server message: {e:?}");
                continue;
            }
        };
        match &message {
            NetworkMessage::CameraPreviewFrame(_) | NetworkMessage::BroadcastScenePaths(_) => {}
            other => debug!("Server: {other:?}"),
        }
        match &message {
            NetworkMessage::Ping { timestamp } => {
                to_server.write(ToServer(NetworkMessage::Pong { timestamp: *timestamp }));
            }
            NetworkMessage::CuratorEnvelope(envelope) => {
                envelopes.write(envelope.clone());
            }
            _ => {}
        }
        messages.write(ServerMessage(message));
    }
}

fn send(mut client: ResMut<QuinnetClient>, mut outbox: MessageReader<ToServer>) {
    let connected = client.is_connected();
    let mut connection = client.get_connection_mut().filter(|_| connected);
    for ToServer(message) in outbox.read() {
        let Some(connection) = connection.as_mut() else {
            warn!("Not connected, dropped {message:?}");
            continue;
        };
        let sent = message.to_bytes().map_err(|e| e.to_string()).and_then(|payload| {
            connection.send_payload(payload).map_err(|e| e.to_string())
        });
        if let Err(e) = sent {
            warn!("Failed to send {message:?}: {e}");
        }
    }
}
