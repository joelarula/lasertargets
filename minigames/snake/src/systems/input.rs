use bevy::prelude::*;
use gamepad::{Btn, GamepadState, PrevGamepadState};

use crate::events::ChangeSnakeDirectionEvent;
use crate::resources::SnakeState;
use crate::types::SnakeDirection;

/// Handle direction input from gamepad thumbstick and DPad
pub fn handle_snake_gamepad_inputs(
    state: Option<Res<GamepadState>>,
    prev: Option<Res<PrevGamepadState>>,
    mut dir_events: MessageWriter<ChangeSnakeDirectionEvent>,
) {
    let (Some(state), Some(prev)) = (state, prev) else { return; };
    if !state.connected { return; }

    let mut desired_dir = None;

    // Check Left Thumbstick Movement (with deadzone)
    let lx = state.left_stick_x;
    let ly = state.left_stick_y;
    let deadzone = 0.35;

    if ly > deadzone && ly.abs() >= lx.abs() {
        desired_dir = Some(SnakeDirection::Up);
    } else if ly < -deadzone && ly.abs() >= lx.abs() {
        desired_dir = Some(SnakeDirection::Down);
    } else if lx < -deadzone && lx.abs() >= ly.abs() {
        desired_dir = Some(SnakeDirection::Left);
    } else if lx > deadzone && lx.abs() >= ly.abs() {
        desired_dir = Some(SnakeDirection::Right);
    }

    // Check DPad buttons (just_pressed)
    if state.just_pressed(&prev, Btn::DPadUp) {
        desired_dir = Some(SnakeDirection::Up);
    } else if state.just_pressed(&prev, Btn::DPadDown) {
        desired_dir = Some(SnakeDirection::Down);
    } else if state.just_pressed(&prev, Btn::DPadLeft) {
        desired_dir = Some(SnakeDirection::Left);
    } else if state.just_pressed(&prev, Btn::DPadRight) {
        desired_dir = Some(SnakeDirection::Right);
    }

    if let Some(dir) = desired_dir {
        dir_events.write(ChangeSnakeDirectionEvent { direction: dir });
    }
}

/// Handle direction change events from keyboard input
pub fn handle_direction_input(
    mut direction_events: MessageReader<ChangeSnakeDirectionEvent>,
    mut snake_state: Option<ResMut<SnakeState>>,
) {
    let Some(ref mut state) = snake_state else {
        return;
    };
    if state.game_over {
        return;
    }
    for event in direction_events.read() {
        let new_dir = event.direction;
        // Don't allow reversing into yourself
        if !new_dir.is_opposite(state.direction) {
            state.queued_direction = Some(new_dir);
            state.is_started = true;
        }
    }
}
