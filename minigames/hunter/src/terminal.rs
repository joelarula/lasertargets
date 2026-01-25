#[derive(Component)]
struct BasictargetToolbarMarker;
use bevy::{app::{App, Plugin}, ecs::{component::Component, entity::Entity, query::{Changed, With}, system::{Commands, Query, Res, ResMut}}, prelude::default, state::{app::AppExtStates, state::{OnEnter, OnExit, State}}, ui::Interaction};
use bevy_quinnet::client::QuinnetClient;
use common::{network::NetworkMessage, state::{GameState, ServerState, TerminalState}, toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem}};
use crate::common::{GAME_ID, HunterGameState};

const BTN_NAME: &str = "start_hunter_game";

#[derive(Component)]
struct MenuButton;

#[derive(Component)]
struct BasicTargetButton;

pub struct HunterTerminalPlugin;

impl Plugin for HunterTerminalPlugin {
    fn build(&self, app: &mut App) {      
        app.add_systems(OnEnter(ServerState::Menu), spawn_menu_toolbar);
        app.add_systems(OnExit(ServerState::Menu), despawn_menu_toolbar);
        app.add_systems(OnEnter(HunterGameState::On), spawn_basictarget_toolbar_item);
        app.add_systems(OnEnter(ServerState::Menu), despawn_basictarget_toolbar_item);
     
        app.add_systems(bevy::prelude::Update, handle_button_click);
    }
   
}

/// Spawns the 'basictarget' toolbar item when entering HunterGameState::On
fn spawn_basictarget_toolbar_item(mut commands: Commands) {
    commands.spawn((
        ToolbarItem {
            name: "basictarget".to_string(),
            order: 10,
            icon: Some("\u{f140}".to_string()), // Target/crosshairs icon
            state: ItemState::On,
            docking: Docking::Bottom,
            button_size: 36.0,
            ..default()
        },
        BasicTargetButton,
    ));
}

/// Despawns the 'basictarget' toolbar item when exiting HunterGameState::On
fn despawn_basictarget_toolbar_item(
    mut commands: Commands,
    query: Query<Entity, With<BasicTargetButton>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}


fn spawn_menu_toolbar(mut commands: Commands) {
    commands.spawn((
        MenuButton,
        ToolbarItem {
            name: BTN_NAME.to_string(),
            order: 1,
            icon: Some("\u{f140}".to_string()), // Target/crosshairs icon
            state: ItemState::On,
            docking: Docking::Left,
            ..default()
        },
    ));
}


fn despawn_menu_toolbar(
    mut commands: Commands,
    button_query: Query<Entity, With<MenuButton>>,
) {
    for entity in &button_query {
        commands.entity(entity).despawn();
    }
}

fn handle_button_click(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    for (interaction, toolbar_button) in &button_query {
            if toolbar_button.name == BTN_NAME && *interaction == Interaction::Pressed {
            log::info!("'Start Hunter Game' button pressed");
            if *terminal_state.get() == TerminalState::Connected {
                if let Some(connection) = client.get_connection_mut() {
                    // Initialize a Hunter game session with a new UUID and game ID
                    let game_uuid = bevy::asset::uuid::Uuid::new_v4();
                    let message = NetworkMessage::InitGameSession(game_uuid, GAME_ID, GameState::Paused);

                    if let Ok(payload) = message.to_bytes() {
                        if let Err(e) = connection.send_payload(payload) {
                            bevy::log::warn!("Failed to send init Hunter game message: {:?}", e);
                        } else {
                            bevy::log::info!("Sent init Hunter game message (UUID: {}, Name: Hunter)", game_uuid);
                        }
                    }
                }
            } else {
                bevy::log::warn!("Cannot start game: not connected to server");
            }
        }
    }
}