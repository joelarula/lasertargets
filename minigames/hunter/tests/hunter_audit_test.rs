//! Hunter on a headless server app: a session starts a round with an audit log; spawns, hits,
//! misses, setup changes and the end of the round all land in it with the parameters of the moment.

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use common::curator_core::{ActorPromptResponseEvent, CuratorEventEnvelope};
use common::game::{ExitGameEvent, GameRegistry, GameSessionCreated, GameSessionUpdate};
use common::network::MouseButtonInputEvent;
use common::pipeline::plugin::PathPipelinePlugin;
use common::config::{ConfigTransform, SceneConfiguration};
use common::scene::{SceneEntity, SceneSetup};
use common::session::Session;
use common::state::{CalibrationState, ServerState, SessionState};
use hunter::common::{GAME_ID, HunterGamePlugin};
use hunter::model::*;
use hunter::server::HunterGameServerPlugin;
use test_harness::TestApp;

const SESSION: Uuid = Uuid::from_u128(0x4855);

fn server_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<ServerState>()
        .add_sub_state::<SessionState>()
        .init_state::<CalibrationState>()
        .add_message::<GameSessionCreated>()
        .add_message::<GameSessionUpdate>()
        .add_message::<ExitGameEvent>()
        .add_message::<MouseButtonInputEvent>()
        .add_message::<CuratorEventEnvelope>()
        .add_message::<ActorPromptResponseEvent>()
        .init_resource::<GameRegistry>()
        // Billboard centred on the world origin, so world and scene metres coincide
        .insert_resource(SceneSetup {
            scene: SceneConfiguration { origin: ConfigTransform::default(), ..default() },
            ..default()
        });
    t.add_plugins((PathPipelinePlugin, HunterGamePlugin, HunterGameServerPlugin));
    t.capture::<CuratorEventEnvelope>();
    t.set_state(CalibrationState::Off).set_state(ServerState::Session).step();
    t
}

fn session(id: Uuid, game_id: u16) -> Session {
    let mut session = Session::new(id, "test");
    session.game_id = game_id;
    session
}

fn started(t: &mut TestApp) {
    t.send(GameSessionCreated { game_session: session(SESSION, GAME_ID) }).step_n(2);
}

fn stats(t: &TestApp) -> &HunterGameStats {
    t.resource::<HunterGameStats>()
}

fn events(t: &TestApp) -> Vec<HunterAuditEvent> {
    stats(t).log.iter().map(|e| e.event.clone()).collect()
}

fn spawn(t: &mut TestApp, target: HunterTarget, at: Vec3) -> Uuid {
    t.send(SpawnHunterTargetEvent { target, position: at }).step_n(2);
    match &stats(t).log.last().unwrap().event {
        HunterAuditEvent::Spawned { target, .. } => *target,
        other => panic!("expected a spawn, got {other:?}"),
    }
}

fn shoot(t: &mut TestApp, at: Vec3) -> Shot {
    t.send(HunterClickEvent { source: ShotSource::Mouse, click_position: at }).step_n(2);
    stats(t).shots().last().copied().expect("shot logged")
}

#[test]
fn round_starts_with_the_setup_in_effect() {
    let mut t = server_app();
    started(&mut t);
    let stats = stats(&t);
    assert_eq!(stats.session_id, SESSION);
    assert!(stats.started_unix > 0.0);
    let setup = t.resource::<SceneSetup>().clone();
    assert!(matches!(&events(&t)[..], [HunterAuditEvent::Setup(s)] if *s == setup));
}

#[test]
fn hit_pops_the_target_and_logs_lifetime_and_offset() {
    let mut t = server_app();
    started(&mut t);
    let id = spawn(&mut t, HunterTarget::Basic(0.3, Color::WHITE), Vec3::new(1.0, 1.0, 0.0));

    let shot = shoot(&mut t, Vec3::new(1.1, 0.9, 0.0));
    let ShotOutcome::Hit { target, reward, lifetime } = shot.outcome else { panic!("{shot:?}") };
    assert_eq!((target, reward), (id, TARGET_REWARD));
    assert!(lifetime > 0.0);
    let nearest = shot.nearest.unwrap();
    assert!((nearest.offset - Vec2::new(0.1, -0.1)).length() < 1e-5);
    assert_eq!(t.count::<Target>(), 0);
    assert_eq!(
        stats(&t).tally,
        HunterTally { spawned: 1, popped: 1, score: TARGET_REWARD, ..default() }
    );
}

#[test]
fn misses_log_the_nearest_target_and_off_scene_shots_are_not_counted() {
    let mut t = server_app();
    started(&mut t);
    let id = spawn(&mut t, HunterTarget::Basic(0.2, Color::WHITE), Vec3::ZERO);

    let miss = shoot(&mut t, Vec3::new(0.5, 0.0, 0.0));
    assert_eq!(miss.outcome, ShotOutcome::Miss);
    assert_eq!(miss.nearest.map(|n| (n.target, n.offset)), Some((id, Vec2::new(0.5, 0.0))));

    // Default scene is 10 × 6 m around the origin
    let off = shoot(&mut t, Vec3::new(20.0, 0.0, 0.0));
    assert_eq!(off.outcome, ShotOutcome::OffScene);
    assert_eq!(stats(&t).tally.misses, 1);
    assert_eq!(t.count::<Target>(), 1, "missed target stays");
}

