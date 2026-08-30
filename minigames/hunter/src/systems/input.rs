use bevy::prelude::*;
use common::game::GameSession;
use common::state::GameState;
use gamepad::{Btn, GamepadState, PrevGamepadState, ServerGamepadCursor};

use crate::common::GAME_ID;
use crate::events::{HunterClickEvent, SpawnHunterTargetEvent};
use crate::resources::HunterTargetSelection;

/// Handles gamepad input during Hunter game:
/// - Button A (South): Roll / cycle through target types
/// - Button B (East) or RT: Release selected target or shoot
/// - LB / RB: Adjust target size
pub fn handle_hunter_gamepad_inputs(
    state: Option<Res<GamepadState>>,
    prev: Option<Res<PrevGamepadState>>,
    cursor: Option<Res<ServerGamepadCursor>>,
    mut selection: ResMut<HunterTargetSelection>,
    game_sessions: Query<&GameSession>,
    mut click_events: MessageWriter<HunterClickEvent>,
    mut spawn_events: MessageWriter<SpawnHunterTargetEvent>,
) {
    let (Some(state), Some(prev), Some(cursor)) = (state, prev, cursor) else { return; };
    if !state.connected { return; }

    let Some(active_session) = game_sessions.iter().find(|s| s.game_id == GAME_ID && s.state == GameState::InGame) else { return; };

    // Button A (South) -> Roll / Cycle reticle mode
    if state.just_pressed(&prev, Btn::South) {
        selection.cycle();
        info!("🎮 [Hunter Mode Switch] Selected cursor mode #{}: {}", selection.selected_index, selection.target_name());
    }

    // LeftBumper (LB) -> Decrease active target radius (-0.05m)
    if state.just_pressed(&prev, Btn::LeftBumper) {
        selection.decrease_size();
        info!("🎮 [Hunter Target Size] Decreased target size: {}", selection.target_name());
    }

    // RightBumper (RB) -> Increase active target radius (+0.05m)
    if state.just_pressed(&prev, Btn::RightBumper) {
        selection.increase_size();
        info!("🎮 [Hunter Target Size] Increased target size: {}", selection.target_name());
    }

    // Button B (East) or Right Trigger (RT) -> Release target or Shoot
    if state.just_pressed(&prev, Btn::East) || state.just_pressed(&prev, Btn::RightTrigger) {
        let click_pos = cursor.position;

        if let Some(target_to_spawn) = selection.get_target() {
            info!("🚀 [Hunter Gamepad] RELEASING target [{}] at {:?}", selection.target_name(), click_pos);
            spawn_events.write(SpawnHunterTargetEvent {
                target: target_to_spawn,
                position: click_pos,
            });
            selection.reset_to_gunshot();
            info!("🎯 [Hunter Gamepad] Reticle cursor mode auto-reset to GunShot Mode");
        } else {
            info!("🎯 [Hunter Gamepad] SHOOTING at {:?}", click_pos);
            click_events.write(HunterClickEvent {
                session_id: active_session.session_id,
                click_position: click_pos,
            });
        }
    }
}
