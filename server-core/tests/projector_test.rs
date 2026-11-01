//! Deterministic tests for the projector plugin's frame building.
//!
//! The DAC is disabled (`ProjectorDacController::enabled = false`) so no native library is
//! loaded and no output thread starts; tests observe the `LaserPointBuffer` the thread would send.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use common::config::{CameraConfiguration, ProjectorConfiguration};
use common::path::{PathHint, PathPoint, PathSegment, UniversalPath};
use common::scene::{SceneEntity, SceneSetupPlugin};
use common::state::CalibrationState;
use gamepad::{Btn, GamepadButtonEvent};
use laserlogic::helios::HeliosPoint;
use server_core::plugins::calibration::CalibrationPath;
use server_core::plugins::projector::{LaserPointBuffer, ProjectorDacController, ProjectorPlugin};
use std::sync::atomic::Ordering;
use test_harness::TestApp;

/// Default scene: 10 x 6 m centred at (0, 3, -10), so x in [-5, 5] and y in [0, 6].
const SCENE_CENTER: Vec3 = Vec3::new(0.0, 3.0, -10.0);

/// Frames for the projector to lock onto the scene and the buffer to settle.
const SETTLE: u32 = 3;

fn projector_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<CalibrationState>()
        .add_message::<GamepadButtonEvent>();
    t.insert_resource(CameraConfiguration::default())
        .add_plugins((SceneSetupPlugin, ProjectorPlugin))
        .insert_resource(ProjectorDacController { enabled: false, ..Default::default() });
    t
}

fn buffer(t: &TestApp) -> Vec<HeliosPoint> {
    t.resource::<LaserPointBuffer>().points.lock().unwrap().clone()
}

fn is_lit(p: &HeliosPoint) -> bool {
    p.r != 0 || p.g != 0 || p.b != 0 || p.i != 0
}

fn lit(t: &TestApp) -> Vec<HeliosPoint> {
    buffer(t).into_iter().filter(is_lit).collect()
}

fn white(x: f32, y: f32) -> PathPoint {
    PathPoint::new(x, y, 255, 255, 255, 0)
}

fn square(half: f32) -> UniversalPath {
    UniversalPath::from_segment(PathSegment::new(vec![
        white(-half, -half),
        white(half, -half),
        white(half, half),
        white(-half, half),
        white(-half, -half),
    ]))
}

fn line(from: f32, to: f32) -> UniversalPath {
    UniversalPath::from_segment(PathSegment::new(vec![white(from, 0.0), white(to, 0.0)]))
}

/// Run a fresh app with one path at `position` and return the settled buffer.
fn render(path: UniversalPath, position: Vec3) -> Vec<HeliosPoint> {
    let mut t = projector_app();
    t.world_mut().spawn((path, Transform::from_translation(position)));
    t.step_n(SETTLE);
    buffer(&t)
}

#[test]
fn path_at_scene_center_is_drawn_around_dac_center() {
    let mut t = projector_app();
    t.world_mut().spawn((square(0.5), Transform::from_translation(SCENE_CENTER)));
    t.step_n(SETTLE);

    let lit = lit(&t);
    assert!(!lit.is_empty(), "square should produce lit points");
    for p in &lit {
        assert!((p.x as i32 - 2047).abs() < 300 && (p.y as i32 - 2047).abs() < 300, "{:?} too far from centre", p);
    }
    // Both sides of the square are present
    assert!(lit.iter().any(|p| p.x < 1947) && lit.iter().any(|p| p.x > 2147));
}

#[test]
fn empty_world_produces_empty_buffer() {
    let mut t = projector_app();
    t.step_n(SETTLE);
    assert!(buffer(&t).is_empty());
}

#[test]
fn paths_outside_scene_are_culled() {
    assert!(render(square(0.5), SCENE_CENTER + Vec3::new(20.0, 0.0, 0.0)).is_empty());
    assert!(render(square(0.5), SCENE_CENTER + Vec3::new(0.0, 10.0, 0.0)).is_empty());
}

#[test]
fn path_leaving_scene_is_clipped_at_the_edge() {
    // Scene right edge is x = 5: a line to x = 8 is cut exactly where a line to x = 5 ends
    let clipped = render(line(-2.0, 8.0), SCENE_CENTER);
    let exact = render(line(-2.0, 5.0), SCENE_CENTER);
    assert!(!clipped.is_empty());
    assert_eq!(clipped, exact);
}

#[test]
#[ignore = "BUG: scene clipping only adds a border point when a path exits the scene; a path entering \
            from outside starts at its first inside point, so a line drawn inward collapses to a dot"]
fn path_clipping_is_direction_independent() {
    let outward = render(line(-2.0, 8.0), SCENE_CENTER);
    let inward = render(line(8.0, -2.0), SCENE_CENTER);
    assert_eq!(lit_count(&outward), lit_count(&inward));
}

fn lit_count(points: &[HeliosPoint]) -> usize {
    points.iter().filter(|p| is_lit(p)).count()
}

#[test]
fn scene_entity_transform_is_composed_with_path_transform() {
    let direct = render(square(0.5), SCENE_CENTER);

    let mut t = projector_app();
    t.world_mut().spawn((SceneEntity, Transform::from_translation(SCENE_CENTER)));
    t.world_mut().spawn((square(0.5), Transform::IDENTITY));
    t.step_n(SETTLE);

    assert!(!direct.is_empty());
    assert_eq!(buffer(&t), direct);
}

