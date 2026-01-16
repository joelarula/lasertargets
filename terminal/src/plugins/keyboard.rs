use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use common::network::NetworkMessage;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct KeyboardSystemSet;

pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_keyboard_input.in_set(KeyboardSystemSet));
    }
}

fn handle_keyboard_input(
    mut client: ResMut<QuinnetClient>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut last_sent_keys: Local<Vec<KeyCode>>,
) {
    let mut current_pressed_keys = Vec::new();
    
    // Collect all currently pressed keys
    for key_code in keyboard_input.get_pressed() {
        current_pressed_keys.push(*key_code);
    }
    
    // Check for newly pressed keys
    for &key_code in &current_pressed_keys {
        if !last_sent_keys.contains(&key_code) {
            send_key_event(&mut client, key_code, true);
        }
    }
    
    // Check for newly released keys
    for &key_code in &*last_sent_keys {
        if !current_pressed_keys.contains(&key_code) {
            send_key_event(&mut client, key_code, false);
        }
    }
    
    // Update the last sent keys
    *last_sent_keys = current_pressed_keys;
}

fn send_key_event(client: &mut ResMut<QuinnetClient>, key_code: KeyCode, is_pressed: bool) {
    if let Some(connection) = client.get_connection_mut() {
        let key_name = format!("{:?}", key_code);
        let message = NetworkMessage::KeyboardInput {
            key: key_name,
            pressed: is_pressed,
        };
        
        let payload = message
            .to_bytes()
            .expect("Failed to serialize keyboard input");
            
        if let Err(e) = connection.send_payload(payload) {
            error!("Failed to send keyboard input: {}", e);
        } else {
            debug!("Sent keyboard event: {} = {}", format!("{:?}", key_code), is_pressed);
        }
    }
}