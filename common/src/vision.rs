//! Camera-driven reactivity: what the camera sees becomes pipeline entities, triggers and signals.
//!
//! The server's vision plugin writes [`VisionDetection`], [`VisionLost`] and [`VisionSignal`]
//! messages in [`VisionSet::Ingest`]; this module turns them into reactive visuals in
//! [`VisionSet::React`], before the pipeline binds, so a reaction shows in the same frame. Tests
//! and other inputs can write the same messages without a camera.
//!
//! For every tracked object labelled `<label>` (`laser`, `person` ...):
//! - broadcast triggers `vision.<label>.appeared` (strength = confidence) and `vision.<label>.lost`,
//! - global signals `vision.<label>.present` (0 or 1) and `vision.<label>.confidence` (strongest),
//! - with [`VisionAppExt::follow_vision`], an entity that follows the object and is drawn by a
//!   pipeline, with signals `self.confidence` and `self.speed`,
//! - with [`VisionAppExt::on_vision_appear`], a one-shot effect where the object appeared.
//!
//! Frame-wide analyzer values (`motion` ...) become global signals `vision.<name>`.
//!
//! Games that want camera input for gameplay read [`VisionDetection`] directly.

use crate::path::PathRenderable;
use crate::pipeline::plugin::{DespawnOnFinish, PathPipelineSet, ReactionTriggered, ReactiveAppExt, TriggerTarget, UsePipeline};
use crate::pipeline::Signals;
use crate::scene::SceneEntity;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Stable identity of a tracked object, assigned by the camera's tracker
pub type TrackId = u64;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisionSet {
    /// Camera results are written as vision messages
    Ingest,
    /// Vision messages become entities, triggers and signals
    React,
}

/// A tracked object seen in one camera frame (several per Bevy frame at high camera rates).
#[derive(Message, Debug, Clone, PartialEq)]
pub struct VisionDetection {
    pub track: TrackId,
    /// The camera reports this track for the first time
    pub appeared: bool,
    pub label: String,
    /// Centre on the scene plane, world space
    pub world_pos: Vec3,
    /// Width and height on the scene plane in metres
    pub size_m: Vec2,
    /// Normalised camera coordinates, (0, 0) top-left
    pub camera_pos: Vec2,
    /// Projector coordinates, -1..1, +Y down
    pub galvo_pos: Vec2,
    /// 0..1
    pub confidence: f32,
    /// Camera clock
    pub timestamp_ms: u64,
}

/// A tracked object the camera no longer sees.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct VisionLost {
    pub track: TrackId,
    pub label: String,
}

/// A frame-wide analyzer value, 0..1, published as global signal `vision.<name>`.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct VisionSignal {
    pub name: String,
    pub value: f32,
}

/// An entity following a tracked object (see [`VisionAppExt::follow_vision`]). Its `Transform` is
/// kept on the object, in scene space.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct VisionTrack {
    pub id: TrackId,
    pub label: String,
    pub confidence: f32,
    pub world_pos: Vec3,
    pub size_m: Vec2,
    pub speed_m_s: f32,
}

/// Speed that reads as 1 in `self.speed`
const FULL_SPEED_M_S: f32 = 2.0;

/// Which pipelines draw what the camera sees, by label.
#[derive(Resource, Default, Debug)]
pub struct VisionBindings {
    follow: HashMap<String, String>,
    on_appear: HashMap<String, Vec<String>>,
}

#[derive(Resource, Debug, Clone)]
pub struct VisionReactSettings {
    /// A track not updated for this long counts as lost, even without a `VisionLost` (e.g. the
    /// camera stopped)
    pub stale_after_secs: f64,
}

impl Default for VisionReactSettings {
    fn default() -> Self {
        Self { stale_after_secs: 0.5 }
    }
}

struct TrackState {
    track: VisionTrack,
    timestamp_ms: u64,
    last_seen_secs: f64,
    follower: Option<Entity>,
}

/// Objects currently tracked, whether or not an entity follows them.
#[derive(Resource, Default)]
pub struct VisionTracks {
    states: HashMap<TrackId, TrackState>,
    /// Every label seen so far, so its signals fall back to 0 once nothing is tracked
    labels: HashSet<String>,
}

impl VisionTracks {
    pub fn get(&self, id: TrackId) -> Option<&VisionTrack> {
        self.states.get(&id).map(|s| &s.track)
    }

    pub fn iter(&self) -> impl Iterator<Item = &VisionTrack> {
        self.states.values().map(|s| &s.track)
    }

    /// The entity following track `id`, if a pipeline is bound to its label
    pub fn follower(&self, id: TrackId) -> Option<Entity> {
        self.states.get(&id).and_then(|s| s.follower)
    }
}

pub trait VisionAppExt {
    /// Each object labelled `label` gets an entity that follows it, drawn by `pipeline`, and
    /// despawned when the object is lost
    fn follow_vision(&mut self, label: &str, pipeline: &str) -> &mut Self;
    /// Each object labelled `label` spawns a one-shot `pipeline` effect where it appeared. The
    /// pipeline needs a `lifetime_ms`; the effect despawns when it ends
    fn on_vision_appear(&mut self, label: &str, pipeline: &str) -> &mut Self;
}

impl VisionAppExt for App {
    fn follow_vision(&mut self, label: &str, pipeline: &str) -> &mut Self {
        self.init_resource::<VisionBindings>();
        self.world_mut().resource_mut::<VisionBindings>().follow.insert(label.to_string(), pipeline.to_string());
        self
    }

    fn on_vision_appear(&mut self, label: &str, pipeline: &str) -> &mut Self {
        self.init_resource::<VisionBindings>();
        self.world_mut()
            .resource_mut::<VisionBindings>()
            .on_appear
            .entry(label.to_string())
            .or_default()
            .push(pipeline.to_string());
        self
    }
}

