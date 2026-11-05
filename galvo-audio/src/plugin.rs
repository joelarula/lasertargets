//! Bevy side of galvo audio: the galvo **output**. What a sound is comes from its composition
//! (`common::sound`, `assets/sounds/*.sound.yaml`), which sounds reach the galvos comes from
//! `audio.routing`; this plugin renders them at the DAC point rate and hands them to the DAC.
//!
//! - **In:** `RoutedSound`s addressed to `AudioOutput::Galvo` (play, stop, inline
//!   compositions) and the live stream (`LiveAudioFrame`) when `audio.routing.stream` includes
//!   `galvo`. Everything plays in parallel through a `SoundMixer` (ducking, limiter).
//! - **Out:** what the galvos play is published as global signals (`galvo.level`, `galvo.x`,
//!   `galvo.y`, `galvo.pitch`, `galvo.playing`). The live stream publishes `audio.level` and
//!   `audio.peak`. Entities with an [`AudioScope`] get a scope figure as their `BasePath`.
//!   The router raises the trigger `sound.<name>` for every routed sound, whatever the output.
//! - **To the DAC:** rendered blocks go onto the [`GalvoAudioBus`]; the projector's DAC thread
//!   adds them to its points through a [`crate::bus::GalvoAudioOutput`], after the guard.
//!
//! Runs after the router and before `PathPipelineSet::React`, so a sound triggered this frame
//! is heard this frame, and the signals it writes reach this frame's reactions and stages.

use crate::bus::{GalvoAudioBus, DEFAULT_SAMPLE_RATE};
use crate::config::GalvoAudioConfig;
use crate::scope::{scope_path, ScopeInput, ScopeMode};
use crate::stream::{AudioWindow, Levels};
use bevy::prelude::*;
use common::audio::{ensure_audio_routing, AudioOutput, AudioRouting, AudioRoutingSet, RoutedSound};
use common::audio_stream::{LiveAudioFrame, Resampler, StereoFrame};
use common::pipeline::plugin::{BasePath, PathPipelineSet, ReactiveAppExt};
use common::pipeline::Signals;
use common::settings::ConfigAppExt;
use common::sound::{SoundCatalog, SoundMixer};
use std::collections::HashSet;

/// Frames kept for scopes (≈0.27 s at 30 kpps)
const WINDOW_FRAMES: usize = 8192;

/// Lowest and highest frequency `galvo.pitch` maps onto 0..1
const PITCH_SIGNAL_RANGE_HZ: (f32, f32) = (50.0, 5500.0);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GalvoAudioSet {
    /// Config changes reach the mixer and the DAC guard
    Config,
    /// Routed sounds and the stream reach the mixer
    Cues,
    /// A block is rendered and sent to the DAC; signals and scopes are updated
    Render,
}

/// The galvo mixer and its per-tick state.
#[derive(Resource)]
pub struct GalvoSynth {
    pub mixer: SoundMixer,
    resampler: Resampler,
    block: Vec<StereoFrame>,
    stream_block: Vec<StereoFrame>,
    level: [f32; 3],
    pitch: f32,
    stream_level: [f32; 2],
}

impl Default for GalvoSynth {
    fn default() -> Self {
        Self {
            mixer: SoundMixer::new(DEFAULT_SAMPLE_RATE, common::sound::Route::Orbit),
            resampler: Resampler::default(),
            block: Vec::new(),
            stream_block: Vec::new(),
            level: [0.0; 3],
            pitch: 0.0,
            stream_level: [0.0; 2],
        }
    }
}

/// Recent galvo output and live stream, at the DAC point rate.
#[derive(Resource)]
pub struct AudioWindows {
    pub galvo: AudioWindow,
    pub stream: AudioWindow,
}

impl Default for AudioWindows {
    fn default() -> Self {
        Self { galvo: AudioWindow::new(WINDOW_FRAMES), stream: AudioWindow::new(WINDOW_FRAMES) }
    }
}

