//! Reactive laser pipeline: binds pipeline files to entities, routes world events to their
//! reactions, and evaluates their stage stacks into `UniversalPath` every frame.
//! See `docs/TRANSFORMER_PIPELINE.md`.
//!
//! Game plugins make entities reactive by
//! - giving them a [`PipelineRef`] (a `.pipeline.yaml` asset or an inline spec),
//! - publishing component values as `self.*` signals with [`ReactiveAppExt::bind_signal`],
//! - turning their messages into triggers with [`ReactiveAppExt::register_trigger_message`] or
//!   writing [`ReactionTriggered`] directly. Curator envelope tags registered with
//!   [`ReactiveAppExt::register_trigger_envelope`] reach every pipeline.
//!
//! Systems that raise triggers or change bound components should run before
//! [`PathPipelineSet::Bind`] for the reaction to show in the same frame.

use bevy::asset::{io::Reader, AssetLoader, LoadContext};
use bevy::prelude::*;
use super::{compile, Clock, EntitySignals, PathSource, PathStage, PipelineInstance, PipelineRegistry, PipelineSpec, ReactionOutput, Signals};
use crate::path::UniversalPath;
use curator_core::CuratorEventEnvelope;
use bevy::ecs::system::SystemParam;
use serde::de::DeserializeOwned;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathPipelineSet {
    /// Pipeline files are resolved into per-entity instances (spawn, hot reload)
    Bind,
    /// Global signals, component bindings and message triggers are written; newly bound
    /// entities already get their component signals in their first frame
    Signals,
    /// Triggers are routed, reactions and clocks advance
    React,
    /// Stage stacks are evaluated into `UniversalPath`
    Stages,
}

/// A parsed `.pipeline.yaml`. Each load (and hot reload) creates a new `Arc`, which is how bound
/// entities notice the change.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct PipelineAsset(pub Arc<PipelineSpec>);

/// The pipeline an entity uses.
#[derive(Component, Clone)]
pub enum PipelineRef {
    /// A `.pipeline.yaml` asset (hot-reloadable). `fallback` is used until the file has loaded,
    /// and for good when it is missing or fails to parse.
    Asset { handle: Handle<PipelineAsset>, fallback: Option<Arc<PipelineSpec>> },
    /// Built in code, e.g. by tests and generated effects
    Inline(Arc<PipelineSpec>),
}

impl PipelineRef {
    pub fn inline(spec: PipelineSpec) -> Self {
        Self::Inline(Arc::new(spec))
    }
}

/// Pipelines compiled into the binary, keyed by name, used when the asset file is unavailable.
#[derive(Resource, Default)]
pub struct PipelineLibrary {
    embedded: HashMap<String, Arc<PipelineSpec>>,
    /// Keeps loaded handles alive so effects spawned and despawned in quick succession don't
    /// reload (or re-report a missing file) every time
    handles: Mutex<HashMap<String, Handle<PipelineAsset>>>,
}

/// Looks up pipelines by name: `pipelines.get("hunter/ripple")` loads
/// `pipelines/hunter/ripple.pipeline.yaml` from the asset folder, falling back to the copy
/// embedded with [`ReactiveAppExt::embed_pipeline`].
#[derive(SystemParam)]
pub struct Pipelines<'w> {
    library: Res<'w, PipelineLibrary>,
    server: Option<Res<'w, AssetServer>>,
}

impl Pipelines<'_> {
    pub fn get(&self, name: &str) -> PipelineRef {
        let fallback = self.library.embedded.get(name).cloned();
        match &self.server {
            Some(server) => {
                let mut handles = self.library.handles.lock().unwrap_or_else(|e| e.into_inner());
                let handle = handles
                    .entry(name.to_string())
                    .or_insert_with(|| server.load(format!("pipelines/{name}.pipeline.yaml")))
                    .clone();
                PipelineRef::Asset { handle, fallback }
            }
            None => match fallback {
                Some(spec) => PipelineRef::Inline(spec),
                None => {
                    warn!("pipeline `{name}` is neither embedded nor loadable (no asset server)");
                    PipelineRef::inline(PipelineSpec::empty(name))
                }
            },
        }
    }
}

/// Live pipeline state of an entity, created from its [`PipelineRef`].
#[derive(Component)]
pub struct PathPipeline {
    pub instance: PipelineInstance,
    spec: Arc<PipelineSpec>,
}

