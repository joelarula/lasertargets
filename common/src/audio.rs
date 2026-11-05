//! Game audio: what sounds exist, and which outputs play them.
//!
//! Sounds are identified by name (`laser_hit`, `sci_fi_lock` …); [`SoundId`] is the typed
//! shortcut for the built-in ones. What a sound *is* lives in its composition,
//! `assets/sounds/<name>.sound.yaml` (see [`crate::sound`]), which every output can render. Game
//! code raises [`PlaySoundEvent`] / [`PlaySound`] / [`StopSound`], or pipelines raise triggers
//! that `audio.routing.on_trigger` / `stop_on_trigger` map to sounds. The [`AudioRoutingPlugin`] decides
//! for each sound which [`AudioOutput`]s play it and sends each one a [`RoutedSound`]:
//!
//! ```text
//! PlaySoundEvent ──┐                       ┌─▶ RoutedSound { output: Speakers } → wav files
//! triggers ────────┼─▶ audio.routing ──────┤
//!                  │   (per-sound outputs) └─▶ RoutedSound { output: Galvo }    → galvo cues
//!                  └─▶ trigger `sound.<name>` (visuals react to any sound)
//! ```
//!
//! An output that has nothing under a sound's name skips it. Outputs live in their own crates
//! (speakers: `server-core`, galvo: `galvo-audio`) and only read [`RoutedSound`].
//!
//! Sounds play in parallel: every request starts a new copy, on every routed output at once, on
//! top of the live stream. `max_parallel` caps how many copies of one sound overlap (rapid
//! laser hits); past it the oldest copy stops.
//!
//! The live stream ([`crate::audio_stream::LiveAudioFrame`]) is not routed per frame: each
//! output reads it and plays it when `audio.routing.stream` lists that output.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::audio_stream::LiveAudioFrame;
use crate::sound::{SoundPlugin, SoundSpec};
use std::sync::Arc;
use crate::pipeline::plugin::{PathPipelineSet, ReactionTriggered, TriggerTarget};
use crate::settings::{ConfigAppExt, ConfigSection};

/// Identifiers for audio sound effects and cues in LaserTargets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundId {
    /// Optical laser strike confirmation
    LaserHit,
    /// Minigame target spawned
    TargetSpawn,
    /// Minigame target successfully hit
    TargetHit,
    /// Minigame target timed out / missed
    TargetMiss,
    /// Match countdown ticks (3, 2, 1)
    CountdownTick,
    /// Match start / go sound
    CountdownGo,
    /// Round over / game over
    GameOver,
    /// Snake minigame pellet eaten
    PelletEat,
    /// Snake crash / collision
    SnakeCrash,
    /// Calibration target corner detected
    CalibrationCornerAcquired,
    /// Calibration sequence succeeded
    CalibrationSuccess,
    /// Warning / error tone
    Alert,
}

impl SoundId {
    pub const ALL: [SoundId; 12] = [
        SoundId::LaserHit,
        SoundId::TargetSpawn,
        SoundId::TargetHit,
        SoundId::TargetMiss,
        SoundId::CountdownTick,
        SoundId::CountdownGo,
        SoundId::GameOver,
        SoundId::PelletEat,
        SoundId::SnakeCrash,
        SoundId::CalibrationCornerAcquired,
        SoundId::CalibrationSuccess,
        SoundId::Alert,
    ];

    /// Name used by routing, the speaker sound bank and galvo cues
    pub fn name(self) -> &'static str {
        match self {
            SoundId::LaserHit => "laser_hit",
            SoundId::TargetSpawn => "target_spawn",
            SoundId::TargetHit => "target_hit",
            SoundId::TargetMiss => "target_miss",
            SoundId::CountdownTick => "countdown_tick",
            SoundId::CountdownGo => "countdown_go",
            SoundId::GameOver => "game_over",
            SoundId::PelletEat => "pellet_eat",
            SoundId::SnakeCrash => "snake_crash",
            SoundId::CalibrationCornerAcquired => "calibration_corner_acquired",
            SoundId::CalibrationSuccess => "calibration_success",
            SoundId::Alert => "alert",
        }
    }
}

