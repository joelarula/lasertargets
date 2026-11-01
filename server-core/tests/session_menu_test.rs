//! Deterministic tests for the server menu workflow and session lifecycle.
//!
//! The app is assembled from the plugins under test only: no Quinnet sockets,
//! no DAC and no hardware gamepad polling. Gamepad input is injected as
//! `GamepadButtonEvent` messages.

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use common::session::*;
use common::state::{CalibrationState, ServerState};
use curator_core::*;
use gamepad::{Btn, GamepadButtonEvent};
use server_core::plugins::menu::{ActiveMenuPrompt, ServerMenuWorkflowMarker, ServerMenuWorkflowPlugin};
use server_core::plugins::session::SessionLifecyclePlugin;
use test_harness::TestApp;

const HUNTER: u16 = 1;
const SNAKE: u16 = 2;

/// Upper bound for a request to settle: message hop + state transition + OnEnter/OnExit.
const SETTLE: u32 = 5;

fn server_app() -> TestApp {
    let mut registry = ChannelRegistry::default();
    registry.register_app(ChannelApp { id: HUNTER, name: "Hunter".into() });
    registry.register_app(ChannelApp { id: SNAKE, name: "Snake".into() });

    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut()
        .init_state::<ServerState>()
        .init_state::<CalibrationState>()
        .add_message::<GamepadButtonEvent>();
    t.insert_resource(registry).add_plugins((
        SessionPlugin,
        CuratorCorePlugin,
        SessionLifecyclePlugin,
        ServerMenuWorkflowPlugin,
    ));
    t.capture::<ActorPromptRequestEvent>()
        .capture::<InitSessionEvent>()
        .capture::<SessionCreated>()
        .capture::<SessionUpdate>();
    t
}

fn sessions(t: &mut TestApp) -> Vec<Session> {
    t.query::<Session>()
}

fn menu_prompt_id(t: &TestApp) -> Uuid {
    t.resource::<ActiveMenuPrompt>().prompt_id.expect("menu prompt should be active")
}

fn choose(t: &mut TestApp, choice: &str) {
    let prompt_id = menu_prompt_id(t);
    t.send(ActorPromptResponseEvent {
        response: ActorPromptResponse {
            prompt_id,
            actor_id: None,
            selected_choice_id: choice.into(),
            custom_payload: None,
        },
    });
}

fn press(t: &mut TestApp, button: Btn) {
    t.send(GamepadButtonEvent { client_id: 0, button, pressed: true });
}

fn audit_tags(t: &TestApp) -> Vec<String> {
    t.resource::<AuditLog>()
        .entries()
        .iter()
        .filter_map(|e| match e {
            AuditEvent::PayloadDispatched { tag, .. } => Some(tag.clone()),
            _ => None,
        })
        .collect()
}

fn in_session(w: &World) -> bool {
    *w.resource::<State<ServerState>>().get() == ServerState::Session
}

fn in_menu(w: &World) -> bool {
    *w.resource::<State<ServerState>>().get() == ServerState::Menu
}

/// Start the app and launch `choice` from the menu; returns the running session.
fn launch(t: &mut TestApp, choice: &str) -> Session {
    t.step();
    choose(t, choice);
    t.step_until(SETTLE, in_session);
    t.step(); // let OnExit(Menu) cleanup and lifecycle broadcasts settle
    let s = sessions(t);
    assert_eq!(s.len(), 1, "exactly one session after launch");
    s.into_iter().next().unwrap()
}

// --- menu -------------------------------------------------------------------