/// Base geometry for pipelines without a `source`. Captured from the entity's `UniversalPath`
/// when the pipeline is bound; game code changes the shape by writing this, not `UniversalPath`.
#[derive(Component, Default, Clone, Deref, DerefMut)]
pub struct BasePath(pub UniversalPath);

/// Despawn the entity once its pipeline's lifetime has run out (effects such as ripples).
#[derive(Component, Default)]
pub struct DespawnOnFinish;

/// Who a trigger is for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerTarget {
    Entity(Entity),
    /// Every bound pipeline
    Broadcast,
}

/// A named moment that pipelines react to (`hit`, `pop`, `session_start` ...).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct ReactionTriggered {
    pub target: TriggerTarget,
    pub key: String,
    /// Scales envelope peaks; 1 for plain events
    pub strength: f32,
}

impl ReactionTriggered {
    pub fn entity(entity: Entity, key: impl Into<String>) -> Self {
        Self { target: TriggerTarget::Entity(entity), key: key.into(), strength: 1.0 }
    }

    pub fn broadcast(key: impl Into<String>) -> Self {
        Self { target: TriggerTarget::Broadcast, key: key.into(), strength: 1.0 }
    }
}

/// Curator envelope tags that become broadcast triggers
#[derive(Resource, Default)]
struct EnvelopeTriggers(HashSet<String>);

pub struct PathPipelinePlugin;

impl Plugin for PathPipelinePlugin {
    fn build(&self, app: &mut App) {
        let mut registry = PipelineRegistry::default();
        super::sources::register(&mut registry);
        super::stages::register(&mut registry);

        app.insert_resource(registry)
            .init_resource::<Signals>()
            .init_resource::<PipelineLibrary>()
            .init_resource::<EnvelopeTriggers>()
            .add_message::<ReactionTriggered>()
            .add_message::<CuratorEventEnvelope>()
            .configure_sets(
                Update,
                (PathPipelineSet::Bind, PathPipelineSet::Signals, PathPipelineSet::React, PathPipelineSet::Stages).chain(),
            )
            .add_systems(Update, bind_pipelines.in_set(PathPipelineSet::Bind))
            .add_systems(
                Update,
                (envelope_triggers, route_triggers, tick_pipelines).chain().in_set(PathPipelineSet::React),
            )
            .add_systems(Update, evaluate_pipelines.in_set(PathPipelineSet::Stages));

        if app.is_plugin_added::<AssetPlugin>() {
            app.init_asset::<PipelineAsset>().register_asset_loader(PipelineAssetLoader);
        }
    }
}

