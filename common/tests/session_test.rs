use bevy::asset::uuid::Uuid;
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use common::session::*;
use common::state::ServerState;
use test_harness::TestApp;

fn session_app() -> TestApp {
    let mut t = TestApp::new();
    t.add_plugins(StatesPlugin);
    t.app_mut().init_state::<ServerState>();
    t.add_plugins(SessionPlugin);
    t
}

fn registry(apps: &[(u16, &str)]) -> ChannelRegistry {
    let mut r = ChannelRegistry::default();
    for (id, name) in apps {
        r.register_app(ChannelApp { id: *id, name: name.to_string() });
    }
    r
}

#[test]
fn plugin_registers_registry_and_session_state() {
    let mut t = session_app();
    t.step();

    assert!(t.world().contains_resource::<ChannelRegistry>());
    t.assert_state(SessionState::Active);
    t.assert_state(ServerState::Menu);
}

#[test]
fn plugin_messages_are_delivered_to_systems_on_next_step() {
    #[derive(Resource, Default)]
    struct Seen(Vec<Uuid>);

    let mut t = session_app();
    t.insert_resource(Seen::default()).add_systems(
        Update,
        |mut init: MessageReader<InitSessionEvent>, mut seen: ResMut<Seen>| {
            seen.0.extend(init.read().map(|e| e.session_id));
        },
    );
    t.step();

    let id = Uuid::from_u128(1);
    t.send(InitSessionEvent { game_id: 1, session_id: id, initial_state: SessionState::Active });
    t.step();
    assert_eq!(t.resource::<Seen>().0, vec![id]);

    // A message is read exactly once, even though Bevy keeps it buffered for two frames.
    t.step_n(3);
    assert_eq!(t.resource::<Seen>().0, vec![id]);
}

#[test]
fn registry_lists_apps_sorted_by_id_and_finds_names_case_insensitively() {
    let r = registry(&[(3, "Snake"), (1, "Hunter"), (2, "Calibrate")]);

    let ids: Vec<u16> = r.all_apps().iter().map(|a| a.id).collect();
    assert_eq!(ids, vec![1, 2, 3]);
    assert_eq!(r.get_app_by_name("snake").map(|a| a.id), Some(3));
    assert_eq!(r.get_app_by_id(2).map(|a| a.name.as_str()), Some("Calibrate"));
    assert!(r.get_app_by_id(9).is_none());
}

#[test]
fn registering_same_id_replaces_app() {
    let mut r = registry(&[(1, "Old")]);
    r.register_app(ChannelApp { id: 1, name: "New".into() });
    assert_eq!(r.all_apps().len(), 1);
    assert_eq!(r.get_app_by_id(1).unwrap().name, "New");
}

#[test]
fn session_pause_and_resume_require_a_started_unfinished_session() {
    let mut s = Session::new(Uuid::from_u128(7), "hunter");
    assert_eq!(s.state, SessionState::Active);

    s.pause();
    assert_eq!(s.state, SessionState::Active, "cannot pause before start");

    s.start();
    let started_at = s.start_timestamp;
    assert!(started_at.is_some());

    s.pause();
    assert_eq!(s.state, SessionState::Paused);
    s.resume();
    assert_eq!(s.state, SessionState::Active);

    s.start();
    assert_eq!(s.start_timestamp, started_at, "restart keeps original start time");

    s.stop();
    assert_eq!(s.state, SessionState::Finished);
    assert!(s.end_timestamp.is_some());

    s.pause();
    s.resume();
    assert_eq!(s.state, SessionState::Finished, "finished session is terminal");
}

#[test]
fn actor_roles() {
    let a = Actor::new("p1", Uuid::from_u128(2)).with_role("player").with_role("controller");
    assert!(a.has_role("player"));
    assert!(a.has_role("controller"));
    assert!(!a.has_role("spectator"));
}

#[test]
fn legacy_types_convert_losslessly() {
    let id = Uuid::from_u128(42);
    let actors = vec![Actor::new("p1", Uuid::from_u128(3))];

    let sa = SessionActors { session_id: id, actors: actors.clone() };
    let back: SessionActors = ActorMetaData::from(sa.clone()).into();
    assert_eq!(back, sa);

    let init: InitSessionEvent = InitGameSessionEvent {
        game_id: 5,
        game_session_uuid: id,
        initial_state: SessionState::Paused,
    }
    .into();
    assert_eq!((init.game_id, init.session_id, init.initial_state), (5, id, SessionState::Paused));

    let close: CloseSessionEvent = ExitGameEvent { game_session_uuid: id }.into();
    assert_eq!(close.session_id, id);
}