/// A place sound comes out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioOutput {
    /// The host's sound device (wav files, `server-core` audio plugin)
    Speakers,
    /// The galvo scanners themselves (`galvo-audio` crate)
    Galvo,
}

/// Global audio settings loaded from YAML (`audio:`)
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfiguration {
    /// Master enable switch for audio output
    pub enabled: bool,
    /// Master volume scale [0.0, 1.0]
    pub volume: f32,
    /// Enable dynamic stereo panning based on horizontal coordinates [-1.0, 1.0]
    pub spatial_panning: bool,
    /// Speaker volume of the live stream, on top of `volume`
    #[serde(default = "one")]
    pub stream_volume: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for AudioConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            spatial_panning: true,
            stream_volume: 1.0,
        }
    }
}

impl ConfigSection for AudioConfiguration {
    const KEY: &'static str = "audio";
}

/// Which outputs play which sounds (`audio.routing`).
///
/// ```yaml
/// audio:
///   routing:
///     default: [speakers]                   # sounds not listed below
///     sounds:
///       countdown_tick: [galvo]             # tick from the scanners only
///       laser_hit: [speakers, galvo]        # both, mixed
///       alert: []                           # muted
///     on_trigger:                           # pipeline trigger → sound
///       snake.gem_eaten: pellet_eat
///       hunter.lock_on: sci_fi_lock         # a galvo-only cue
///     stream: [speakers, galvo]             # live audio stream (receiver mode)
///     max_parallel: 4                       # copies of one sound at once; the oldest stops
///     parallel:
///       laser_hit: 2
///       countdown_tick: 1
/// ```
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AudioRouting {
    /// Outputs for sounds without an entry in `sounds`
    pub default: Vec<AudioOutput>,
    /// Outputs per sound name; an empty list mutes the sound
    pub sounds: BTreeMap<String, Vec<AudioOutput>>,
    /// Pipeline trigger key → sound name. Keys starting with `sound.` are ignored, because the
    /// router raises those itself
    pub on_trigger: BTreeMap<String, String>,
    /// Pipeline trigger key → sound name to stop (instruments, drones)
    pub stop_on_trigger: BTreeMap<String, String>,
    /// Outputs that play the live audio stream
    pub stream: Vec<AudioOutput>,
    /// How many copies of one sound may play at once, per output
    pub max_parallel: usize,
    /// `max_parallel` per sound name
    pub parallel: BTreeMap<String, usize>,
}

impl Default for AudioRouting {
    fn default() -> Self {
        Self {
            default: vec![AudioOutput::Speakers],
            sounds: BTreeMap::new(),
            on_trigger: BTreeMap::new(),
            stop_on_trigger: BTreeMap::new(),
            stream: Vec::new(),
            max_parallel: 4,
            parallel: BTreeMap::new(),
        }
    }
}

impl ConfigSection for AudioRouting {
    const KEY: &'static str = "audio.routing";
}

impl AudioRouting {
    /// Outputs that play `name`
    pub fn outputs_for(&self, name: &str) -> &[AudioOutput] {
        self.sounds.get(name).unwrap_or(&self.default)
    }

    /// How many copies of `name` may overlap (at least 1)
    pub fn max_parallel_for(&self, name: &str) -> usize {
        self.parallel.get(name).copied().unwrap_or(self.max_parallel).max(1)
    }

    /// Whether the live stream goes to `output`
    pub fn streams_to(&self, output: AudioOutput) -> bool {
        self.stream.contains(&output)
    }
}

/// Message fired to trigger a one-shot audio sound effect.
#[derive(Message, Debug, Clone)]
pub struct PlaySoundEvent {
    /// Which sound cue to play
    pub sound: SoundId,
    /// Per-sound volume scale [0.0, 1.0]. Defaults to 1.0.
    pub volume: f32,
    /// Optional horizontal coordinate for stereo panning in [-1.0, 1.0].
    /// -1.0 is full Left, 0.0 is Center, 1.0 is full Right.
    pub pan: Option<f32>,
    /// Outputs for this one event, instead of `audio.routing`
    pub outputs: Option<Vec<AudioOutput>>,
}

