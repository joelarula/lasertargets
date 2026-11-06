//! Snake on a headless server app: sessions start and stop the game, game keys steer it, pause
//! freezes it, and GAME OVER restarts it without respawning the drawn entities.

use std::time::Duration;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use common::calibration::{ControlKey, GameKeyPressed};
use common::curator_core::CuratorEventEnvelope;
use common::game::{GameRegistry, GameSessionCreated, GameSessionUpdate};
use common::pipeline::plugin::PathPipelinePlugin;
use common::scene::SceneSetup;
use common::session::Session;
use common::state::{CalibrationState, ServerState, SessionState};
use snake::common::{SnakeGamePlugin, SnakePhase};
use snake::model::*;
use snake::server::SnakeGameServerPlugin;
use test_harness::TestApp;

const SESSION: Uuid = Uuid::from_u128(7);

fn server_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<ServerState>()
        .add_sub_state::<SessionState>()
        .init_state::<CalibrationState>()
        .add_message::<GameSessionCreated>()
        .add_message::<GameSessionUpdate>()
        .init_resource::<GameRegistry>()
        .init_resource::<SceneSetup>();
    t.add_plugins((PathPipelinePlugin, SnakeGamePlugin, SnakeGameServerPlugin));
    t.capture::<SnakeGameOverEvent>()
        .capture::<SnakeRestartEvent>()
        .capture::<CuratorEventEnvelope>();
    t.set_state(CalibrationState::Off)
        .set_state(ServerState::Session)
        .step();
    t
}

fn session(game_id: u16) -> Session {
    let mut session = Session::new(
        if game_id == GAME_ID {
            SESSION
        } else {
            Uuid::from_u128(8)
        },
        "test",
    );
    session.game_id = game_id;
    session
}

fn started() -> TestApp {
    let mut t = server_app();
    t.send(GameSessionCreated {
        game_session: session(GAME_ID),
    })
    .step_n(2);
    t
}

fn playing() -> TestApp {
    let mut t = started();
    press(&mut t, ControlKey::Up);
    t.assert_state(SnakePhase::Playing);
    t
}

fn press(t: &mut TestApp, key: ControlKey) {
    t.send(GameKeyPressed { client_id: 1, key }).step_n(2);
}

fn head(t: &TestApp) -> IVec2 {
    t.resource::<SnakeState>().head()
}

fn part_entities(t: &mut TestApp) -> Vec<(Entity, SnakePart, Vec3)> {
    let world = t.world_mut();
    let mut parts: Vec<_> = world
        .query::<(Entity, &SnakePart, &Transform)>()
        .iter(world)
        .map(|(e, part, transform)| (e, *part, transform.translation))
        .collect();
    parts.sort_by_key(|(e, ..)| *e);
    parts
}

#[test]
fn snake_session_starts_a_waiting_round_with_title() {
    let mut t = started();
    t.assert_state(SnakePhase::Waiting);
    assert_eq!(t.resource::<SnakeState>().session_id, SESSION);
    assert_eq!(t.count::<SnakePart>(), 3);
    assert_eq!(
        t.count::<SnakeEntity>(),
        4,
        "head, body, gem and the SNAKE title"
    );

    let start = head(&t);
    t.advance(Duration::from_millis(500));
    assert_eq!(
        head(&t),
        start,
        "nothing moves before the first direction input"
    );
}

#[test]
fn arrow_key_starts_the_round_and_steers() {
    let mut t = started();
    let start = head(&t);
    press(&mut t, ControlKey::Up);
    t.assert_state(SnakePhase::Playing);

    t.advance(Duration::from_millis(150));
    assert_eq!(head(&t).x, start.x);
    assert!(head(&t).y > start.y, "moved up");

    // The head entity follows the state
    let state = t.resource::<SnakeState>().clone();
    let (_, _, head_at) = part_entities(&mut t)
        .into_iter()
        .find(|(_, p, _)| *p == SnakePart::Head)
        .unwrap();
    assert_eq!(head_at, state.grid.to_local(state.head()));
}

#[test]
fn non_arrow_game_keys_do_not_start_the_round() {
    let mut t = started();
    press(&mut t, ControlKey::Space);
    t.assert_state(SnakePhase::Waiting);
}

#[test]
fn paused_session_freezes_the_snake() {
    let mut t = playing();
    t.set_state(SessionState::Paused).step();
    let frozen = head(&t);
    t.advance(Duration::from_millis(500));
    assert_eq!(head(&t), frozen);

    t.set_state(SessionState::Active).step();
    t.advance(Duration::from_millis(200));
    assert_ne!(head(&t), frozen, "moves again after resume");
}

#[test]
fn game_over_restarts_in_place_and_tells_pipelines() {
    let mut t = playing();
    let before = part_entities(&mut t)
        .into_iter()
        .map(|(e, ..)| e)
        .collect::<Vec<_>>();

    // Curl the snake so the next move runs into its own body
    {
        let mut state = t.resource_mut::<SnakeState>();
        state.body = [
            IVec2::new(5, 5),
            IVec2::new(5, 6),
            IVec2::new(6, 6),
            IVec2::new(6, 5),
            IVec2::new(7, 5),
        ]
        .into();
        state.direction = SnakeDirection::Right;
        state.queued_direction = None;
    }
    t.step_until(20, |w| {
        *w.resource::<State<SnakePhase>>().get() == SnakePhase::GameOver
    });
    assert_eq!(t.captured::<SnakeGameOverEvent>().len(), 1);
    press(&mut t, ControlKey::Left);
    assert_eq!(
        t.resource::<SnakeState>().queued_direction,
        None,
        "no steering during GAME OVER"
    );

    t.advance(Duration::from_secs_f32(RESTART_DELAY_SECS + 0.1));
    t.assert_state(SnakePhase::Playing);
    assert_eq!(t.captured::<SnakeRestartEvent>().len(), 1);
    assert_eq!(t.resource::<SnakeState>().gems_eaten, 0);

    let after = part_entities(&mut t)
        .into_iter()
        .map(|(e, ..)| e)
        .collect::<Vec<_>>();
    assert_eq!(before, after, "same entities: no respawn, no laser gap");

    let stats: Vec<BroadcastSnakeStatsEvent> = t
        .captured::<CuratorEventEnvelope>()
        .iter()
        .filter(|env| env.tag == "snake_stats")
        .map(|env| serde_json::from_value(env.payload.clone()).unwrap())
        .collect();
    assert!(
        stats.iter().any(|s| s.game_over),
        "terminals heard about GAME OVER"
    );
    assert!(!stats.last().unwrap().game_over, "and about the next round");
}

#[test]
fn another_game_starting_clears_the_snake() {
    let mut t = playing();
    t.send(GameSessionCreated {
        game_session: session(1),
    })
    .step_n(2);
    t.assert_state(SnakePhase::Off);
    assert!(t.world().get_resource::<SnakeState>().is_none());
    assert_eq!(t.count::<SnakeEntity>(), 0);
}

#[test]
fn session_update_restarts_a_missing_game_only() {
    let mut t = playing();
    let before = part_entities(&mut t)
        .into_iter()
        .map(|(e, ..)| e)
        .collect::<Vec<_>>();
    t.send(GameSessionUpdate {
        game_session: session(GAME_ID),
    })
    .step_n(2);
    t.assert_state(SnakePhase::Playing);
    let after = part_entities(&mut t)
        .into_iter()
        .map(|(e, ..)| e)
        .collect::<Vec<_>>();
    assert_eq!(before, after, "a running game is left alone");
}
