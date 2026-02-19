
use common::{state::TerminalState, toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem}};
use bevy::{asset::uuid::Uuid, prelude::*};
use common::{game::{GameSessionUpdate as GameSessionUpdate, GameSessionCreated}, state::{GameState, ServerState}};

const GAME_BUTTON: &str = "exit_game";
const PAUSE_RESUME_BUTTON: &str = "pause_resume_game";

#[derive(Component)]
struct ExitGameButton;

#[derive(Component)]
struct PauseResumeButton;

#[derive(Component)]

struct GameSessionMarker;

#[derive(Resource, Default)]
struct LastGameSessionId(Option<Uuid>);

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastGameSessionId>();
        app.add_systems(OnEnter(ServerState::InGame), spawn_exit_game_toolbar_item);
        app.add_systems(OnEnter(ServerState::InGame), spawn_pause_resume_toolbar_item);
        app.add_systems(OnExit(ServerState::InGame), despawn_exit_game_toolbar_item);
        app.add_systems(OnExit(ServerState::InGame), despawn_pause_resume_toolbar_item);
        app.add_systems(OnExit(ServerState::InGame), clear_last_session_id);
        app.add_systems(OnEnter(TerminalState::Connecting), clear_game_session_on_disconnect);
        app.add_systems(Update, spawn_gamesession_entity);
        app.add_systems(Update, update_gamesession_entity);
        app.add_systems(Update, handle_exit_game_button);
        app.add_systems(Update, handle_pause_resume_button);
    }
}
/// Spawns a GameSession entity when GameSessionCreated is received
fn spawn_gamesession_entity(
    mut commands: Commands,
    mut events: MessageReader<GameSessionCreated>,
    query: Query<Entity, With<GameSessionMarker>>,
    mut last_session_id: ResMut<LastGameSessionId>,
) {
    for event in events.read() {
        info!("[GamePlugin] Listener: Received GameSessionCreated: {:?}", event.game_session);
        // Despawn any existing GameSession entity
        for entity in query.iter() {
            commands.entity(entity).despawn();
        }
        // Spawn new GameSession entity
        commands.spawn((event.game_session.clone(), GameSessionMarker));
        last_session_id.0 = Some(event.game_session.session_id);
        info!("[GamePlugin] Spawned new GameSession entity");
    }
}

/// Updates the GameSession entity when a GameSessionUpdate is received
fn update_gamesession_entity(
    mut commands: Commands,
    mut updates: MessageReader<GameSessionUpdate>,
    mut query: Query<&mut common::game::GameSession, With<GameSessionMarker>>,
    mut last_session_id: ResMut<LastGameSessionId>,
) {
    for update in updates.read() {
        info!("[GamePlugin] Listener: Received GameSessionUpdate: {:?}", update.game_session);
        if let Ok(mut session) = query.single_mut() {
            *session = update.game_session.clone();
        } else {
            commands.spawn((update.game_session.clone(), GameSessionMarker));
            info!("[GamePlugin] Spawned GameSession entity from update");
        }
        last_session_id.0 = Some(update.game_session.session_id);
    }
}
/// Spawns the Exit Game toolbar item when ServerState is InGame
fn spawn_exit_game_toolbar_item(
    mut commands: Commands,
) {

          commands.spawn((
                ToolbarItem {
                    name: GAME_BUTTON.to_string(),
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

/// Spawns the Pause/Resume toolbar item after Exit Game
fn spawn_pause_resume_toolbar_item(
    mut commands: Commands,
    session_query: Query<&common::game::GameSession, With<GameSessionMarker>>,
) {
    // Default to Pause if in-game, Resume if paused
    let (icon, label) = if let Ok(session) = session_query.single() {
        match session.state {
            GameState::Paused => ("\u{f04b}", "Resume"), // Play icon
            _ => ("\u{f04c}", "Pause"), // Pause icon
        }
    } else {
        ("\u{f04c}", "Pause")
    };
    commands.spawn((
        ToolbarItem {
            name: PAUSE_RESUME_BUTTON.to_string(),
            order: 2,
            icon: Some(icon.to_string()),
            state: ItemState::On,
            docking: Docking::Left,
            button_size: 36.0,
            ..Default::default()
        },
        PauseResumeButton,
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

/// Despawns the Pause/Resume toolbar item
fn despawn_pause_resume_toolbar_item(
    mut commands: Commands,
    query: Query<Entity, With<PauseResumeButton>>,
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
    last_session_id: Res<LastGameSessionId>,
) {

    for (toolbar_item, interaction) in &interaction_query {
        info!("Handling interaction for toolbar item: {}", toolbar_item.name);
        if toolbar_item.name == GAME_BUTTON && *interaction == Interaction::Pressed {
            info!("[GamePlugin] Exit Game button pressed");
            let session_id = session_query
                .single()
                .ok()
                .map(|session| session.session_id)
                .or(last_session_id.0);

            if let Some(session_id) = session_id {
                if let Some(connection) = client.get_connection_mut() {
                    let payload = common::network::NetworkMessage::ExitGameSession(session_id)
                        .to_bytes()
                        .expect("Failed to serialize ExitGameSession");
                    if let Err(e) = connection.send_payload(payload) {
                        error!("Failed to send ExitGameSession: {e}");
                    }
                }
            } else {
                warn!("[GamePlugin] Exit Game pressed, but no session id available");
            }
        }
    }
}

fn clear_last_session_id(mut last_session_id: ResMut<LastGameSessionId>) {
    last_session_id.0 = None;
}

fn clear_game_session_on_disconnect(
    mut commands: Commands,
    query: Query<Entity, With<GameSessionMarker>>,
    mut last_session_id: ResMut<LastGameSessionId>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    last_session_id.0 = None;
}

/// Handles Pause/Resume button presses and sends Pause/Resume messages to the server
fn handle_pause_resume_button(
    interaction_query: Query<(&ToolbarButton, &Interaction), Changed<Interaction>>,
    mut client: ResMut<bevy_quinnet::client::QuinnetClient>,
    session_query: Query<&common::game::GameSession, With<GameSessionMarker>>,
) {
    for (toolbar_item, interaction) in &interaction_query {
        if toolbar_item.name == PAUSE_RESUME_BUTTON && *interaction == Interaction::Pressed {
            info!("[GamePlugin] Pause/Resume button pressed");
            if let Ok(session) = session_query.single() {
                if let Some(connection) = client.get_connection_mut() {
                    let msg = match session.state {
                        GameState::Paused => common::network::NetworkMessage::ResumeGameSession(session.session_id),
                        _ => common::network::NetworkMessage::PauseGameSession(session.session_id),
                    };
                    let payload = msg.to_bytes().expect("Failed to serialize Pause/Resume message");
                    if let Err(e) = connection.send_payload(payload) {
                        error!("Failed to send Pause/Resume message: {e}");
                    }
                }
            }
        }
    }
}