/// Makes the entity an oscilloscope: its `BasePath` is rewritten every frame from recent audio.
/// Give it a pipeline for the look, e.g. `UsePipeline::new("galvo/scope")`.
#[derive(Component, Debug, Clone)]
#[require(BasePath)]
pub struct AudioScope {
    pub mode: ScopeMode,
    pub input: ScopeInput,
    /// Width of the figure in metres (local frame)
    pub size_m: f32,
    /// How much audio one figure shows
    pub window_ms: f32,
    pub max_points: usize,
    pub gain: f32,
    pub color: [u8; 3],
}

impl Default for AudioScope {
    fn default() -> Self {
        Self {
            mode: ScopeMode::Xy,
            input: ScopeInput::Galvo,
            size_m: 0.5,
            window_ms: 20.0,
            max_points: 300,
            gain: 1.0,
            color: [0, 255, 200],
        }
    }
}

pub struct GalvoAudioPlugin;

impl Plugin for GalvoAudioPlugin {
    fn build(&self, app: &mut App) {
        ensure_audio_routing(app);
        app.init_config::<GalvoAudioConfig>();
        if !app.world().contains_resource::<GalvoAudioBus>() {
            let settings = app.world().resource::<GalvoAudioConfig>().guard_settings();
            app.insert_resource(GalvoAudioBus::new(settings));
        }
        app.init_resource::<GalvoSynth>()
            .init_resource::<AudioWindows>()
            .init_resource::<Signals>()
            .add_message::<LiveAudioFrame>()
            .configure_sets(
                Update,
                (GalvoAudioSet::Config, GalvoAudioSet::Cues, GalvoAudioSet::Render)
                    .chain()
                    .after(AudioRoutingSet)
                    .before(PathPipelineSet::React),
            )
            .add_systems(Update, apply_config.run_if(resource_changed::<GalvoAudioConfig>).in_set(GalvoAudioSet::Config))
            .add_systems(Update, (play_routed_sounds, ingest_stream).chain().in_set(GalvoAudioSet::Cues))
            .add_systems(Update, (render, draw_scopes).chain().in_set(GalvoAudioSet::Render))
            .embed_pipeline(include_str!("../../assets/pipelines/galvo/scope.pipeline.yaml"))
            .embed_pipeline(include_str!("../../assets/pipelines/galvo/theremin.pipeline.yaml"));
    }
}

fn apply_config(config: Res<GalvoAudioConfig>, bus: Res<GalvoAudioBus>, mut synth: ResMut<GalvoSynth>) {
    bus.set_settings(config.guard_settings());
    let mixer = &mut synth.mixer;
    mixer.set_default_route(config.route);
    mixer.set_stream_gain(config.stream.gain);
    mixer.set_stream_duck(config.stream.duck);
    if !config.enabled {
        mixer.stop_all();
    }
}

/// Routed sounds → mixer: play (catalog or inline composition) or stop
fn play_routed_sounds(
    config: Res<GalvoAudioConfig>,
    catalog: Res<SoundCatalog>,
    mut synth: ResMut<GalvoSynth>,
    mut routed: MessageReader<RoutedSound>,
    mut reported: Local<HashSet<String>>,
) {
    for sound in routed.read().filter(|s| s.output == AudioOutput::Galvo) {
        if sound.stop {
            synth.mixer.stop(&sound.name);
            continue;
        }
        if !config.enabled {
            continue;
        }
        let result = match &sound.inline {
            Some(spec) => catalog.voice_of(spec).map(|voice| synth.mixer.play_voice(Some(&sound.name), voice.with_volume(sound.volume), sound.max_parallel)),
            None => synth.mixer.play(&catalog, &sound.name, sound.volume, sound.max_parallel),
        };
        if let Err(e) = result {
            if reported.insert(sound.name.clone()) {
                warn!("galvo: {e}");
            }
        }
    }
}

