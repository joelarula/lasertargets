//! Deterministic tests for the reactive pipeline plugin: binding, component signals, triggers,
//! reactions, curator envelopes both ways, effect lifetimes and hot reload.

use bevy::prelude::*;
use common::curator_core::CuratorEventEnvelope;
use common::path::{PathPoint, PathSegment, UniversalPath};
use common::pipeline::plugin::*;
use common::pipeline::{EntitySignals, PipelineSpec};
use test_harness::TestApp;

#[derive(Component)]
struct Health(f32);

#[derive(Message, Clone)]
struct Shot {
    target: Entity,
    power: f32,
}

fn app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(PathPipelinePlugin);
    t.app_mut()
        .bind_signal::<Health>("health", |h| h.0)
        .add_message::<Shot>()
        .register_trigger_message::<Shot>("hit", |m| Some((TriggerTarget::Entity(m.target), m.power)))
        .register_trigger_envelope("phase");
    t.capture::<CuratorEventEnvelope>();
    t
}

fn spec(yaml: &str) -> PipelineRef {
    PipelineRef::inline(PipelineSpec::from_yaml(yaml).unwrap())
}

fn dot(x: f32) -> UniversalPath {
    UniversalPath::from_segment(PathSegment::new(vec![PathPoint::new(x, 0.0, 200, 200, 200, 0)]))
}

fn first(t: &TestApp, e: Entity) -> PathPoint {
    t.world().get::<UniversalPath>(e).unwrap().segments[0].points[0].clone()
}

#[test]
fn binds_pipeline_and_transforms_own_shape() {
    let mut t = app();
    let e = t.world_mut().spawn((dot(1.0), spec("name: t\nstages:\n  - translate: { x: 2 }\n"))).id();
    t.step();
    assert_eq!(first(&t, e).x, 3.0);
    assert_eq!(t.world().get::<BasePath>(e).unwrap().0, dot(1.0), "own shape captured as base");

    // Game code changes the shape through BasePath; the stage still applies once
    t.world_mut().get_mut::<BasePath>(e).unwrap().0 = dot(10.0);
    t.step_n(3);
    assert_eq!(first(&t, e).x, 12.0);
}

#[test]
fn source_generates_geometry() {
    let mut t = app();
    let e = t.world_mut().spawn(spec("name: t\nsource:\n  shape: { builtin: circle, size_m: 0.5 }\n")).id();
    t.step();
    let path = t.world().get::<UniversalPath>(e).unwrap();
    let max_x = path.flatten().iter().map(|p| p.x).fold(f32::MIN, f32::max);
    assert!((max_x - 0.5).abs() < 0.01, "{max_x}");
}

#[test]
fn component_signal_drives_parameter() {
    let mut t = app();
    let e = t
        .world_mut()
        .spawn((dot(0.0), Health(1.0), spec("name: t\nstages:\n  - color_lerp: { to: [255, 0, 0], amount: { signal: self.health, range: [1, 0] } }\n")))
        .id();
    t.step();
    assert_eq!(first(&t, e).g, 200, "full health keeps the colour");

    t.world_mut().get_mut::<Health>(e).unwrap().0 = 0.0;
    t.step();
    assert_eq!((first(&t, e).r, first(&t, e).g), (255, 0), "no health is red, same frame");
    assert_eq!(t.world().get::<EntitySignals>(e).unwrap().get("health"), Some(0.0));
}

#[test]
fn message_trigger_flashes_with_strength_then_decays() {
    let mut t = app();
    let e = t
        .world_mut()
        .spawn((
            dot(0.0),
            spec(
                "name: t
stages:
  - translate: { x: { signal: self.hit, range: [0, 10] } }
reactions:
  hit:
    - envelope: { name: hit, decay_ms: 100 }
",
            ),
        ))
        .id();
    t.step();
    t.send(Shot { target: e, power: 0.5 });
    t.step();
    assert_eq!(first(&t, e).x, 5.0, "strength 0.5 halves the peak");
    t.step_n(10);
    assert_eq!(first(&t, e).x, 0.0);
}

#[test]
fn envelope_tags_broadcast_and_emits_go_back_to_curator() {
    let mut t = app();
    let a = t
        .world_mut()
        .spawn(spec("name: a\nreactions:\n  phase.Title:\n    - emit: { tag: title_shown, payload: 1 }\n"))
        .id();
    t.world_mut().spawn(spec("name: b\nreactions:\n  phase:\n    - emit: phase_seen\n"));
    t.step();
    t.drain::<CuratorEventEnvelope>();

    t.send(CuratorEventEnvelope::new("phase", serde_json::json!("Title")));
    t.step();
    let mut tags: Vec<(String, Option<Entity>)> =
        t.drain::<CuratorEventEnvelope>().into_iter().map(|e| (e.tag, e.source_entity)).filter(|(tag, _)| tag != "phase").collect();
    tags.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(tags.len(), 2, "{tags:?}");
    assert_eq!(tags[0].0, "phase_seen");
    assert_eq!(tags[1], ("title_shown".to_string(), Some(a)));
}

#[test]
fn effect_despawns_when_lifetime_ends() {
    let mut t = app();
    let e = t
        .world_mut()
        .spawn((
            DespawnOnFinish,
            spec("name: ripple\nlifetime_ms: 100\nsource:\n  shape: { builtin: circle, size_m: 0.01 }\nstages:\n  - scale: { factor: { signal: self.life, range: [1, 40] } }\n"),
        ))
        .id();
    t.step_n(3);
    assert!(t.world().get_entity(e).is_ok());
    t.step_n(5);
    assert!(t.world().get_entity(e).is_err(), "despawned after 100 ms");
}

#[test]
fn invalid_reload_keeps_previous_version() {
    let mut t = app();
    let e = t.world_mut().spawn((dot(1.0), spec("name: t\nstages:\n  - translate: { x: 1 }\n"))).id();
    t.step();
    assert_eq!(first(&t, e).x, 2.0);

    *t.world_mut().get_mut::<PipelineRef>(e).unwrap() = spec("name: t\nstages:\n  - translate: { x: 1, bogus: 3 }\n");
    t.step_n(2);
    assert_eq!(first(&t, e).x, 2.0, "broken version rejected");

    *t.world_mut().get_mut::<PipelineRef>(e).unwrap() = spec("name: t\nstages:\n  - translate: { x: 5 }\n");
    t.step();
    assert_eq!(first(&t, e).x, 6.0, "fixed version applied");
}

#[test]
fn use_pipeline_resolves_embedded_spec_without_asset_server() {
    let mut t = app();
    t.app_mut().embed_pipeline("name: fx/shift\nstages:\n  - translate: { x: 4 }\n");
    let e = t.world_mut().spawn((dot(1.0), UsePipeline::new("fx/shift"))).id();
    t.step();
    assert_eq!(first(&t, e).x, 5.0);
}
