use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use common::network::NetworkMessage;
use crate::plugins::scene::SceneData;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MouseSystemSet;

pub struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_mouse_input.in_set(MouseSystemSet));
    }
}

fn handle_mouse_input(
    mut client: ResMut<QuinnetClient>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    scene_data: Res<SceneData>,
    mut last_sent_buttons: Local<Vec<MouseButton>>,
) {
    let mut current_pressed_buttons = Vec::new();
    
    // Get current world position from scene data
    let world_position = scene_data.mouse_world_pos;
    
    // Collect all currently pressed mouse buttons
    for button in mouse_button_input.get_pressed() {
        current_pressed_buttons.push(*button);
    }
    
    // Check for newly pressed buttons
    for &button in &current_pressed_buttons {
        if !last_sent_buttons.contains(&button) {
            send_mouse_event(&mut client, button, true, world_position);
        }
    }
    
    // Check for newly released buttons
    for &button in &*last_sent_buttons {
        if !current_pressed_buttons.contains(&button) {
            send_mouse_event(&mut client, button, false, world_position);
        }
    }
    
    // Update the last sent buttons
    *last_sent_buttons = current_pressed_buttons;
}

fn send_mouse_event(
    client: &mut ResMut<QuinnetClient>, 
    button: MouseButton, 
    is_pressed: bool,
    world_position: Option<Vec3>,
) {
    if let Some(connection) = client.get_connection_mut() {
        let button_name = format!("{:?}", button);
        let message = NetworkMessage::MouseButtonInput {
            button: button_name,
            pressed: is_pressed,
            position: world_position,
        };
        
        let payload = message
            .to_bytes()
            .expect("Failed to serialize mouse input");
            
        if let Err(e) = connection.send_payload(payload) {
            error!("Failed to send mouse input: {}", e);
        } else {
            debug!("Sent mouse event: {} = {} at world pos {:?}", 
                   format!("{:?}", button), 
                   is_pressed, 
                   world_position);
        }
    }
}