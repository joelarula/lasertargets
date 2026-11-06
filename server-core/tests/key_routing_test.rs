//! Keys forwarded by terminals: calibration gets them while it is on, the running game otherwise.
//! The calibration and laser toggles always go to calibration.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_quinnet::server::ConnectionLostEvent;
use common::calibration::{CalibrationAction, ControlKey, GameKeyPressed};
use common::network::KeyboardInputEvent;
use common::state::CalibrationState;
use server_core::plugins::keyboard::KeyboardControlPlugin;
use test_harness::TestApp;

fn app(calibration: CalibrationState) -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<CalibrationState>()
        .add_message::<KeyboardInputEvent>()
        .add_message::<CalibrationAction>()
        .add_message::<ConnectionLostEvent>();
    t.add_plugins(KeyboardControlPlugin);
    t.capture::<CalibrationAction>().capture::<GameKeyPressed>();
    t.set_state(calibration).step();
    t
}

fn key(t: &mut TestApp, key: &str, pressed: bool) {
    t.send(KeyboardInputEvent { client_id: 3, key: key.into(), pressed }).step();
}

#[test]
fn arrows_nudge_calibration_while_it_is_on() {
    let mut t = app(CalibrationState::On);
    key(&mut t, "ArrowUp", true);
    assert_eq!(t.captured::<CalibrationAction>(), [CalibrationAction::Nudge { dir: IVec2::Y, fine: false }]);
    assert!(t.captured::<GameKeyPressed>().is_empty());
}

#[test]
fn arrows_go_to_the_game_once_per_press_while_calibration_is_off() {
    let mut t = app(CalibrationState::Off);
    key(&mut t, "ArrowUp", true);
    t.advance(Duration::from_secs(1));
    key(&mut t, "ArrowUp", false);
    key(&mut t, "Space", true);

    assert!(t.captured::<CalibrationAction>().is_empty());
    assert_eq!(
        t.captured::<GameKeyPressed>(),
        [
            GameKeyPressed { client_id: 3, key: ControlKey::Up },
            GameKeyPressed { client_id: 3, key: ControlKey::Space },
        ],
        "held arrows don't repeat for games"
    );
}

#[test]
fn toggles_always_reach_calibration() {
    let mut t = app(CalibrationState::Off);
    key(&mut t, "KeyC", true);
    key(&mut t, "F1", true);
    assert_eq!(t.captured::<CalibrationAction>(), [CalibrationAction::CycleCalibration, CalibrationAction::ToggleLaser]);
    assert!(t.captured::<GameKeyPressed>().is_empty());
}