#[test]
fn menu_opens_on_first_frame_with_registered_apps() {
    let mut t = server_app();
    t.step();

    t.assert_state(ServerState::Menu);
    assert_eq!(t.count::<ServerMenuWorkflowMarker>(), 1);

    let requests = t.captured::<ActorPromptRequestEvent>();
    assert_eq!(requests.len(), 1, "prompt is dispatched on the startup frame");
    let prompt = &requests[0].prompt;
    assert_eq!(prompt.prompt_id, menu_prompt_id(&t));
    let choices: Vec<(&str, &str)> = prompt.choices.iter().map(|c| (c.id.as_str(), c.label.as_str())).collect();
    assert_eq!(choices, vec![("1", "Hunter"), ("2", "Snake")]);

    assert_eq!(audit_tags(&t), vec!["server:menu:opened"]);
}

#[test]
fn choosing_app_by_id_starts_session_and_closes_menu() {
    let mut t = server_app();
    let session = launch(&mut t, "2");

    assert_eq!(session.game_id, SNAKE);
    assert_eq!(session.name, "Snake");
    assert_eq!(session.state, SessionState::Active);
    assert!(session.start_timestamp.is_some());

    t.assert_state(ServerState::Session);
    t.assert_state(SessionState::Active);
    assert_eq!(t.count::<ServerMenuWorkflowMarker>(), 0);
    assert_eq!(t.resource::<ActiveMenuPrompt>().prompt_id, None);

    let created = t.captured::<SessionCreated>();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].session.session_id, session.session_id);

    assert_eq!(
        audit_tags(&t),
        vec![
            "server:menu:opened",
            "server:menu:game_selected",
            "session:created",
            "server:menu:closed",
            // the answered prompt completes the menu workflow; its event is read a frame later
            "workflow:completed",
        ]
    );
}

#[test]
fn choosing_app_by_name_is_case_insensitive() {
    let mut t = server_app();
    assert_eq!(launch(&mut t, "hunter").game_id, HUNTER);
}

#[test]
fn invalid_choice_keeps_menu_and_a_later_valid_choice_still_launches() {
    let mut t = server_app();
    t.step();

    choose(&mut t, "99");
    t.step_n(SETTLE);
    t.assert_state(ServerState::Menu);
    assert!(sessions(&mut t).is_empty());
    assert!(t.captured::<InitSessionEvent>().is_empty());

    choose(&mut t, "1");
    t.step_until(SETTLE, in_session);
    assert_eq!(sessions(&mut t)[0].game_id, HUNTER);
}

#[test]
fn launch_envelope_starts_session_by_name() {
    let mut t = server_app();
    t.step();

    t.send(CuratorEventEnvelope {
        tag: "menu:launch_game".into(),
        payload: serde_json::json!({ "game_name": "snake" }),
        session_id: None,
        source_entity: None,
        timestamp_ms: 0,
    });
    t.step_until(SETTLE, in_session);

    assert_eq!(sessions(&mut t)[0].game_id, SNAKE);
    assert!(audit_tags(&t).contains(&"server:menu:envelope_launch".to_string()));
}

#[test]
fn init_for_unregistered_app_is_ignored() {
    let mut t = server_app();
    t.step();

    t.send(InitSessionEvent { game_id: 42, session_id: Uuid::from_u128(1), initial_state: SessionState::Active });
    t.step_n(SETTLE);

    t.assert_state(ServerState::Menu);
    assert!(sessions(&mut t).is_empty());
    assert!(t.captured::<SessionCreated>().is_empty());
}

// --- lifecycle --------------------------------------------------------------

#[test]
fn pause_resume_finish_update_session_and_broadcast() {
    let mut t = server_app();
    let id = launch(&mut t, "1").session_id;
    t.drain::<SessionUpdate>();

    t.send(PauseSessionEvent { session_id: id }).step_n(2);
    assert_eq!(sessions(&mut t)[0].state, SessionState::Paused);
    t.assert_state(SessionState::Paused);

    t.send(ResumeSessionEvent { session_id: id }).step_n(2);
    assert_eq!(sessions(&mut t)[0].state, SessionState::Active);
    t.assert_state(SessionState::Active);

    t.send(FinishSessionEvent { session_id: id }).step_n(2);
    let s = &sessions(&mut t)[0];
    assert_eq!(s.state, SessionState::Finished);
    assert!(s.end_timestamp.is_some());

    let broadcast: Vec<SessionState> = t.drain::<SessionUpdate>().into_iter().map(|u| u.session.state).collect();
    assert_eq!(broadcast, vec![SessionState::Paused, SessionState::Active, SessionState::Finished]);
}