impl PlaySoundEvent {
    pub fn new(sound: SoundId) -> Self {
        Self {
            sound,
            volume: 1.0,
            pan: None,
            outputs: None,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_pan(mut self, pan: f32) -> Self {
        self.pan = Some(pan.clamp(-1.0, 1.0));
        self
    }

    /// Plays on exactly these outputs, ignoring `audio.routing`
    pub fn on(mut self, outputs: impl IntoIterator<Item = AudioOutput>) -> Self {
        self.outputs = Some(outputs.into_iter().collect());
        self
    }
}

/// Plays a sound by name, or a composition built in code.
///
/// `PlaySound::named("sci_fi_lock")`, `PlaySound::named("theremin").on([AudioOutput::Galvo])`,
/// `PlaySound::inline(spec)`.
#[derive(Message, Debug, Clone)]
pub struct PlaySound {
    pub name: String,
    pub volume: f32,
    pub pan: Option<f32>,
    /// Outputs for this one request, instead of `audio.routing`
    pub outputs: Option<Vec<AudioOutput>>,
    /// Composition to play instead of the catalog entry named `name`
    pub inline: Option<Arc<SoundSpec>>,
}

impl PlaySound {
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), volume: 1.0, pan: None, outputs: None, inline: None }
    }

    pub fn inline(spec: SoundSpec) -> Self {
        Self { name: spec.name.clone(), volume: 1.0, pan: None, outputs: None, inline: Some(Arc::new(spec)) }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn on(mut self, outputs: impl IntoIterator<Item = AudioOutput>) -> Self {
        self.outputs = Some(outputs.into_iter().collect());
        self
    }
}

/// Fades out every copy of a sound (instruments and drones play until stopped).
#[derive(Message, Debug, Clone)]
pub struct StopSound {
    pub name: String,
    /// Outputs to stop it on; `None` = everywhere
    pub outputs: Option<Vec<AudioOutput>>,
}

impl StopSound {
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), outputs: None }
    }
}

/// A sound for one output, written by the router. Outputs read only the ones addressed to them.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct RoutedSound {
    pub name: String,
    pub output: AudioOutput,
    /// 0..1
    pub volume: f32,
    pub pan: Option<f32>,
    /// Copies of this sound allowed at once on this output; the output stops the oldest
    pub max_parallel: usize,
    /// Composition built in code, instead of the catalog entry
    pub inline: Option<Arc<SoundSpec>>,
    /// Stop the sound instead of starting it
    pub stop: bool,
}

/// Routing runs here: after trigger messages are written, before pipelines react
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioRoutingSet;

/// Turns sound requests into [`RoutedSound`]s per output. Output plugins add it through
/// [`ensure_audio_routing`], which also adds the [`SoundPlugin`] (compositions).
pub struct AudioRoutingPlugin;

impl Plugin for AudioRoutingPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<AudioRouting>()
            .add_message::<PlaySoundEvent>()
            .add_message::<PlaySound>()
            .add_message::<StopSound>()
            .add_message::<RoutedSound>()
            .add_message::<LiveAudioFrame>()
            .add_message::<ReactionTriggered>()
            .configure_sets(Update, AudioRoutingSet.after(PathPipelineSet::Signals).before(PathPipelineSet::React))
            .add_systems(Update, route_sounds.in_set(AudioRoutingSet));
    }
}

/// Adds [`AudioRoutingPlugin`] and [`SoundPlugin`] unless they are already there; output plugins
/// call this in `build`
pub fn ensure_audio_routing(app: &mut App) {
    if !app.is_plugin_added::<SoundPlugin>() {
        app.add_plugins(SoundPlugin);
    }
    if !app.is_plugin_added::<AudioRoutingPlugin>() {
        app.add_plugins(AudioRoutingPlugin);
    }
}

