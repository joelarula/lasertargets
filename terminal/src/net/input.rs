//! Forwards every key and mouse button, and the scene point under the cursor, to the server,
//! which routes them to the projector, calibration and the running game.

use bevy::prelude::*;
use common::network::{NetworkMessage, ToServer};
use common::scene::SceneData;
use common::state::TerminalState;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (forward_keys, forward_mouse).run_if(in_state(TerminalState::Connected)));
    }
}

fn forward_keys(keys: Res<ButtonInput<KeyCode>>, mut to_server: MessageWriter<ToServer>) {
    let pressed = keys.get_just_pressed().map(|key| (key, true));
    let released = keys.get_just_released().map(|key| (key, false));
    to_server.write_batch(pressed.chain(released).map(|(key, pressed)| {
        ToServer(NetworkMessage::KeyboardInput { key: format!("{key:?}"), pressed })
    }));
}

fn forward_mouse(
    buttons: Res<ButtonInput<MouseButton>>,
    scene: Res<SceneData>,
    mut last_position: Local<Option<Vec3>>,
    mut to_server: MessageWriter<ToServer>,
) {
    let position = scene.mouse_world_pos;
    if *last_position != position {
        *last_position = position;
        to_server.write(ToServer(NetworkMessage::UpdateMousePosition(position)));
    }
    let pressed = buttons.get_just_pressed().map(|button| (button, true));
    let released = buttons.get_just_released().map(|button| (button, false));
    to_server.write_batch(pressed.chain(released).map(|(button, pressed)| {
        ToServer(NetworkMessage::MouseButtonInput { button: format!("{button:?}"), pressed, position })
    }));
}
