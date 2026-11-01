//! Deterministic, frame-stepped Bevy test harness.
//!
//! [`TestApp`] wraps a headless [`App`] built on [`MinimalPlugins`] and makes it
//! deterministic:
//!
//! * **Fixed time** – [`TimeUpdateStrategy::ManualDuration`] advances the clock by
//!   exactly one timestep per [`TestApp::step`], so `Time::delta_secs()`,
//!   `Time<Fixed>` and anything timer-driven behave identically on every run.
//! * **Fixed system order** – every schedule is switched to the single-threaded
//!   executor before each step, so ambiguous systems always run in the same order.
//! * **Observable messages** – [`TestApp::capture`] records every message of a type
//!   across frames so tests can assert on what was emitted, not only on end state.
//!
//! Bevy's clock reports a zero delta on the very first update. The first `step()`
//! therefore runs `Startup` and the initial state transitions with `dt == 0`;
//! every following step advances time by exactly [`TestApp::timestep`].

use std::fmt::Debug;
use std::time::Duration;

use bevy::app::{Plugins, PluginsState};
use bevy::ecs::message::{Message, MessageReader};
use bevy::ecs::schedule::ExecutorKind;
use bevy::prelude::*;
use bevy::state::state::FreelyMutableState;
use bevy::time::TimeUpdateStrategy;

/// Default timestep: 50 Hz, matching the server's fixed update rate.
pub const DEFAULT_TIMESTEP: Duration = Duration::from_millis(20);

/// All messages of type `M` observed since capture started (or since the last drain).
#[derive(Resource)]
pub struct Captured<M: Message> {
    items: Vec<M>,
}

impl<M: Message> Default for Captured<M> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

fn capture_system<M: Message + Clone>(mut reader: MessageReader<M>, mut captured: ResMut<Captured<M>>) {
    captured.items.extend(reader.read().cloned());
}

/// A headless Bevy app that advances in deterministic, fixed-size frames.
pub struct TestApp {
    app: App,
    timestep: Duration,
    frames: u32,
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TestApp {
    /// Minimal headless app stepping at [`DEFAULT_TIMESTEP`].
    pub fn new() -> Self {
        Self::with_timestep(DEFAULT_TIMESTEP)
    }

    /// Minimal headless app stepping by `timestep` per frame.
    pub fn with_timestep(timestep: Duration) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
        Self { app, timestep, frames: 0 }
    }