/// Plugin-facing registration API.
pub trait ReactiveAppExt {
    /// Makes a stage available to pipeline files under `key`
    fn register_pipeline_stage<T: PathStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self;
    /// Makes a source available to pipeline files under `key`
    fn register_pipeline_source<T: PathSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self;
    /// Publishes a component value (0..1) as the entity signal `self.<key>` whenever it changes
    fn bind_signal<C: Component>(&mut self, key: &'static str, read: fn(&C) -> f32) -> &mut Self;
    /// Turns messages of type `M` into trigger `key`; `route` picks the target and strength, or
    /// skips the message with `None`
    fn register_trigger_message<M: Message>(&mut self, key: &'static str, route: fn(&M) -> Option<(TriggerTarget, f32)>) -> &mut Self;
    /// Curator envelopes with this tag become broadcast triggers `tag` and `tag.<variant>`, where
    /// the variant is the payload's enum variant name (`hunter_phase.TitleAnnouncement`)
    fn register_trigger_envelope(&mut self, tag: &str) -> &mut Self;
    /// Compiles a pipeline file into the binary as the fallback for [`Pipelines::get`]:
    /// `app.embed_pipeline(include_str!("../assets/pipelines/hunter/ripple.pipeline.yaml"))`.
    /// Panics on invalid YAML, so a broken embedded pipeline fails at startup.
    fn embed_pipeline(&mut self, yaml: &str) -> &mut Self;
}

impl ReactiveAppExt for App {
    fn register_pipeline_stage<T: PathStage + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.world_mut().resource_mut::<PipelineRegistry>().register_stage::<T>(key);
        self
    }

    fn register_pipeline_source<T: PathSource + DeserializeOwned>(&mut self, key: &str) -> &mut Self {
        self.world_mut().resource_mut::<PipelineRegistry>().register_source::<T>(key);
        self
    }

    fn bind_signal<C: Component>(&mut self, key: &'static str, read: fn(&C) -> f32) -> &mut Self {
        self.add_systems(
            Update,
            (move |mut query: Query<(&C, &mut EntitySignals), Or<(Changed<C>, Added<EntitySignals>)>>| {
                for (component, mut signals) in &mut query {
                    signals.set(key, read(component));
                }
            })
            .in_set(PathPipelineSet::Signals),
        )
    }

    fn register_trigger_message<M: Message>(&mut self, key: &'static str, route: fn(&M) -> Option<(TriggerTarget, f32)>) -> &mut Self {
        self.add_systems(
            Update,
            (move |mut messages: MessageReader<M>, mut triggers: MessageWriter<ReactionTriggered>| {
                for message in messages.read() {
                    if let Some((target, strength)) = route(message) {
                        triggers.write(ReactionTriggered { target, key: key.to_string(), strength });
                    }
                }
            })
            .in_set(PathPipelineSet::Signals),
        )
    }

    fn register_trigger_envelope(&mut self, tag: &str) -> &mut Self {
        self.world_mut().resource_mut::<EnvelopeTriggers>().0.insert(tag.to_string());
        self
    }

    fn embed_pipeline(&mut self, yaml: &str) -> &mut Self {
        let spec = PipelineSpec::from_yaml(yaml).unwrap_or_else(|e| panic!("embedded pipeline: {e}"));
        self.init_resource::<PipelineLibrary>();
        self.world_mut().resource_mut::<PipelineLibrary>().embedded.insert(spec.name.clone(), Arc::new(spec));
        self
    }
}

/// Loads `*.pipeline.yaml` files into [`PipelineAsset`]s. Validation against the registry
/// happens when the pipeline is bound, so a broken file keeps the last good version live.
#[derive(Default, TypePath)]
pub struct PipelineAssetLoader;

impl AssetLoader for PipelineAssetLoader {
    type Asset = PipelineAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(&self, reader: &mut dyn Reader, _settings: &(), load_context: &mut LoadContext<'_>) -> Result<PipelineAsset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        PipelineSpec::from_yaml(&text)
            .map(|spec| PipelineAsset(Arc::new(spec)))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {e}", load_context.path().display())))
    }

