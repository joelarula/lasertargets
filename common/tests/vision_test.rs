//! Camera-driven reactivity without a camera: vision messages become follower entities, effects,
//! triggers and signals.

use bevy::prelude::*;
use common::path::UniversalPath;
use common::pipeline::plugin::*;
use common::pipeline::{EntitySignals, PipelineSpec, Signals};
use common::vision::*;
use std::time::Duration;
use test_harness::TestApp;

const RETICLE: &str = "name: test/reticle\nsource:\n  shape: { builtin: crosshair, size_m: 0.02 }\n";
const RIPPLE: &str = "name: test/ripple\nlifetime_ms: 100\nsource:\n  shape: { builtin: circle, size_m: 0.01 }\n";

fn app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins((PathPipelinePlugin, VisionReactivePlugin));
    t.app_mut()
        .embed_pipeline(RETICLE)
        .embed_pipeline(RIPPLE)
        .follow_vision("laser", "test/reticle")
        .on_vision_appear("laser", "test/ripple");
    t.capture::<ReactionTriggered>();
    t
}

fn seen(track: TrackId, x: f32, timestamp_ms: u64) -> VisionDetection {
    VisionDetection {
        track,
        appeared: false,
        label: "laser".into(),
        world_pos: Vec3::new(x, 1.0, 0.0),
        scene_pos: Vec2::new(x, 1.0),
        size_m: Vec2::splat(0.01),
        camera_pos: Vec2::splat(0.5),
        galvo_pos: Vec2::ZERO,
        confidence: 0.8,
        timestamp_ms,
    }
}

fn keys(t: &TestApp) -> Vec<String> {
    t.captured::<ReactionTriggered>().iter().map(|r| r.key.clone()).collect()
}

fn follower(t: &mut TestApp) -> Entity {
    let mut query = t.world_mut().query_filtered::<Entity, With<VisionTrack>>();
    query.single(t.world()).expect("one follower")
}

#[test]
fn a_tracked_object_gets_a_follower_and_an_appear_effect() {
    let mut t = app();
    t.send(seen(7, 0.2, 0)).step();

    let e = follower(&mut t);
    assert_eq!(t.world().get::<Transform>(e).unwrap().translation, Vec3::new(0.2, 1.0, 0.0));
    assert!(!t.world().get::<UniversalPath>(e).unwrap().segments.is_empty(), "drawn by its pipeline in the first frame");
    assert_eq!(t.world().get::<EntitySignals>(e).unwrap().get("confidence"), Some(0.8));
    assert_eq!(t.count::<DespawnOnFinish>(), 1, "one ripple where the dot appeared");

    assert!(keys(&t).contains(&"vision.laser.appeared".to_string()));
    assert_eq!(t.resource::<Signals>().get("vision.laser.present"), Some(1.0));
    assert_eq!(t.resource::<Signals>().get("vision.laser.confidence"), Some(0.8));
}

#[test]
fn the_follower_moves_with_the_object_and_reports_speed() {
    let mut t = app();
    t.send(seen(7, 0.0, 0)).step();
    // 0.5 m in 0.5 s, sent as two camera frames in one Bevy frame
    t.send(seen(7, 0.25, 250)).send(seen(7, 0.5, 500)).step();

    let e = follower(&mut t);
    assert_eq!(t.world().get::<Transform>(e).unwrap().translation.x, 0.5);
    let track = t.world().get::<VisionTrack>(e).unwrap();
    assert!((track.speed_m_s - 1.0).abs() < 1e-4, "{}", track.speed_m_s);
    assert_eq!(t.world().get::<EntitySignals>(e).unwrap().get("speed"), Some(0.5), "1 m/s of the 2 m/s full scale");
    assert_eq!(t.count::<DespawnOnFinish>(), 1, "appear effect only once");
}

#[test]
fn a_lost_object_despawns_its_follower_and_clears_its_signals() {
    let mut t = app();
    t.send(seen(7, 0.0, 0)).step();
    t.send(VisionLost { track: 7, label: "laser".into() }).step();

    assert_eq!(t.count::<VisionTrack>(), 0);
    assert!(keys(&t).contains(&"vision.laser.lost".to_string()));
    assert_eq!(t.resource::<Signals>().get("vision.laser.present"), Some(0.0));
}

#[test]
fn an_object_that_goes_quiet_is_lost_without_a_message() {
    let mut t = app();
    t.send(seen(7, 0.0, 0)).step();
    t.advance(Duration::from_secs_f64(VisionReactSettings::default().stale_after_secs + 0.1));
    assert_eq!(t.count::<VisionTrack>(), 0);
}

#[test]
fn analyzer_signals_and_appear_triggers_drive_pipelines() {
    let mut t = app();
    let watcher = t
        .world_mut()
        .spawn((
            UniversalPath::default(),
            PipelineRef::inline(
                PipelineSpec::from_yaml("name: t\nreactions:\n  vision.laser.appeared:\n    - envelope: { name: flash, decay_ms: 1000 }\n").unwrap(),
            ),
        ))
        .id();
    t.step();
    t.send(VisionSignal { name: "motion".into(), value: 0.4 }).send(seen(1, 0.0, 0)).step();

    assert_eq!(t.resource::<Signals>().get("vision.motion"), Some(0.4));
    let flash = t.world().get::<EntitySignals>(watcher).unwrap().get("flash").unwrap_or(0.0);
    assert!(flash > 0.5, "the appear trigger reached a pipeline in the same frame ({flash})");
}
