use bevy::prelude::*;
use common::network::MouseButtonInputEvent;
use common::state::CalibrationState;
use gamepad::{Btn, GamepadState, PrevGamepadState, ServerGamepadCursor};

use crate::audit::ShotSource;
use crate::events::{HunterClickEvent, SpawnHunterTargetEvent};
use crate::resources::HunterTargetSelection;

/// Trigger at `at`: release the carried target there, or shoot
fn trigger(
    selection: &mut HunterTargetSelection,
    at: Vec3,
    source: ShotSource,
    spawns: &mut MessageWriter<SpawnHunterTargetEvent>,
    shots: &mut MessageWriter<HunterClickEvent>,
) {
    match selection.release() {
        Some(target) => {
            info!("[Hunter] {source:?} released {target:?} at {at}");
            spawns.write(SpawnHunterTargetEvent { target, position: at });
        }
        None => {
            shots.write(HunterClickEvent { source, click_position: at });
        }
    }
}

/// Gamepad: A cycles the carried target, LB/RB resize it, B or RT releases it or shoots.
/// While calibrating, the buttons calibration uses (A, RB, RT) are its own; LB and B still work.
pub fn read_gamepad(
    state: Option<Res<GamepadState>>,
    prev: Option<Res<PrevGamepadState>>,
    cursor: Option<Res<ServerGamepadCursor>>,
    calibration: Option<Res<State<CalibrationState>>>,
    mut selection: ResMut<HunterTargetSelection>,
    mut spawns: MessageWriter<SpawnHunterTargetEvent>,
    mut shots: MessageWriter<HunterClickEvent>,
) {
    let (Some(state), Some(prev), Some(cursor)) = (state, prev, cursor) else { return };
    if !state.connected {
        return;
    }
    let calibrating = calibration.is_some_and(|c| *c.get() == CalibrationState::On);
    let pressed = |btn: Btn| btn.is_for_game(calibrating) && state.just_pressed(&prev, btn);

    if pressed(Btn::South) {
        selection.cycle();
    }
    if pressed(Btn::LeftBumper) {
        selection.resize(-1.0);
    }
    if pressed(Btn::RightBumper) {
        selection.resize(1.0);
    }
    if [Btn::South, Btn::LeftBumper, Btn::RightBumper].into_iter().any(|btn| pressed(btn)) {
        info!("🎮 [Hunter] Reticle: {}", selection.name());
    }
    if pressed(Btn::East) || pressed(Btn::RightTrigger) {
        trigger(&mut selection, cursor.position, ShotSource::Gamepad, &mut spawns, &mut shots);
    }
}

/// Mouse: right click cycles the carried target, left click releases it or shoots
pub fn read_mouse(
    mut clicks: MessageReader<MouseButtonInputEvent>,
    mut selection: ResMut<HunterTargetSelection>,
    mut spawns: MessageWriter<SpawnHunterTargetEvent>,
    mut shots: MessageWriter<HunterClickEvent>,
) {
    for click in clicks.read().filter(|c| c.pressed) {
        let Some(at) = click.position else { continue };
        match click.button.as_str() {
            "Right" => {
                selection.cycle();
                info!("🖱️ [Hunter] Reticle: {}", selection.name());
            }
            "Left" => trigger(&mut selection, at, ShotSource::Mouse, &mut spawns, &mut shots),
            _ => {}
        }
    }
}