    /// Wrap an app the caller has already built. Time is still forced to manual stepping.
    pub fn from_app(mut app: App, timestep: Duration) -> Self {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(timestep));
        Self { app, timestep, frames: 0 }
    }

    pub fn timestep(&self) -> Duration {
        self.timestep
    }

    /// Number of frames stepped so far.
    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    pub fn world(&self) -> &World {
        self.app.world()
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        self.app.add_plugins(plugins);
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.app.insert_resource(resource);
        self
    }

    pub fn add_systems<M>(
        &mut self,
        schedule: impl bevy::ecs::schedule::ScheduleLabel,
        systems: impl IntoScheduleConfigs<bevy::ecs::system::ScheduleSystem, M>,
    ) -> &mut Self {
        self.app.add_systems(schedule, systems);
        self
    }

    /// Start recording every `M` message. Read them with [`captured`](Self::captured)
    /// or [`drain`](Self::drain). Call before stepping so nothing is missed.
    pub fn capture<M: Message + Clone>(&mut self) -> &mut Self {
        if !self.app.world().contains_resource::<Captured<M>>() {
            self.app
                .init_resource::<Captured<M>>()
                .add_systems(Last, capture_system::<M>);
        }
        self
    }

    // --- stepping -------------------------------------------------------------

    /// Run exactly one frame.
    pub fn step(&mut self) -> &mut Self {
        self.finish_plugins();
        self.force_single_threaded();
        self.app.update();
        self.frames += 1;
        self
    }

    /// Run exactly `n` frames.
    pub fn step_n(&mut self, n: u32) -> &mut Self {
        for _ in 0..n {
            self.step();
        }
        self
    }

    /// Run as many whole frames as needed to cover `duration` of simulated time.
    pub fn advance(&mut self, duration: Duration) -> &mut Self {
        let n = duration.as_nanos().div_ceil(self.timestep.as_nanos()) as u32;
        self.step_n(n)
    }

    /// Step until `predicate` holds, up to `max_frames`.
    /// Returns the number of frames stepped, or panics if the predicate never held.
    #[track_caller]
    pub fn step_until(&mut self, max_frames: u32, mut predicate: impl FnMut(&World) -> bool) -> u32 {
        for n in 1..=max_frames {
            self.step();
            if predicate(self.world()) {
                return n;
            }
        }
        panic!("condition not met within {max_frames} frames");
    }

    // --- messages -------------------------------------------------------------

    /// Write a message as if a system had sent it; it is visible to readers on the next step.
    pub fn send<M: Message>(&mut self, message: M) -> &mut Self {
        self.app.world_mut().write_message(message);
        self
    }

    /// Messages of type `M` recorded so far. Requires [`capture`](Self::capture).
    #[track_caller]
    pub fn captured<M: Message>(&self) -> &[M] {
        &self
            .world()
            .get_resource::<Captured<M>>()
            .unwrap_or_else(|| panic!("call capture::<{}>() first", std::any::type_name::<M>()))
            .items
    }

    /// Take and clear the recorded messages of type `M`. Requires [`capture`](Self::capture).
    #[track_caller]
    pub fn drain<M: Message>(&mut self) -> Vec<M> {
        let mut captured = self
            .world_mut()
            .get_resource_mut::<Captured<M>>()
            .unwrap_or_else(|| panic!("call capture::<{}>() first", std::any::type_name::<M>()));
        std::mem::take(&mut captured.items)
    }

    // --- world helpers --------------------------------------------------------

    #[track_caller]
    pub fn resource<R: Resource>(&self) -> &R {
        self.world().resource::<R>()
    }

    #[track_caller]
    pub fn resource_mut<R: Resource>(&mut self) -> Mut<'_, R> {
        self.world_mut().resource_mut::<R>()
    }

    /// Current value of state `S`.
    #[track_caller]
    pub fn state<S: States + Clone>(&self) -> S {
        self.world().resource::<State<S>>().get().clone()
    }

    /// Queue a transition to `state`; it is applied during the next step.
    pub fn set_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        self.world_mut().resource_mut::<NextState<S>>().set(state);
        self
    }

    /// Clone every `C` component currently in the world.
    pub fn query<C: Component + Clone>(&mut self) -> Vec<C> {
        let world = self.world_mut();
        let mut query = world.query::<&C>();
        query.iter(world).cloned().collect()
    }

    /// Number of entities with component `C`.
    pub fn count<C: Component>(&mut self) -> usize {
        let world = self.world_mut();
        let mut query = world.query_filtered::<(), With<C>>();
        query.iter(world).count()
    }

    /// Assert `S` currently equals `expected`, reporting the frame number on failure.
    #[track_caller]
    pub fn assert_state<S: States + Clone + Debug>(&self, expected: S) {
        let actual = self.state::<S>();
        assert_eq!(actual, expected, "unexpected {} at frame {}", std::any::type_name::<S>(), self.frames);
    }

    // --- internals ------------------------------------------------------------

    fn finish_plugins(&mut self) {
        if self.app.plugins_state() == PluginsState::Cleaned {
            return;
        }
        while self.app.plugins_state() == PluginsState::Adding {
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        self.app.finish();
        self.app.cleanup();
    }

    /// Schedules can be created lazily by plugins and state registration, so re-apply
    /// on every step. `set_executor_kind` is a no-op when the kind is already set.
    fn force_single_threaded(&mut self) {
        let mut schedules = self.app.world_mut().resource_mut::<Schedules>();
        for (_, schedule) in schedules.iter_mut() {
            schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        }
    }
}
