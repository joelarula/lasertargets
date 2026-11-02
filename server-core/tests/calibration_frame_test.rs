//! The real calibration overlays (red DAC perimeter, green scene box, yellow crosshair) rendered
//! through the projector pipeline.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy_quinnet::server::ConnectionLostEvent;
use common::config::CameraConfiguration;
use common::network::MousePositionEvent;
use common::scene::SceneSetupPlugin;
use common::state::CalibrationState;
use gamepad::{GamepadButtonEvent, GamepadState, PrevGamepadState};
use laserlogic::helios::HeliosPoint;
use server_core::plugins::calibration::CalibrationPlugin;
use server_core::plugins::projector::{LaserPointBuffer, ProjectorDacController, ProjectorPlugin};
use server_core::plugins::scene::ScenePlugin;
use test_harness::TestApp;

fn calibration_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<CalibrationState>()
        .add_message::<GamepadButtonEvent>()
        .add_message::<MousePositionEvent>()
        .add_message::<ConnectionLostEvent>()
        .init_resource::<GamepadState>()
        .init_resource::<PrevGamepadState>();
    t.insert_resource(CameraConfiguration::default())
        .add_plugins((SceneSetupPlugin, ScenePlugin, CalibrationPlugin, ProjectorPlugin))
        .insert_resource(ProjectorDacController { enabled: false, ..Default::default() });
    t
}

fn frame() -> Vec<HeliosPoint> {
    let mut t = calibration_app();
    t.step_n(5);
    let points = t.resource::<LaserPointBuffer>().points.lock().unwrap().clone();
    points
}

fn is_lit(p: &HeliosPoint) -> bool {
    p.r != 0 || p.g != 0 || p.b != 0
}

#[test]
#[ignore = "diagnostic dump"]
fn dump_calibration_frame() {
    let points = frame();
    println!("frame length {}", points.len());
    // Run-length encode: position + colour
    let mut runs: Vec<(HeliosPoint, usize)> = Vec::new();
    for p in points {
        match runs.last_mut() {
            Some((q, n)) if q.x == p.x && q.y == p.y && (q.r, q.g, q.b, q.i) == (p.r, p.g, p.b, p.i) => *n += 1,
            _ => runs.push((p, 1)),
        }
    }
    for (p, n) in runs {
        println!("{:5} {:5}  rgb({:3},{:3},{:3}) i{:3} {} x{}", p.x, p.y, p.r, p.g, p.b, p.i, if is_lit(&p) { "LIT" } else { "   " }, n);
    }
}
