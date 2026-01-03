use bevy::prelude::*;
use bevy_quinnet::client::{certificate::CertificateVerificationMode, connection::ClientAddrConfiguration, ClientConnectionConfiguration, QuinnetClient, QuinnetClientPlugin};
use common::network::SERVER_PORT;
use common::state::TerminalState;
use std::net::{IpAddr, Ipv6Addr};
// Removed unused imports: use bevy::color::palettes::css::{GREEN, RED};
use crate::plugins::toolbar::{ToolbarRegistry, ToolbarItem, Docking};

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

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(QuinnetClientPlugin::default())
            .init_resource::<NetworkingConfiguration>()
            .add_systems(Startup, (start_client, register_connection_toolbar_button))
            .add_systems(Update, (handle_client_connection_events, update_connection_toolbar_button));
    }
}

fn start_client(
    mut client: ResMut<QuinnetClient>,
    config: Res<NetworkingConfiguration>,
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
        Ok(_) => info!("Connection initiated successfully"),
        Err(e) => warn!("Failed to initiate connection: {:?}", e),
    }
}

fn handle_client_connection_events(
    mut client: ResMut<QuinnetClient>,
    config: Res<NetworkingConfiguration>,
    current_state: Res<State<TerminalState>>,
    mut next_state: ResMut<NextState<TerminalState>>,
) {
    if client.is_connected() {
        if *current_state.get() != TerminalState::Connected {
            info!("Connected to server!");
            next_state.set(TerminalState::Connected);
        }
    } else {
        if *current_state.get() != TerminalState::Connecting {
            info!("Disconnected from server, attempting to reconnect...");
            next_state.set(TerminalState::Connecting);
        }
        
        // Attempt to reconnect if not currently connected
        if client.get_connection().is_none() {
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
                Ok(_) => info!("Reconnection attempt initiated"),
                Err(e) => debug!("Reconnection attempt failed: {:?}", e),
            }
        }
    }
}

const CONN_BTN_NAME: &str = "connection_status";

fn register_connection_toolbar_button(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_button(ToolbarItem {
        name: CONN_BTN_NAME.to_string(),
        label: "Connection".to_string(),
        icon: Some("\u{f057}".to_string()), // times-circle icon for disconnected/connecting
        is_active: false,
        docking: Docking::Left,
        button_size: 36.0,
    });
}

fn update_connection_toolbar_button(
    terminal_state: Res<State<TerminalState>>,
    mut toolbar: ResMut<ToolbarRegistry>,
) {
    if terminal_state.is_changed() {
        let (icon, is_active) = match *terminal_state.get() {
            TerminalState::Connected => ("\u{f058}".to_string(), true), // check-circle icon for connected
            TerminalState::Connecting => ("\u{f057}".to_string(), false), // times-circle icon for connecting
        };
        toolbar.update_button_state(CONN_BTN_NAME, is_active);
        toolbar.update_button_icon(CONN_BTN_NAME, Some(icon));
    }
}
