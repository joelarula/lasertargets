use std::time::Duration;

use bevy::prelude::*;
use test_harness::{TestApp, DEFAULT_TIMESTEP};

#[derive(Resource, Default)]
struct FixedTicks(u32);

#[derive(Resource, Default)]
struct Deltas(Vec<Duration>);

#[derive(Message, Clone, Debug, PartialEq)]
struct Ping(u32);

#[test]
fn first_step_has_zero_delta_then_exact_timestep() {
    let mut t = TestApp::new();
    t.insert_resource(Deltas::default())
        .add_systems(Update, |time: Res<Time>, mut d: ResMut<Deltas>| d.0.push(time.delta()));

    t.step_n(4);

    assert_eq!(
        t.resource::<Deltas>().0,
        vec![Duration::ZERO, DEFAULT_TIMESTEP, DEFAULT_TIMESTEP, DEFAULT_TIMESTEP]
    );
    assert_eq!(t.resource::<Time>().elapsed(), DEFAULT_TIMESTEP * 3);
}

#[test]
fn fixed_update_runs_once_per_step_at_matching_rate() {
    let mut t = TestApp::new();
    t.insert_resource(Time::<Fixed>::from_duration(DEFAULT_TIMESTEP))
        .init_ticks()
        .add_systems(FixedUpdate, |mut n: ResMut<FixedTicks>| n.0 += 1);

    t.step(); // startup frame, dt = 0
    assert_eq!(t.resource::<FixedTicks>().0, 0);

    t.step_n(10);
    assert_eq!(t.resource::<FixedTicks>().0, 10);
}

#[test]
fn advance_covers_duration_in_whole_frames() {
    let mut t = TestApp::with_timestep(Duration::from_millis(100));
    t.step();
    t.advance(Duration::from_millis(250));
    assert_eq!(t.frames(), 1 + 3);
    assert_eq!(t.resource::<Time>().elapsed(), Duration::from_millis(300));
}

#[test]
fn capture_records_messages_across_frames() {
    let mut t = TestApp::new();
    t.app_mut().add_message::<Ping>();
    t.capture::<Ping>();

    t.send(Ping(1)).step();
    t.send(Ping(2)).send(Ping(3)).step();
    t.step();

    assert_eq!(t.captured::<Ping>(), &[Ping(1), Ping(2), Ping(3)]);
    assert_eq!(t.drain::<Ping>().len(), 3);
    assert!(t.captured::<Ping>().is_empty());
}

#[test]
fn step_until_reports_frame_count() {
    let mut t = TestApp::new();
    t.init_ticks()
        .add_systems(Update, |mut n: ResMut<FixedTicks>| n.0 += 1);

    let frames = t.step_until(100, |w| w.resource::<FixedTicks>().0 == 7);
    assert_eq!(frames, 7);
}

#[test]
#[should_panic(expected = "condition not met within 5 frames")]
fn step_until_panics_when_condition_never_holds() {
    let mut t = TestApp::new();
    t.step_until(5, |_| false);
}

trait InitTicks {
    fn init_ticks(&mut self) -> &mut Self;
}

impl InitTicks for TestApp {
    fn init_ticks(&mut self) -> &mut Self {
        self.insert_resource(FixedTicks::default())
    }
}