    fn extensions(&self) -> &[&str] {
        &["pipeline.yaml"]
    }
}

/// Creates or updates `PathPipeline` whenever an entity's spec is new or its file changed.
fn bind_pipelines(
    mut commands: Commands,
    registry: Res<PipelineRegistry>,
    assets: Option<Res<Assets<PipelineAsset>>>,
    mut query: Query<(Entity, &PipelineRef, Option<&mut PathPipeline>, Option<&UniversalPath>, Has<BasePath>, Has<EntitySignals>)>,
    mut failed: Local<Vec<Weak<PipelineSpec>>>,
) {
    failed.retain(|weak| weak.strong_count() > 0);
    for (entity, pipeline_ref, pipeline, path, has_base, has_signals) in &mut query {
        let spec = match pipeline_ref {
            PipelineRef::Inline(spec) => spec.clone(),
            PipelineRef::Asset { handle, fallback } => match (assets.as_ref().and_then(|a| a.get(handle)), fallback) {
                (Some(asset), _) => asset.0.clone(),
                (None, Some(fallback)) => fallback.clone(),
                (None, None) => continue, // still loading
            },
        };
        if pipeline.as_ref().is_some_and(|p| Arc::ptr_eq(&p.spec, &spec)) {
            continue;
        }
        if failed.iter().any(|weak| weak.as_ptr() == Arc::as_ptr(&spec)) {
            continue;
        }
        let plan = match compile(&spec, &registry) {
            Ok(plan) => Arc::new(plan),
            Err(errors) => {
                let list: Vec<String> = errors.iter().map(|e| format!("  {e}")).collect();
                error!("pipeline `{}` is invalid, keeping the previous version:\n{}", spec.name, list.join("\n"));
                failed.push(Arc::downgrade(&spec));
                continue;
            }
        };
        match pipeline {
            Some(mut existing) => {
                match existing.instance.rebind(plan, &registry) {
                    Ok(()) => info!("pipeline `{}` reloaded", spec.name),
                    Err(e) => error!("pipeline `{}`: {e}", spec.name),
                }
                existing.spec = spec;
            }
            None => {
                let mut instance = match PipelineInstance::new(plan, &registry) {
                    Ok(instance) => instance,
                    Err(e) => {
                        error!("pipeline `{}`: {e}", spec.name);
                        failed.push(Arc::downgrade(&spec));
                        continue;
                    }
                };
                let mut e = commands.entity(entity);
                if !instance.has_source() {
                    let base = path.cloned().unwrap_or_default();
                    instance.set_base(&base);
                    if !has_base {
                        e.insert(BasePath(base));
                    }
                }
                if path.is_none() {
                    e.insert(UniversalPath::default());
                }
                if !has_signals {
                    e.insert(EntitySignals::default());
                }
                e.insert(PathPipeline { instance, spec });
            }
        }
    }
}

/// Curator envelopes with registered tags → broadcast triggers
fn envelope_triggers(
    tags: Res<EnvelopeTriggers>,
    mut envelopes: MessageReader<CuratorEventEnvelope>,
    mut triggers: MessageWriter<ReactionTriggered>,
) {
    for envelope in envelopes.read() {
        if !tags.0.contains(&envelope.tag) {
            continue;
        }
        triggers.write(ReactionTriggered::broadcast(envelope.tag.clone()));
        let variant = match &envelope.payload {
            serde_json::Value::String(name) => Some(name.clone()),
            serde_json::Value::Object(map) if map.len() == 1 => map.keys().next().cloned(),
            _ => None,
        };
        if let Some(variant) = variant {
            triggers.write(ReactionTriggered::broadcast(format!("{}.{variant}", envelope.tag)));
        }
    }
}

fn route_triggers(mut triggers: MessageReader<ReactionTriggered>, mut pipelines: Query<&mut PathPipeline>) {
    for trigger in triggers.read() {
        match trigger.target {
            TriggerTarget::Entity(entity) => {
                if let Ok(mut pipeline) = pipelines.get_mut(entity) {
                    pipeline.instance.trigger(trigger.key.clone(), trigger.strength);
                }
            }
            TriggerTarget::Broadcast => {
                for mut pipeline in &mut pipelines {
                    pipeline.instance.trigger(trigger.key.clone(), trigger.strength);
                }
            }
        }
    }
}

fn tick_pipelines(
    mut commands: Commands,
    game: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    registry: Res<PipelineRegistry>,
    mut pipelines: Query<(Entity, &mut PathPipeline, &mut EntitySignals, Has<DespawnOnFinish>)>,
    mut envelopes: MessageWriter<CuratorEventEnvelope>,
) {
    for (entity, mut pipeline, mut signals, despawn_on_finish) in &mut pipelines {
        let dt = match pipeline.instance.plan().clock {
            Clock::Game => game.delta_secs(),
            Clock::Real => real.delta_secs(),
        };
        for output in pipeline.instance.tick(dt, &mut signals, &registry) {
            match output {
                ReactionOutput::Emit { tag, payload } => {
                    let mut envelope = CuratorEventEnvelope::new(tag, payload);
                    envelope.source_entity = Some(entity);
                    envelopes.write(envelope);
                }
            }
        }
        if despawn_on_finish && pipeline.instance.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn evaluate_pipelines(
    game: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    global: Res<Signals>,
    mut pipelines: Query<(&mut PathPipeline, &EntitySignals, Option<Ref<BasePath>>, &mut UniversalPath)>,
) {
    pipelines.par_iter_mut().for_each(|(mut pipeline, signals, base, mut path)| {
        if let Some(base) = base {
            if base.is_changed() {
                pipeline.instance.set_base(&base.0);
            }
        }
        let (elapsed, dt) = match pipeline.instance.plan().clock {
            Clock::Game => (game.elapsed_secs(), game.delta_secs()),
            Clock::Real => (real.elapsed_secs(), real.delta_secs()),
        };
        if pipeline.instance.evaluate(signals, &global, elapsed, dt, path.bypass_change_detection()) {
            path.set_changed();
        }
    });
}