/// Live stream → stream window, `audio.*` signals, and the galvos when routed to them
fn ingest_stream(
    config: Res<GalvoAudioConfig>,
    routing: Res<AudioRouting>,
    bus: Res<GalvoAudioBus>,
    time: Res<Time<Real>>,
    mut frames: MessageReader<LiveAudioFrame>,
    mut synth: ResMut<GalvoSynth>,
    mut windows: ResMut<AudioWindows>,
    mut signals: ResMut<Signals>,
) {
    let rate = bus.sample_rate();
    let synth = &mut *synth;
    synth.stream_block.clear();
    for frame in frames.read() {
        synth.resampler.process(&frame.samples, frame.channels, frame.sample_rate, rate, &mut synth.stream_block);
    }
    let levels = Levels::of(&synth.stream_block);
    let decay = release(time.delta_secs(), 0.15);
    synth.stream_level[0] = levels.level().max(synth.stream_level[0] * decay);
    synth.stream_level[1] = levels.peak.min(1.0).max(synth.stream_level[1] * decay);
    signals.set("audio.level", synth.stream_level[0]);
    signals.set("audio.peak", synth.stream_level[1]);
    if synth.stream_block.is_empty() {
        return;
    }
    windows.stream.push(&synth.stream_block);
    if config.enabled && routing.streams_to(AudioOutput::Galvo) {
        synth.mixer.push_stream(&synth.stream_block);
    }
}

/// Renders the next block, sends it to the DAC and publishes `galvo.*`
fn render(
    config: Res<GalvoAudioConfig>,
    bus: Res<GalvoAudioBus>,
    time: Res<Time<Real>>,
    mut synth: ResMut<GalvoSynth>,
    mut windows: ResMut<AudioWindows>,
    mut signals: ResMut<Signals>,
) {
    let rate = bus.sample_rate();
    let synth = &mut *synth;
    if synth.mixer.sample_rate() != rate {
        synth.mixer.set_sample_rate(rate);
    }

    let n = if !config.enabled {
        0
    } else if bus.has_consumer() {
        // Keep `latency_ms` of audio queued ahead of the DAC
        let target = (config.latency_ms.max(1.0) * rate as f32 / 1000.0) as usize;
        target.saturating_sub(bus.queued())
    } else {
        // No DAC: run in real time anyway so signals, scopes and tests behave the same
        (time.delta_secs() * rate as f32).round() as usize
    };
    // A long stall must not turn into a burst of seconds of audio
    let n = n.min(rate as usize / 4);

    synth.block.clear();
    synth.mixer.render(n, &signals, &mut synth.block);
    if bus.has_consumer() && !synth.block.is_empty() {
        bus.push(&synth.block);
    }
    windows.galvo.push(&synth.block);

    let levels = Levels::of(&synth.block);
    let decay = release(time.delta_secs(), 0.08);
    let now = [levels.level(), (levels.rms[0] * std::f32::consts::SQRT_2).min(1.0), (levels.rms[1] * std::f32::consts::SQRT_2).min(1.0)];
    for (smoothed, value) in synth.level.iter_mut().zip(now) {
        *smoothed = value.max(*smoothed * decay);
    }
    let hz = synth.mixer.loudest_hz();
    if hz > 0.0 {
        let (lo, hi) = PITCH_SIGNAL_RANGE_HZ;
        synth.pitch = ((hz / lo).ln() / (hi / lo).ln()).clamp(0.0, 1.0);
    }
    let playing = synth.mixer.active_voices() > 0;
    signals.set("galvo.level", synth.level[0]);
    signals.set("galvo.x", synth.level[1]);
    signals.set("galvo.y", synth.level[2]);
    signals.set("galvo.pitch", synth.pitch);
    signals.set("galvo.playing", if playing { 1.0 } else { 0.0 });
}

fn draw_scopes(bus: Res<GalvoAudioBus>, windows: Res<AudioWindows>, mut scopes: Query<(&AudioScope, &mut BasePath)>) {
    let rate = bus.sample_rate() as f32;
    for (scope, mut base) in &mut scopes {
        let window = match scope.input {
            ScopeInput::Galvo => &windows.galvo,
            ScopeInput::Stream => &windows.stream,
        };
        let n = (scope.window_ms.max(1.0) * rate / 1000.0) as usize;
        let frames: Vec<StereoFrame> = window.latest(n).collect();
        base.0 = scope_path(&frames, scope.mode, scope.size_m, scope.max_points, scope.gain, scope.color);
    }
}

/// Per-tick multiplier of a level falling with time constant `secs`
fn release(delta_secs: f32, secs: f32) -> f32 {
    (-delta_secs / secs).exp()
}