#[test]
fn setup_changes_are_logged_once_each() {
    let mut t = server_app();
    started(&mut t);
    t.resource_mut::<SceneSetup>().projector.angle += 5.0;
    t.step_n(2);
    t.resource_mut::<SceneSetup>().set_changed(); // touched but equal: not logged again
    t.step_n(2);
    let setups: Vec<_> = events(&t)
        .into_iter()
        .filter_map(|e| match e {
            HunterAuditEvent::Setup(s) => Some(s.projector.angle),
            _ => None,
        })
        .collect();
    let base = t.resource::<SceneSetup>().projector.angle - 5.0;
    assert_eq!(setups, vec![base, base + 5.0]);
}

#[test]
fn positions_are_logged_in_scene_metres_through_the_scene_setup() {
    let mut t = server_app();
    t.resource_mut::<SceneSetup>().scene.origin.translation = Vec3::new(2.0, 1.0, 5.0);
    t.world_mut().spawn((SceneEntity, Transform::from_xyz(2.0, 1.0, 5.0)));
    started(&mut t);

    // Requested off the plane: snapped onto it
    spawn(&mut t, HunterTarget::Basic(0.3, Color::WHITE), Vec3::new(2.5, 1.0, 9.0));
    let Some(HunterAuditEvent::Spawned { local, world, .. }) = events(&t).last().cloned() else { panic!() };
    assert_eq!(local, Vec3::new(0.5, 0.0, 0.0));
    assert_eq!(world, Vec3::new(2.5, 1.0, 5.0));

    let shot = shoot(&mut t, Vec3::new(2.5, 1.1, 5.0));
    assert!(matches!(shot.outcome, ShotOutcome::Hit { .. }));
    assert!((shot.local - Vec3::new(0.5, 0.1, 0.0)).length() < 1e-5);
}

#[test]
fn the_report_summarises_the_log() {
    let mut t = server_app();
    started(&mut t);
    spawn(&mut t, HunterTarget::Basic(0.3, Color::WHITE), Vec3::ZERO);
    spawn(&mut t, HunterTarget::Basic(0.3, Color::WHITE), Vec3::new(2.0, 0.0, 0.0));
    shoot(&mut t, Vec3::new(0.1, 0.0, 0.0));
    shoot(&mut t, Vec3::new(2.1, 1.0, 0.0));

    let report = GameReport::new(stats(&t), t.world().resource::<Time>().elapsed_secs_f64());
    assert_eq!(report.tally.spawned, 2);
    assert_eq!((report.tally.popped, report.tally.misses), (1, 1));
    let offset = report.mean_aim_offset.unwrap();
    assert!((offset - Vec2::new(0.1, 0.5)).length() < 1e-5, "{offset}");
    let md = report.to_markdown();
    assert!(md.contains("| hit |") && md.contains("| miss |") && md.contains("| setup |"), "{md}");
    let json = serde_json::to_string(&report).unwrap();
    let back: GameReport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tally, report.tally);
    assert_eq!(back.log.len(), report.log.len());
    assert_eq!(back.log.last().map(|e| e.event.clone()), report.log.last().map(|e| e.event.clone()));
}

#[test]
fn another_game_ends_the_round_and_clears_it() {
    let mut t = server_app();
    started(&mut t);
    spawn(&mut t, HunterTarget::Balloon(0.2, Color::WHITE), Vec3::ZERO);
    t.send(GameSessionCreated { game_session: session(Uuid::from_u128(9), 2) }).step_n(2);
    assert!(t.world().get_resource::<HunterGameStats>().is_none());
    assert_eq!(t.count::<HunterEntity>(), 0);
    assert!(std::path::Path::new(&format!("stats/hunter/{SESSION}/report.json")).exists());
}

#[test]
fn tally_is_broadcast_when_it_changes() {
    let mut t = server_app();
    started(&mut t);
    spawn(&mut t, HunterTarget::Basic(0.3, Color::WHITE), Vec3::ZERO);
    t.step_n(3);
    let tallies: Vec<HunterTally> = t
        .captured::<CuratorEventEnvelope>()
        .iter()
        .filter_map(|env| env.downcast::<BroadcastStatsUpdateEvent>().ok())
        .map(|e| e.tally)
        .collect();
    assert_eq!(tallies, vec![HunterTally::default(), HunterTally { spawned: 1, ..default() }]);
}

#[test]
fn gamepad_keeps_the_buttons_calibration_does_not_use() {
    use gamepad::{GamepadState, PrevGamepadState, ServerGamepadCursor};
    let mut t = server_app();
    t.insert_resource(ServerGamepadCursor::default())
        .insert_resource(PrevGamepadState::default());
    started(&mut t);
    t.set_state(CalibrationState::On).step();

    let press = |t: &mut TestApp, set: fn(&mut GamepadState)| {
        let mut state = GamepadState { connected: true, ..default() };
        set(&mut state);
        t.insert_resource(state).step();
        t.insert_resource(GamepadState { connected: true, ..default() }).step();
    };
    press(&mut t, |s| s.right_trigger = true); // calibration's "move the middle"
    assert_eq!(stats(&t).shots().count(), 0);
    press(&mut t, |s| s.east = true); // free: still shoots
    assert_eq!(stats(&t).shots().count(), 1);
    assert_eq!(stats(&t).shots().last().unwrap().source, ShotSource::Gamepad);
}