/// Pipelines can react to any routed sound with the broadcast trigger `sound.<name>`
pub const SOUND_TRIGGER_PREFIX: &str = "sound.";

struct Request {
    name: String,
    volume: f32,
    pan: Option<f32>,
    outputs: Option<Vec<AudioOutput>>,
    inline: Option<Arc<SoundSpec>>,
    stop: bool,
}

fn route_sounds(
    routing: Res<AudioRouting>,
    mut events: MessageReader<PlaySoundEvent>,
    mut plays: MessageReader<PlaySound>,
    mut stops: MessageReader<StopSound>,
    mut triggers: ResMut<Messages<ReactionTriggered>>,
    mut trigger_cursor: Local<bevy::ecs::message::MessageCursor<ReactionTriggered>>,
    mut routed: MessageWriter<RoutedSound>,
) {
    let play = |name: String, volume: f32, pan, outputs, inline| Request { name, volume, pan, outputs, inline, stop: false };
    let mut requests: Vec<Request> = events.read().map(|e| play(e.sound.name().to_string(), e.volume, e.pan, e.outputs.clone(), None)).collect();
    requests.extend(plays.read().map(|p| play(p.name.clone(), p.volume, p.pan, p.outputs.clone(), p.inline.clone())));
    requests.extend(stops.read().map(|s| Request { name: s.name.clone(), volume: 0.0, pan: None, outputs: s.outputs.clone(), inline: None, stop: true }));
    for trigger in trigger_cursor.read(&triggers) {
        if trigger.key.starts_with(SOUND_TRIGGER_PREFIX) {
            continue;
        }
        if let Some(name) = routing.on_trigger.get(&trigger.key) {
            requests.push(play(name.clone(), trigger.strength.clamp(0.0, 1.0), None, None, None));
        }
        if let Some(name) = routing.stop_on_trigger.get(&trigger.key) {
            requests.push(Request { name: name.clone(), volume: 0.0, pan: None, outputs: None, inline: None, stop: true });
        }
    }
    for request in requests {
        let all = [AudioOutput::Speakers, AudioOutput::Galvo];
        let outputs: &[AudioOutput] = match (&request.outputs, request.stop) {
            (Some(outputs), _) => outputs,
            // Stop everywhere, so a re-routed instrument never keeps playing somewhere
            (None, true) => &all,
            (None, false) => routing.outputs_for(&request.name),
        };
        let max_parallel = routing.max_parallel_for(&request.name);
        for &output in outputs {
            routed.write(RoutedSound {
                name: request.name.clone(),
                output,
                volume: request.volume,
                pan: request.pan,
                max_parallel,
                inline: request.inline.clone(),
                stop: request.stop,
            });
        }
        if !request.stop && !outputs.is_empty() {
            triggers.write(ReactionTriggered {
                target: TriggerTarget::Broadcast,
                key: format!("{SOUND_TRIGGER_PREFIX}{}", request.name),
                strength: request.volume,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_names_are_unique_snake_case() {
        let mut names: Vec<&str> = SoundId::ALL.iter().map(|s| s.name()).collect();
        assert!(names.iter().all(|n| n.chars().all(|c| c.is_ascii_lowercase() || c == '_')));
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SoundId::ALL.len());
    }

    #[test]
    fn routing_falls_back_to_default_and_can_mute() {
        let mut routing = AudioRouting::default();
        routing.sounds.insert("alert".into(), vec![]);
        routing.sounds.insert("countdown_tick".into(), vec![AudioOutput::Galvo]);
        assert_eq!(routing.outputs_for("laser_hit"), &[AudioOutput::Speakers]);
        assert_eq!(routing.outputs_for("countdown_tick"), &[AudioOutput::Galvo]);
        assert!(routing.outputs_for("alert").is_empty());
        assert!(!routing.streams_to(AudioOutput::Galvo));
        routing.parallel.insert("countdown_tick".into(), 0);
        assert_eq!(routing.max_parallel_for("laser_hit"), 4);
        assert_eq!(routing.max_parallel_for("countdown_tick"), 1, "never below one");
    }
}
