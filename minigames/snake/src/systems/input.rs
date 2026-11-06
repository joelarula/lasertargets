use bevy::prelude::*;
use common::calibration::GameKeyPressed;
use gamepad::{Btn, GamepadState, PrevGamepadState};

use crate::common::SnakePhase;
use crate::events::ChangeSnakeDirectionEvent;
use crate::rules::SnakeState;
use crate::types::SnakeDirection;

const STICK_DEADZONE: f32 = 0.35;

/// Arrow keys (from terminals or the server console, while calibration is off) steer
pub fn read_game_keys(
    mut keys: MessageReader<GameKeyPressed>,
    mut directions: MessageWriter<ChangeSnakeDirectionEvent>,
) {
    for key in keys.read() {
        if let Some(direction) = SnakeDirection::from_key(key.key) {
            directions.write(ChangeSnakeDirectionEvent { direction });
        }
    }
}

/// Left stick (held past the deadzone) and DPad presses steer; the DPad wins
pub fn read_gamepad(
    state: Option<Res<GamepadState>>,
    prev: Option<Res<PrevGamepadState>>,
    mut directions: MessageWriter<ChangeSnakeDirectionEvent>,
) {
    let (Some(state), Some(prev)) = (state, prev) else {
        return;
    };
    if !state.connected {
        return;
    }
    let dpad = [
        (Btn::DPadUp, SnakeDirection::Up),
        (Btn::DPadDown, SnakeDirection::Down),
        (Btn::DPadLeft, SnakeDirection::Left),
        (Btn::DPadRight, SnakeDirection::Right),
    ]
    .into_iter()
    .find(|(button, _)| state.just_pressed(&prev, *button))
    .map(|(_, direction)| direction);

    if let Some(direction) =
        dpad.or_else(|| stick_direction(state.left_stick_x, state.left_stick_y))
    {
        directions.write(ChangeSnakeDirectionEvent { direction });
    }
}

/// The stick's dominant axis, once it is past the deadzone
fn stick_direction(x: f32, y: f32) -> Option<SnakeDirection> {
    if x.abs().max(y.abs()) <= STICK_DEADZONE {
        None
    } else if y.abs() >= x.abs() {
        Some(if y > 0.0 {
            SnakeDirection::Up
        } else {
            SnakeDirection::Down
        })
    } else {
        Some(if x > 0.0 {
            SnakeDirection::Right
        } else {
            SnakeDirection::Left
        })
    }
}

/// Queues turns; the first accepted one starts the round
pub fn steer_snake(
    mut directions: MessageReader<ChangeSnakeDirectionEvent>,
    mut state: ResMut<SnakeState>,
    phase: Res<State<SnakePhase>>,
    mut next_phase: ResMut<NextState<SnakePhase>>,
) {
    for event in directions.read() {
        if state.steer(event.direction) && *phase.get() == SnakePhase::Waiting {
            next_phase.set(SnakePhase::Playing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_follows_the_dominant_axis_past_the_deadzone() {
        assert_eq!(stick_direction(0.2, 0.3), None);
        assert_eq!(stick_direction(0.1, 0.9), Some(SnakeDirection::Up));
        assert_eq!(stick_direction(0.5, -0.6), Some(SnakeDirection::Down));
        assert_eq!(stick_direction(-0.8, 0.4), Some(SnakeDirection::Left));
        assert_eq!(stick_direction(0.8, 0.4), Some(SnakeDirection::Right));
    }
}
