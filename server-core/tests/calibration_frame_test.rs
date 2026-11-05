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

use common::calibration::{CalibrationAction, CalibrationOverlays};
use common::path::UniversalPath;
use server_core::plugins::calibration::{CalibrationCenterCrosshair, CenterShape, CenterShapes};

fn has_colour(points: &[HeliosPoint], rgb: (u8, u8, u8)) -> bool {
    points.iter().any(|p| (p.r, p.g, p.b) == rgb)
}

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 255, 0);
const YELLOW: (u8, u8, u8) = (255, 255, 0);

fn buffer(t: &TestApp) -> Vec<HeliosPoint> {
    t.resource::<LaserPointBuffer>().points.lock().unwrap().clone()
}

#[test]
fn calibration_key_turns_overlays_off_one_by_one_then_all_on() {
    let mut t = calibration_app();
    t.step_n(5);
    let frame = buffer(&t);
    assert!(has_colour(&frame, RED) && has_colour(&frame, GREEN) && has_colour(&frame, YELLOW));

    t.send(CalibrationAction::CycleCalibration).step_n(3);
    let frame = buffer(&t);
    assert!(!has_colour(&frame, RED), "red DAC perimeter off first");
    assert!(has_colour(&frame, GREEN) && has_colour(&frame, YELLOW));

    t.send(CalibrationAction::CycleCalibration).step_n(3);
    let frame = buffer(&t);
    assert!(!has_colour(&frame, RED) && !has_colour(&frame, GREEN), "green scene box off second");
    assert!(has_colour(&frame, YELLOW));

    t.send(CalibrationAction::CycleCalibration).step_n(3);
    assert_eq!(*t.resource::<CalibrationOverlays>(), CalibrationOverlays::Off);
    assert_eq!(*t.resource::<State<CalibrationState>>().get(), CalibrationState::Off);
    assert!(buffer(&t).iter().all(|p| (p.r, p.g, p.b) == (0, 0, 0)), "all off third");

    t.send(CalibrationAction::CycleCalibration).step_n(5);
    assert_eq!(*t.resource::<State<CalibrationState>>().get(), CalibrationState::On);
    let frame = buffer(&t);
    assert!(has_colour(&frame, RED) && has_colour(&frame, GREEN) && has_colour(&frame, YELLOW), "all on again");
}

fn center_path(t: &mut TestApp) -> UniversalPath {
    let mut query = t.world_mut().query_filtered::<&UniversalPath, With<CalibrationCenterCrosshair>>();
    query.single(t.world()).unwrap().clone()
}

#[test]
fn shape_key_cycles_centre_shapes_and_all_on_returns_to_crosshair() {
    let mut t = calibration_app();
    t.step_n(5);
    let crosshair = center_path(&mut t);
    let names: Vec<String> = t.resource::<CenterShapes>().0.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(names[0], "crosshair");
    assert_eq!(names.len(), 10, "crosshair + 9 v2 templates: {:?}", names);

    t.send(CalibrationAction::NextShape).step_n(2);
    assert_eq!(t.resource::<CenterShape>().0, 1);
    assert_ne!(center_path(&mut t), crosshair);

    // Through all off and back to all on: the crosshair returns
    for _ in 0..4 {
        t.send(CalibrationAction::CycleCalibration).step_n(3);
    }
    t.step_n(2);
    assert_eq!(t.resource::<CenterShape>().0, 0);
    assert_eq!(center_path(&mut t), crosshair);

    // Wraps around after the last shape
    for _ in 0..names.len() {
        t.send(CalibrationAction::NextShape).step();
    }
    assert_eq!(t.resource::<CenterShape>().0, 0);
}

#[test]
fn mouse_crosshair_stays_on_scene_plane_when_scene_moves() {
    use common::config::SceneConfiguration;
    use server_core::plugins::calibration::CalibrationCrosshair;

    let mut t = calibration_app();
    t.step_n(3);
    let on_plane = SceneConfiguration::default().origin.translation + Vec3::new(1.0, 0.5, 0.0);
    t.send(MousePositionEvent { client_id: 7, position: Some(on_plane) }).step_n(2);

    // Move the scene 2 m farther away and 1 m up without moving the mouse
    {
        let mut scene = t.resource_mut::<SceneConfiguration>();
        scene.origin.translation.z -= 2.0;
        scene.origin.translation.y += 1.0;
    }
    t.step_n(3);

    // The crosshair is a child of the scene plane at the same scene x, y, so it moved with it
    let scene_entity = {
        let mut query = t.world_mut().query_filtered::<Entity, With<common::scene::SceneEntity>>();
        query.single(t.world()).unwrap()
    };
    let mut query = t.world_mut().query::<(&Transform, &ChildOf, &CalibrationCrosshair)>();
    let (transform, parent, _) = query.single(t.world()).unwrap();
    assert_eq!(parent.parent(), scene_entity);
    assert_eq!(transform.translation, Vec3::new(1.0, 0.5, 0.0));
}