pub struct VisionReactivePlugin;

impl Plugin for VisionReactivePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<VisionDetection>()
            .add_message::<VisionLost>()
            .add_message::<VisionSignal>()
            .add_message::<ReactionTriggered>()
            .init_resource::<Signals>()
            .init_resource::<VisionBindings>()
            .init_resource::<VisionTracks>()
            .init_resource::<VisionReactSettings>()
            .configure_sets(Update, (VisionSet::Ingest, VisionSet::React).chain().before(PathPipelineSet::Bind))
            .add_systems(Update, react_to_vision.in_set(VisionSet::React))
            .bind_signal::<VisionTrack>("confidence", |t| t.confidence)
            .bind_signal::<VisionTrack>("speed", |t| (t.speed_m_s / FULL_SPEED_M_S).min(1.0));
    }
}

/// Spawns an entity with `pipeline` at `local` (scene space), under the scene entity if there is one
fn spawn_visual(commands: &mut Commands, scene: Option<Entity>, local: Vec3, pipeline: &str, extra: impl Bundle) -> Entity {
    let transform = Transform::from_translation(local);
    let entity = commands
        .spawn((UsePipeline::new(pipeline), transform, GlobalTransform::from(transform), Visibility::default(), PathRenderable::default(), extra))
        .id();
    if let Some(scene) = scene {
        commands.entity(scene).add_child(entity);
    }
    entity
}

fn react_to_vision(
    mut commands: Commands,
    bindings: Res<VisionBindings>,
    settings: Res<VisionReactSettings>,
    time: Res<Time<Real>>,
    mut tracks: ResMut<VisionTracks>,
    mut signals: ResMut<Signals>,
    mut detections: MessageReader<VisionDetection>,
    mut lost: MessageReader<VisionLost>,
    mut analyzer_signals: MessageReader<VisionSignal>,
    mut triggers: MessageWriter<ReactionTriggered>,
    scene: Query<(Entity, &Transform), With<SceneEntity>>,
    mut followers: Query<(&mut Transform, &mut VisionTrack), Without<SceneEntity>>,
) {
    let now = time.elapsed_secs_f64();
    let (scene_entity, world_to_scene) = match scene.single() {
        Ok((entity, transform)) => (Some(entity), transform.to_matrix().inverse()),
        Err(_) => (None, Mat4::IDENTITY),
    };
    let tracks = &mut *tracks;
    let mut updated = HashSet::new();

    for d in detections.read() {
        let local = world_to_scene.transform_point3(d.world_pos);
        let is_new = !tracks.states.contains_key(&d.track);
        let state = tracks.states.entry(d.track).or_insert_with(|| TrackState {
            track: VisionTrack {
                id: d.track,
                label: d.label.clone(),
                confidence: d.confidence,
                world_pos: d.world_pos,
                size_m: d.size_m,
                speed_m_s: 0.0,
            },
            timestamp_ms: d.timestamp_ms,
            last_seen_secs: now,
            follower: None,
        });

        if d.timestamp_ms > state.timestamp_ms {
            let dt = (d.timestamp_ms - state.timestamp_ms) as f32 / 1000.0;
            state.track.speed_m_s = state.track.world_pos.distance(d.world_pos) / dt;
        }
        state.track.confidence = d.confidence;
        state.track.world_pos = d.world_pos;
        state.track.size_m = d.size_m;
        state.timestamp_ms = d.timestamp_ms;
        state.last_seen_secs = now;
        updated.insert(d.track);

        if is_new {
            tracks.labels.insert(d.label.clone());
            triggers.write(ReactionTriggered {
                target: TriggerTarget::Broadcast,
                key: format!("vision.{}.appeared", d.label),
                strength: d.confidence,
            });
            for pipeline in bindings.on_appear.get(&d.label).into_iter().flatten() {
                spawn_visual(&mut commands, scene_entity, local, pipeline, DespawnOnFinish);
            }
            if let Some(pipeline) = bindings.follow.get(&d.label) {
                state.follower = Some(spawn_visual(&mut commands, scene_entity, local, pipeline, state.track.clone()));
            }
        }
    }

    // Followers move to their object's latest position
    for id in &updated {
        let state = &tracks.states[id];
        if let Some(Ok((mut transform, mut track))) = state.follower.map(|e| followers.get_mut(e)) {
            transform.translation = world_to_scene.transform_point3(state.track.world_pos);
            *track = state.track.clone();
        }
    }

    // Objects the camera lost, or that have gone quiet
    let mut gone: Vec<TrackId> = lost.read().map(|l| l.track).collect();
    gone.extend(
        tracks
            .states
            .iter()
            .filter(|(_, s)| now - s.last_seen_secs > settings.stale_after_secs)
            .map(|(id, _)| *id),
    );
    for id in gone {
        let Some(state) = tracks.states.remove(&id) else { continue };
        if let Some(follower) = state.follower {
            commands.entity(follower).try_despawn();
        }
        triggers.write(ReactionTriggered::broadcast(format!("vision.{}.lost", state.track.label)));
    }

    for label in &tracks.labels {
        let strongest = tracks
            .states
            .values()
            .filter(|s| &s.track.label == label)
            .map(|s| s.track.confidence)
            .fold(None, |best: Option<f32>, c| Some(best.map_or(c, |b| b.max(c))));
        signals.set(format!("vision.{label}.present"), if strongest.is_some() { 1.0 } else { 0.0 });
        signals.set(format!("vision.{label}.confidence"), strongest.unwrap_or(0.0));
    }
    for signal in analyzer_signals.read() {
        signals.set(format!("vision.{}", signal.name), signal.value);
    }
}
