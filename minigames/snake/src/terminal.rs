use bevy::app::{App, Plugin, Update};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::{NextState, OnEnter, OnExit, State};
use bevy::ui::Interaction;
use bevy_quinnet::client::QuinnetClient;
use common::network::NetworkMessage;
use common::state::{GameState, ServerState, TerminalState};
use common::toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem};

use crate::common::SnakeGameState;
use crate::model::{SnakeGameStats, GAME_ID};

const START_SNAKE_BTN: &str = "start_snake_game";

#[derive(Component)]
struct SnakeMenuButton;

#[derive(Component)]
struct SnakeStatsDisplay;

pub struct SnakeTerminalPlugin;

impl Plugin for SnakeTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Menu), spawn_snake_menu_button);
        app.add_systems(OnExit(ServerState::Menu), despawn_snake_menu_button);
        app.add_systems(Update, handle_snake_start_button);
        app.add_systems(OnEnter(SnakeGameState::On), spawn_snake_stats_ui);
        app.add_systems(OnExit(SnakeGameState::On), cleanup_snake_ui);
        app.add_systems(OnEnter(ServerState::Menu), cleanup_snake_state_on_menu);
        app.add_systems(
            Update,
            (handle_snake_keyboard_input, update_snake_stats_display)
                .run_if(in_state(SnakeGameState::On)),
        );
        app.add_systems(
            Update,
            handle_snake_escape.run_if(in_state(SnakeGameState::On)),
        );
        app.add_systems(OnEnter(TerminalState::Connecting), clear_snake_on_disconnect);
    }
}

fn clear_snake_on_disconnect(
    mut commands: Commands,
    stats: Option<Res<SnakeGameStats>>,
    query: Query<Entity, With<SnakeStatsDisplay>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    if stats.is_some() {
        commands.remove_resource::<SnakeGameStats>();
    }
}

// ---------------------------------------------------------------------------
// Menu button
// ---------------------------------------------------------------------------

fn spawn_snake_menu_button(mut commands: Commands) {
    commands.spawn((
        SnakeMenuButton,
        ToolbarItem {
            name: START_SNAKE_BTN.to_string(),
            order: 2,
            icon: Some("\u{f25a}".to_string()), // snake-ish icon (gamepad)
            state: ItemState::On,
            docking: Docking::Left,
            ..default()
        },
    ));
}

fn despawn_snake_menu_button(
    mut commands: Commands,
    query: Query<Entity, With<SnakeMenuButton>>,
    mut next_state: ResMut<NextState<SnakeGameState>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    next_state.set(SnakeGameState::Off);
}

fn handle_snake_start_button(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    for (interaction, toolbar_button) in &button_query {
        if toolbar_button.name == START_SNAKE_BTN && *interaction == Interaction::Pressed {
            log::info!("'Start Snake Game' button pressed");
            if *terminal_state.get() == TerminalState::Connected {
                if let Some(connection) = client.get_connection_mut() {
                    let game_uuid = bevy::asset::uuid::Uuid::new_v4();
                    let message =
                        NetworkMessage::InitGameSession(game_uuid, GAME_ID, GameState::Paused);
                    if let Ok(payload) = message.to_bytes() {
                        if let Err(e) = connection.send_payload(payload) {
                            bevy::log::warn!("Failed to send init Snake game message: {:?}", e);
                        } else {
                            bevy::log::info!(
                                "Sent init Snake game message (UUID: {})",
                                game_uuid
                            );
                        }
                    }
                }
            } else {
                bevy::log::warn!("Cannot start snake game: not connected to server");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Keyboard → direction change via network
// ---------------------------------------------------------------------------

fn handle_snake_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    // Arrow keys send KeyboardInput messages to server
    // Server's network plugin already handles KeyboardInput → ChangeSnakeDirectionEvent
    // but we also handle it locally for responsiveness display.
    // The existing keyboard plugin already sends all key events to the server,
    // so we don't need to send again here. This system is a no-op placeholder.
    // Direction mapping is handled server-side.
    let _ = (keyboard_input, client, terminal_state);
}

// ---------------------------------------------------------------------------
// Escape to exit game
// ---------------------------------------------------------------------------

fn handle_snake_escape(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
    snake_stats: Option<Res<SnakeGameStats>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        log::info!("Escape pressed during snake game — requesting exit");
        if *terminal_state.get() == TerminalState::Connected {
            if let Some(connection) = client.get_connection_mut() {
                // Try to get session_id from stats
                let session_id = snake_stats
                    .map(|s| s.session_id)
                    .unwrap_or_else(bevy::asset::uuid::Uuid::new_v4);
                let message = NetworkMessage::ExitGameSession(session_id);
                if let Ok(payload) = message.to_bytes() {
                    let _ = connection.send_payload(payload);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Stats UI
// ---------------------------------------------------------------------------

fn spawn_snake_stats_ui(mut commands: Commands) {
    commands.spawn((
        SnakeStatsDisplay,
        Text::new("Snake | Score: 0 | Length: 3"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(5.0),
            ..default()
        },
        ZIndex(1000),
    ));
}

fn update_snake_stats_display(
    stats: Option<Res<SnakeGameStats>>,
    mut query: Query<&mut Text, With<SnakeStatsDisplay>>,
) {
    let Some(stats) = stats else { return };
    if stats.is_changed() {
        if let Ok(mut text) = query.single_mut() {
            if stats.game_over {
                **text = format!(
                    "GAME OVER | Score: {} | Length: {}",
                    stats.score, stats.length
                );
            } else {
                **text = format!(
                    "Snake | Score: {} | Length: {}",
                    stats.score, stats.length
                );
            }
        }
    }
}

fn cleanup_snake_ui(
    mut commands: Commands,
    query: Query<Entity, With<SnakeStatsDisplay>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn cleanup_snake_state_on_menu(
    mut commands: Commands,
    stats: Option<Res<SnakeGameStats>>,
    query: Query<Entity, With<SnakeStatsDisplay>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    if stats.is_some() {
        commands.remove_resource::<SnakeGameStats>();
    }
}
