//! Deterministic tests for the server path plugin: modulator morphing and the aggregated
//! `BroadcastScenePaths` stream sent to terminals.

use bevy::prelude::*;
use common::path::*;
use server_core::plugins::path::{PathNetworkPlugin, VectorFrameCounter};
use test_harness::TestApp;

fn path_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(PathNetworkPlugin).capture::<BroadcastScenePaths>();
    t
}

fn white(x: f32, y: f32) -> PathPoint {
    PathPoint::new(x, y, 255, 255, 255, 0)
}

fn horizontal(y: f32) -> UniversalPath {
    UniversalPath::from_segment(PathSegment::new(vec![white(0.0, y), white(10.0, y)]))
}

fn ys(path: &UniversalPath) -> Vec<f32> {
    path.segments[0].points.iter().map(|p| p.y).collect()
}

// --- modulators ---------------------------------------------------------------

#[test]
fn modulator_morphs_path_in_place() {
    let mut t = path_app();
    let e = t
        .world_mut()
        .spawn((
            UniversalPath::empty(),
            PathModulator::new(horizontal(0.0), horizontal(10.0), ModulatorSource::Manual(0.5), EasingFunction::Linear),
        ))
        .id();
    t.step();

    assert_eq!(ys(t.world().get::<UniversalPath>(e).unwrap()), vec![5.0, 5.0]);
    assert_eq!(t.world().get::<PathModulator>(e).unwrap().current_t, 0.5);
}

#[test]
fn modulator_applies_easing() {
    let mut t = path_app();
    let e = t
        .world_mut()
        .spawn((
            UniversalPath::empty(),
            PathModulator::new(horizontal(0.0), horizontal(10.0), ModulatorSource::Manual(0.5), EasingFunction::EaseInQuad),
        ))
        .id();
    t.step();

    assert_eq!(t.world().get::<PathModulator>(e).unwrap().current_t, 0.25);
    assert_eq!(ys(t.world().get::<UniversalPath>(e).unwrap()), vec![2.5, 2.5]);
}

#[test]
fn timeline_modulator_advances_with_time() {
    let mut t = path_app();
    let e = t
        .world_mut()
        .spawn((
            UniversalPath::empty(),
            PathModulator::from_animation(horizontal(0.0), horizontal(10.0), 1.0, LoopMode::Once),
        ))
        .id();

    // First step has zero delta; 25 more steps of 20 ms = 0.5 s
    t.step_n(26);
    let current = t.world().get::<PathModulator>(e).unwrap().current_t;
    assert!((current - 0.5).abs() < 1e-3, "t = {}", current);

    t.step_n(50);
    assert_eq!(t.world().get::<PathModulator>(e).unwrap().current_t, 1.0);
    assert_eq!(ys(t.world().get::<UniversalPath>(e).unwrap()), vec![10.0, 10.0]);
}

// --- broadcast --------------------------------------------------------------------

#[test]
fn first_frame_broadcasts_only_renderable_paths() {
    let mut t = path_app();
    t.world_mut().spawn((horizontal(1.0), PathRenderable::default()));
    t.world_mut().spawn(horizontal(2.0));
    t.step();

    let frames = t.drain::<BroadcastScenePaths>();
    assert_eq!(frames.len(), 1);
    let frame = &frames[0].frame;
    assert_eq!(frame.frame_id, 1);
    assert_eq!(frame.len(), 1);
    assert_eq!(frame.paths[0].path, horizontal(1.0));
    assert_eq!(frame.paths[0].position, Vec3::ZERO);
}

#[test]
fn unchanged_scene_is_not_rebroadcast_every_frame() {
    let mut t = path_app();
    t.world_mut().spawn((horizontal(1.0), PathRenderable::default()));
    t.step();
    t.drain::<BroadcastScenePaths>();

    t.step_n(10);
    assert!(t.drain::<BroadcastScenePaths>().is_empty());
}

#[test]
fn path_change_triggers_broadcast_with_next_frame_id() {
    let mut t = path_app();
    let e = t.world_mut().spawn((horizontal(1.0), PathRenderable::default())).id();
    t.step_n(3);
    t.drain::<BroadcastScenePaths>();

    *t.world_mut().get_mut::<UniversalPath>(e).unwrap() = horizontal(7.0);
    t.step();

    let frames = t.drain::<BroadcastScenePaths>();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].frame.frame_id, 2);
    assert_eq!(frames[0].frame.paths[0].path, horizontal(7.0));
    assert_eq!(t.resource::<VectorFrameCounter>().0, 2);
}

#[test]
fn despawn_broadcasts_the_smaller_scene() {
    let mut t = path_app();
    let a = t.world_mut().spawn((horizontal(1.0), PathRenderable::default())).id();
    t.world_mut().spawn((horizontal(2.0), PathRenderable::default()));
    t.step();
    t.drain::<BroadcastScenePaths>();

    t.world_mut().despawn(a);
    t.step();
    let frames = t.drain::<BroadcastScenePaths>();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].frame.len(), 1);
}

#[test]
fn removing_last_path_broadcasts_empty_frame() {
    // Terminals need the empty frame to clear what they are drawing
    let mut t = path_app();
    let a = t.world_mut().spawn((horizontal(1.0), PathRenderable::default())).id();
    t.step();
    t.drain::<BroadcastScenePaths>();

    t.world_mut().despawn(a);
    t.step();
    let frames = t.drain::<BroadcastScenePaths>();
    assert_eq!(frames.len(), 1);
    assert!(frames[0].frame.is_empty());
}

#[test]
fn modulated_renderable_path_is_broadcast_every_frame() {
    let mut t = path_app();
    t.world_mut().spawn((
        UniversalPath::empty(),
        PathRenderable::default(),
        PathModulator::new(horizontal(0.0), horizontal(10.0), ModulatorSource::LfoSine { frequency_hz: 1.0 }, EasingFunction::Linear),
    ));
    t.step_n(5);
    assert_eq!(t.drain::<BroadcastScenePaths>().len(), 5);
}

#[test]
#[ignore = "BUG: the heartbeat checks frame_counter % 30, but the counter only advances when a frame is \
            broadcast, so an unchanged scene stops broadcasting after the first frame and never heartbeats"]
fn unchanged_scene_heartbeats_periodically() {
    let mut t = path_app();
    t.world_mut().spawn((horizontal(1.0), PathRenderable::default()));
    t.step();
    t.drain::<BroadcastScenePaths>();

    t.step_n(60);
    assert!(!t.drain::<BroadcastScenePaths>().is_empty());
}

#[test]
#[ignore = "BUG: broadcast prefers GlobalTransform, which the server never propagates (no TransformPlugin); \
            every Transform gets an identity GlobalTransform, so broadcast positions ignore the Transform"]
fn broadcast_position_follows_transform() {
    let mut t = path_app();
    t.world_mut().spawn((horizontal(1.0), PathRenderable::default(), Transform::from_xyz(1.0, 2.0, 3.0)));
    t.step();
    let frames = t.drain::<BroadcastScenePaths>();
    assert_eq!(frames[0].frame.paths[0].position, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
#[ignore = "BUG: PathNetworkPlugin uses init_resource::<MorphStrategyRegistry>(), whose derived Default \
            is empty; the built-in cycle_modulo strategy is only registered by MorphStrategyRegistry::new()"]
fn plugin_registry_contains_builtin_strategy() {
    let mut t = path_app();
    t.step();
    assert!(t.resource::<MorphStrategyRegistry>().create("cycle_modulo").is_some());
}