fn dwelled_square() -> UniversalPath {
    let mut path = square(0.5);
    path.segments[0].points[1].dwell = 5;
    path
}

#[test]
fn point_dwell_repeats_points() {
    let dwelled = render(dwelled_square(), SCENE_CENTER);
    // The corner at points[1] is the right-most, lowest-on-screen vertex: max x, max y
    let max_x = dwelled.iter().filter(|p| is_lit(p)).map(|p| p.x).max().unwrap();
    let max_y = dwelled.iter().filter(|p| is_lit(p)).map(|p| p.y).max().unwrap();
    let at_corner = dwelled.iter().filter(|p| is_lit(p) && p.x == max_x && p.y == max_y).count();
    assert!(at_corner >= 5, "only {} points at the dwelled corner", at_corner);
}

#[test]
#[ignore = "BUG: PathPoint.dwell is expanded into duplicate points before optimisation; duplicates make \
            angle_at_point degenerate, so the automatic corner dwell is lost and a dwell hint yields fewer \
            points than no hint"]
fn point_dwell_adds_to_corner_dwell() {
    let plain = render(square(0.5), SCENE_CENTER);
    let dwelled = render(dwelled_square(), SCENE_CENTER);
    assert!(lit_count(&dwelled) >= lit_count(&plain) + 4, "dwelled {} plain {}", lit_count(&dwelled), lit_count(&plain));
}

#[test]
fn text_hint_uses_text_pipeline() {
    // Many collinear points: the general pipeline keeps them, Douglas-Peucker in the text pipeline drops them
    let points: Vec<PathPoint> = (0..=8).map(|i| white(-1.0 + i as f32 * 0.25, 0.0)).collect();
    let general = UniversalPath::from_segment(PathSegment::new(points.clone()));
    let mut text_seg = PathSegment::new(points);
    text_seg.hint = PathHint::Text;
    let text = UniversalPath::from_segment(text_seg);

    let general = render(general, SCENE_CENTER);
    let text = render(text, SCENE_CENTER);
    assert!(lit_count(&text) < lit_count(&general), "text {} general {}", lit_count(&text), lit_count(&general));
}

#[test]
fn calibration_paths_follow_calibration_state() {
    let mut t = projector_app();
    t.world_mut().spawn((square(0.5), Transform::from_translation(SCENE_CENTER), CalibrationPath));
    t.step_n(SETTLE);
    assert!(!lit(&t).is_empty(), "calibration path drawn while CalibrationState::On");

    t.set_state(CalibrationState::Off);
    t.step_n(SETTLE);
    assert!(buffer(&t).is_empty(), "calibration path hidden while CalibrationState::Off");
}

#[test]
fn switched_off_projector_stops_updating_buffer() {
    let mut t = projector_app();
    let path = t.world_mut().spawn((square(0.5), Transform::from_translation(SCENE_CENTER))).id();
    t.step_n(SETTLE);
    let before = buffer(&t);
    assert!(!before.is_empty());

    t.resource_mut::<ProjectorConfiguration>().switched_on = false;
    t.world_mut().despawn(path);
    t.step_n(SETTLE);

    // The DAC thread is told to blank; the buffer itself is left as it was
    assert!(!t.resource::<ProjectorDacController>().switched_on_flag.load(Ordering::Acquire));
    assert_eq!(buffer(&t), before);
}

#[test]
fn gamepad_start_toggles_laser() {
    let mut t = projector_app();
    t.step();
    assert!(t.resource::<ProjectorConfiguration>().switched_on);

    t.send(GamepadButtonEvent { client_id: 0, button: Btn::Start, pressed: true }).step();
    assert!(!t.resource::<ProjectorConfiguration>().switched_on);

    // Releases and other buttons are ignored
    t.send(GamepadButtonEvent { client_id: 0, button: Btn::Start, pressed: false })
        .send(GamepadButtonEvent { client_id: 0, button: Btn::South, pressed: true })
        .step();
    assert!(!t.resource::<ProjectorConfiguration>().switched_on);

    t.send(GamepadButtonEvent { client_id: 0, button: Btn::Start, pressed: true }).step();
    assert!(t.resource::<ProjectorConfiguration>().switched_on);
    // update_projector is not ordered after the toggle system, so the DAC flag may lag one frame
    t.step();
    assert!(t.resource::<ProjectorDacController>().switched_on_flag.load(Ordering::Acquire));
}

#[test]
fn disabled_dac_never_connects_or_starts_thread() {
    let mut t = projector_app();
    // Longer than the 5 s reconnect interval
    t.advance(std::time::Duration::from_secs(6));

    let dac = t.resource::<ProjectorDacController>();
    assert!(!dac.initialized);
    assert!(!dac.thread_running);
    assert!(dac.shutdown_sender.is_none());
    assert!(!t.resource::<ProjectorConfiguration>().connected);
}

#[test]
fn projector_locks_onto_scene_center() {
    let mut t = projector_app();
    t.step_n(SETTLE);
    let config = t.resource::<ProjectorConfiguration>();
    let forward = config.origin.rotation * Vec3::NEG_Z;
    let expected = (SCENE_CENTER - config.origin.translation).normalize();
    assert!(forward.dot(expected) > 0.9999, "forward {:?} expected {:?}", forward, expected);
}
