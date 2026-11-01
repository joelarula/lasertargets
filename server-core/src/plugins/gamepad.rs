use bevy::prelude::*;
use common::config::SceneConfiguration;
use gamepad::{GamepadBasePlugin, GamepadButtonEvent, GamepadState, ServerGamepadCursor, GAMEPAD_STICK_DEADZONE, SERVER_GAMEPAD_CLIENT_ID};
use common::actor::ActorLink;
use crate::plugins::network::MousePositionEvent;

/// App-specific plugin registering gamepad event handlers for LaserTargets server.
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        // Add core hardware gamepad polling plugin from gamepad utility crate
        app.add_plugins(GamepadBasePlugin);

        // Register Server Gamepad as direct Actor on startup
        app.add_systems(Startup, register_server_gamepad_actor);

        // Register virtual cursor navigation and debug button logging
        app.add_systems(Update, (
            gamepad_cursor_movement,
            log_gamepad_buttons,
        ));
    }
}

/// Spawns an ActorLink entity for the server gamepad so it is recognized as a direct Actor.
fn register_server_gamepad_actor(mut commands: Commands) {
    let actor = common::actor::Actor {
        name: "Server-Gamepad".to_string(),
        uuid: bevy::asset::uuid::Uuid::from_u128(0x53455256_4741_4d45_5041_443030303030),
        roles: vec!["Server".to_string(), "Controller".to_string(), "Player".to_string()],
    };
    let actor_link = ActorLink {
        client_id: SERVER_GAMEPAD_CLIENT_ID,
        actor: actor.clone(),
    };
    commands.spawn((actor, actor_link));
    info!("✓ Registered Server Gamepad as direct Actor (Client ID {})", SERVER_GAMEPAD_CLIENT_ID);
}

/// Updates the virtual mouse/cursor position based on left thumbstick input and emits MousePositionEvent.
fn gamepad_cursor_movement(
    state: Res<GamepadState>,
    time: Res<Time>,
    scene_config: Res<SceneConfiguration>,
    mut cursor: ResMut<ServerGamepadCursor>,
    mut mouse_events: MessageWriter<MousePositionEvent>,
) {
    if !state.connected { return; }

    let dt = time.delta_secs();
    let move_speed = cursor.sensitivity;

    // Initialize cursor position to scene origin center if uninitialized
    if cursor.position == Vec3::ZERO {
        cursor.position = scene_config.origin.translation;
    }

    if state.left_stick_x.abs() > GAMEPAD_STICK_DEADZONE || state.left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
        cursor.position.x += state.left_stick_x * move_speed * dt;
        cursor.position.y += state.left_stick_y * move_speed * dt;
    }

    let half_w = (scene_config.scene_dimension.x as f32 / 2.0).max(0.5);
    let half_h = (scene_config.scene_dimension.y as f32 / 2.0).max(0.5);

    let min_x = scene_config.origin.translation.x - half_w;
    let max_x = scene_config.origin.translation.x + half_w;
    let min_y = scene_config.origin.translation.y - half_h;
    let max_y = scene_config.origin.translation.y + half_h;

    cursor.position.x = cursor.position.x.clamp(min_x, max_x);
    cursor.position.y = cursor.position.y.clamp(min_y, max_y);
    cursor.position.z = scene_config.origin.translation.z;

    // Always emit current cursor position for calibration crosshairs and game aim
    mouse_events.write(MousePositionEvent {
        client_id: SERVER_GAMEPAD_CLIENT_ID,
        position: Some(cursor.position),
    });
}

// --- Debug logging ---

fn log_gamepad_buttons(
    mut button_events: MessageReader<GamepadButtonEvent>,
    state: Res<GamepadState>,
) {
    for event in button_events.read() {
        if event.pressed {
            info!("Gamepad button PRESSED: {:?}", event.button);
        } else {
            debug!("Gamepad button RELEASED: {:?}", event.button);
        }
    }

    if state.left_stick_x.abs() > 0.5 || state.left_stick_y.abs() > 0.5 {
        debug!("Gamepad Left stick: ({:.2}, {:.2})", state.left_stick_x, state.left_stick_y);
    }
    if state.right_stick_x.abs() > 0.5 || state.right_stick_y.abs() > 0.5 {
        debug!("Gamepad Right stick: ({:.2}, {:.2})", state.right_stick_x, state.right_stick_y);
    }
}
