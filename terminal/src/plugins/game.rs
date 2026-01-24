
use common::toolbar::{ToolbarItem, Docking, ItemState};
use bevy::prelude::*;
use common::{game::{GameSessionUpdate as GameSessionUpdate, GameSessionCreated}, state::{GameState, ServerState}};

#[derive(Component)]
struct ExitGameButton;

#[derive(Component)]

struct GameSessionMarker;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::InGame), spawn_exit_game_toolbar_item);
        app.add_systems(OnExit(ServerState::InGame), despawn_exit_game_toolbar_item);
        app.add_systems(Update, spawn_gamesession_entity);
        app.add_systems(Update, update_gamesession_entity);
        app.add_systems(Update, handle_exit_game_button);
    }
}
/// Spawns a GameSession entity when GameSessionCreated is received
fn spawn_gamesession_entity(
    mut commands: Commands,
    mut events: MessageReader<GameSessionCreated>,
    query: Query<Entity, With<GameSessionMarker>>,
) {
    for event in events.read() {
        // Despawn any existing GameSession entity
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
        // Spawn new GameSession entity
        commands.spawn((event.game_session.clone(), GameSessionMarker));
    }
}

/// Updates the GameSession entity when a GameSessionUpdate is received
fn update_gamesession_entity(
    mut updates: MessageReader<GameSessionUpdate>,
    mut query: Query<&mut common::game::GameSession, With<GameSessionMarker>>,
) {
    for update in updates.read() {
        if let Ok(mut session) = query.single_mut() {
            *session = update.game_session.clone();
        }
    }
}
/// Spawns the Exit Game toolbar item when ServerState is InGame
fn spawn_exit_game_toolbar_item(
    mut commands: Commands,
    server_state: Res<State<ServerState>>,
    query: Query<Entity, With<ExitGameButton>>,
) {
    if server_state.get() == &ServerState::InGame {
        // Only spawn if not already present
        if query.is_empty() {
            commands.spawn((
                ToolbarItem {
                    name: "exit_game".to_string(),
                    order: 1,
                    icon: Some("󰗼".to_string()), // Example NerdFont icon
                    state: ItemState::On,
                    docking: Docking::Left,
                    button_size: 36.0,
                    ..Default::default()
                },
                ExitGameButton,
            ));
        }
    } else {
        // Despawn if not in InGame state
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Despawns the Exit Game toolbar item when leaving InGame state
fn despawn_exit_game_toolbar_item(
    mut commands: Commands,
    query: Query<Entity, With<ExitGameButton>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}


/// Handles ExitGameButton presses and sends ExitGameSession message to the server
fn handle_exit_game_button(
    mut interaction_query: Query<(&Interaction, &ToolbarItem), (Changed<Interaction>, With<ExitGameButton>)>,
    mut client: ResMut<bevy_quinnet::client::QuinnetClient>,
    session_query: Query<&common::game::GameSession, With<GameSessionMarker>>,
    terminal_state: Res<State<common::state::TerminalState>>,
) {
    if *terminal_state.get() != common::state::TerminalState::Connected {
        return;
    }
    for (interaction, _toolbar_item) in &mut interaction_query {
        if let Interaction::Pressed = interaction {
            // Get the current session id
            if let Ok(session) = session_query.single() {
                if let Some(connection) = client.get_connection_mut() {
                    let payload = common::network::NetworkMessage::ExitGameSession(session.session_id)
                        .to_bytes()
                        .expect("Failed to serialize ExitGameSession");
                    if let Err(e) = connection.send_payload(payload) {
                        error!("Failed to send ExitGameSession: {e}");
                    }
                }
            }
        }
    }
}
