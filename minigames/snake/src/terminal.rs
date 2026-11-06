use bevy::prelude::*;
use common::curator_core::CuratorEventEnvelope;
use common::network::{NetworkMessage, ToServer};
use common::state::{ServerState, TerminalState};
use terminal_ui::{ToolbarItem, ToolbarPressed};

use crate::common::SnakeGameState;
use crate::model::{BroadcastSnakeStatsEvent, SnakeGameStats};

#[derive(Component)]
struct SnakeStatsDisplay;

/// Terminal side of Snake: the start button, the stats line and Escape to leave. Steering keys
/// need nothing here: the terminal forwards every key and the server routes them to the game.
pub struct SnakeTerminalPlugin;

impl Plugin for SnakeTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(ServerState::Menu), (spawn_snake_menu_button, clear_snake_stats))
            .add_systems(OnEnter(TerminalState::Connecting), clear_snake_stats)
            .add_systems(OnEnter(SnakeGameState::On), spawn_snake_stats_ui)
            .add_systems(Update, receive_snake_stats)
            .add_systems(
                Update,
                (
                    update_snake_stats_display.run_if(resource_exists_and_changed::<SnakeGameStats>),
                    handle_snake_escape,
                )
                    .run_if(in_state(SnakeGameState::On)),
            );
    }
}

/// Stats belong to the game that was running; drop them with its display
fn clear_snake_stats(mut commands: Commands, displays: Query<Entity, With<SnakeStatsDisplay>>) {
    for entity in &displays {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<SnakeGameStats>();
}

/// The server sends score and length as a `snake_stats` curator envelope
fn receive_snake_stats(mut envelopes: MessageReader<CuratorEventEnvelope>, mut commands: Commands) {
    for envelope in envelopes.read() {
        let Ok(event) = envelope.downcast::<BroadcastSnakeStatsEvent>() else { continue };
        commands.insert_resource(SnakeGameStats {
            session_id: envelope.session_id.unwrap_or(event.session_id),
            score: event.score,
            length: event.length,
            game_over: event.game_over,
        });
    }
}

// ---------------------------------------------------------------------------
// Menu button
// ---------------------------------------------------------------------------

fn spawn_snake_menu_button(mut commands: Commands) {
    commands
        .spawn((
            // Gamepad icon
            ToolbarItem::new("start_snake_game").icon("\u{f25a}").order(2).on(true),
            DespawnOnExit(ServerState::Menu),
        ))
        .observe(|_: On<ToolbarPressed>, mut to_server: MessageWriter<ToServer>| {
            let session_id = bevy::asset::uuid::Uuid::new_v4();
            info!("Opening Snake session {session_id}");
            to_server.write(ToServer(NetworkMessage::OpenSession { session_id, channel_name: "snake".to_string() }));
        });
}

// ---------------------------------------------------------------------------
// Escape to exit game
// ---------------------------------------------------------------------------

fn handle_snake_escape(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    stats: Option<Res<SnakeGameStats>>,
    mut to_server: MessageWriter<ToServer>,
) {
    if !keyboard_input.just_pressed(KeyCode::Escape) {
        return;
    }
    // The session id arrives with the first stats; without it there is nothing to close
    let Some(stats) = stats else {
        warn!("Escape pressed during snake game, but its session is not known yet");
        return;
    };
    info!("Escape pressed during snake game — requesting exit");
    to_server.write(ToServer(NetworkMessage::CloseSession { session_id: stats.session_id }));
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
        DespawnOnExit(SnakeGameState::On),
    ));
}

fn update_snake_stats_display(
    stats: Res<SnakeGameStats>,
    mut text: Single<&mut Text, With<SnakeStatsDisplay>>,
) {
    let title = if stats.game_over {
        "GAME OVER"
    } else {
        "Snake"
    };
    text.0 = format!(
        "{} | Score: {} | Length: {}",
        title, stats.score, stats.length
    );
}
