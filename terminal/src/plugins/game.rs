
use common::toolbar::{ToolbarItem, Docking, ItemState,ToolbarButton};
use bevy::prelude::*;
use common::{game::{GameSessionUpdate as GameSessionUpdate, GameSessionCreated}, state::{GameState, ServerState}};

const BTN_NAME: &str = "exit_game";

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
        info!("[GamePlugin] Listener: Received GameSessionCreated: {:?}", event.game_session);
        // Despawn any existing GameSession entity
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
        // Spawn new GameSession entity
        commands.spawn((event.game_session.clone(), GameSessionMarker));
        info!("[GamePlugin] Spawned new GameSession entity");
    }
}

/// Updates the GameSession entity when a GameSessionUpdate is received
fn update_gamesession_entity(
    mut updates: MessageReader<GameSessionUpdate>,
    mut query: Query<&mut common::game::GameSession, With<GameSessionMarker>>,
) {
    for update in updates.read() {
        info!("[GamePlugin] Listener: Received GameSessionUpdate: {:?}", update.game_session);
        if let Ok(mut session) = query.single_mut() {
            *session = update.game_session.clone();
        }
    }
}
/// Spawns the Exit Game toolbar item when ServerState is InGame
fn spawn_exit_game_toolbar_item(
    mut commands: Commands,
) {

          commands.spawn((
                ToolbarItem {
                    name: BTN_NAME.to_string(),
                    order: 1,
                    icon: Some("\u{f060}".to_string()), // NerdFont arrow back icon (U+F060)
                    state: ItemState::On,
                    docking: Docking::Left,
                    button_size: 36.0,
                    ..Default::default()
                },
                ExitGameButton,
            ));
        
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
    interaction_query: Query<(&ToolbarButton, &Interaction), Changed<Interaction>>,
    mut client: ResMut<bevy_quinnet::client::QuinnetClient>,
    session_query: Query<&common::game::GameSession, With<GameSessionMarker>>,
) {

    for (toolbar_item, interaction) in &interaction_query {
        info!("Handling interaction for toolbar item: {}", toolbar_item.name);
        if toolbar_item.name == BTN_NAME && *interaction == Interaction::Pressed {
            info!("[GamePlugin] Exit Game button pressed");
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