#[test]
fn lifecycle_events_for_unknown_session_are_ignored() {
    let mut t = server_app();
    launch(&mut t, "1");
    t.drain::<SessionUpdate>();

    let other = Uuid::from_u128(0xdead);
    t.send(PauseSessionEvent { session_id: other })
        .send(FinishSessionEvent { session_id: other })
        .step_n(SETTLE);

    assert_eq!(sessions(&mut t)[0].state, SessionState::Active);
    assert!(t.captured::<SessionUpdate>().is_empty());
}

#[test]
fn closing_session_returns_to_menu_with_fresh_prompt() {
    let mut t = server_app();
    t.step();
    let first_prompt = menu_prompt_id(&t);
    choose(&mut t, "1");
    t.step_until(SETTLE, in_session);
    let id = sessions(&mut t)[0].session_id;

    t.send(CloseSessionEvent { session_id: id });
    t.step_until(SETTLE, in_menu);
    t.step();

    assert!(sessions(&mut t).is_empty());
    assert_eq!(t.count::<ServerMenuWorkflowMarker>(), 1);
    let second_prompt = menu_prompt_id(&t);
    assert_ne!(second_prompt, first_prompt);
    assert_eq!(t.captured::<ActorPromptRequestEvent>().len(), 2);

    let closed = t.captured::<SessionUpdate>().last().unwrap();
    assert_eq!(closed.session.state, SessionState::Finished);
}

// --- gamepad switcher -------------------------------------------------------

#[test]
fn gamepad_west_cycles_apps_then_returns_to_menu() {
    let mut t = server_app();
    t.step();

    press(&mut t, Btn::West);
    t.step_until(SETTLE, in_session);
    t.step();
    assert_eq!(sessions(&mut t).iter().map(|s| s.game_id).collect::<Vec<_>>(), vec![HUNTER]);

    press(&mut t, Btn::West);
    t.step_until(SETTLE, |w| {
        let mut q = w.try_query::<&Session>().unwrap();
        q.iter(w).map(|s| s.game_id).collect::<Vec<_>>() == vec![SNAKE]
    });
    t.step_n(2);
    t.assert_state(ServerState::Session);
    assert_eq!(sessions(&mut t).len(), 1);

    press(&mut t, Btn::West);
    t.step_until(SETTLE, in_menu);
    t.step();
    assert!(sessions(&mut t).is_empty());
    assert_eq!(t.count::<ServerMenuWorkflowMarker>(), 1);
}

#[test]
fn other_buttons_and_releases_do_not_switch() {
    let mut t = server_app();
    t.step();

    press(&mut t, Btn::South);
    t.send(GamepadButtonEvent { client_id: 0, button: Btn::West, pressed: false });
    t.step_n(SETTLE);

    t.assert_state(ServerState::Menu);
    assert!(t.captured::<InitSessionEvent>().is_empty());
}

/// Same input script, same frame-by-frame trace of (ServerState, running game ids).
#[test]
fn gamepad_cycle_is_frame_deterministic() {
    fn run() -> Vec<(ServerState, Vec<u16>)> {
        let mut t = server_app();
        let mut trace = Vec::new();
        for frame in 0..30 {
            if frame % 8 == 1 {
                press(&mut t, Btn::West);
            }
            t.step();
            let mut ids: Vec<u16> = sessions(&mut t).iter().map(|s| s.game_id).collect();
            ids.sort();
            trace.push((t.state::<ServerState>(), ids));
        }
        trace
    }

    let first = run();
    assert!(first.iter().any(|(s, _)| *s == ServerState::Session));
    assert_eq!(first, run());
}
