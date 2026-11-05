//! The speakers output. Plays what `audio.routing` sends to `speakers`:
//!
//! - a sound with a recorded file in the [`SoundBank`] plays that file;
//! - every other sound is rendered from its composition (`assets/sounds/<name>.sound.yaml`)
//!   through a [`SoundMixer`] at 48 kHz, the same files the galvos play;
//! - the live stream, when `audio.routing.stream` includes `speakers`, goes through the same
//!   mixer (ducked under sound effects).
//!
//! Sounds overlap freely, capped per sound by `max_parallel`.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use bevy::audio::{AddAudioSource, Decodable, Volume};
use bevy::prelude::*;
use common::audio::{ensure_audio_routing, AudioConfiguration, AudioOutput, AudioRouting, AudioRoutingSet, PlaySoundEvent, RoutedSound, SoundId};
use common::audio_stream::{LiveAudioFrame, Resampler, StereoFrame};
use common::pipeline::Signals;
use common::settings::ConfigAppExt;
use common::sound::{Route, SoundCatalog, SoundMixer};
use common::vision::VisionDetection;

/// Speaker mix sample rate
const RATE: u32 = 48_000;
/// Rendered ahead of the sound card; lower is snappier, higher survives slow ticks
const LATENCY_SECS: f32 = 0.06;

/// Recorded files by sound name. Optional: a sound without a file plays its composition.
#[derive(Resource, Default)]
pub struct SoundBank {
    pub handles: HashMap<String, Handle<AudioSource>>,
}

/// A recorded file playing on the speakers; `seq` orders copies of the same sound by age
#[derive(Component)]
pub struct SpeakerSound {
    pub name: String,
    seq: u64,
}

pub struct LaserAudioPlugin;

impl Plugin for LaserAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<AudioConfiguration>();
        app.add_plugins(bevy::audio::AudioPlugin::default());
        app.add_audio_source::<SpeakerMixAudio>();
        ensure_audio_routing(app);
        app.init_resource::<SoundBank>();
        app.add_systems(Startup, (setup_sound_bank, setup_speaker_mix));
        app.add_systems(
            Update,
            (
                handle_vision_hit_audio.before(AudioRoutingSet),
                (play_routed_sounds, feed_stream, render_speaker_mix, manage_mix_player).chain().after(AudioRoutingSet),
            ),
        );
    }
}

/// Loads recorded files that override compositions. Lists only files that exist, so a fresh
/// checkout plays the compositions without missing-file errors.
fn setup_sound_bank(asset_server: Res<AssetServer>, mut sound_bank: ResMut<SoundBank>, runtime: Option<Res<crate::RuntimeSettings>>) {
    let assets_dir = std::path::PathBuf::from(runtime.map(|r| r.resolved_assets_dir()).unwrap_or_else(|| "assets".into()));
    for id in SoundId::ALL {
        for ext in ["wav", "ogg", "flac"] {
            let path = format!("sounds/{}.{ext}", id.name());
            if assets_dir.join(&path).is_file() {
                sound_bank.handles.insert(id.name().to_string(), asset_server.load(path));
                break;
            }
        }
    }
    info!("SoundBank: {} recorded files; other sounds play their compositions", sound_bank.handles.len());
}

/// Plays the sounds routed to the speakers, in parallel. Past a sound's `max_parallel` the
/// oldest copy stops.
fn play_routed_sounds(
    mut commands: Commands,
    mut sounds: MessageReader<RoutedSound>,
    config: Res<AudioConfiguration>,
    sound_bank: Res<SoundBank>,
    catalog: Res<SoundCatalog>,
    mix: Option<ResMut<SpeakerMix>>,
    playing: Query<(Entity, &SpeakerSound)>,
    mut next_seq: Local<u64>,
) {
    let (Some(mut mix), true) = (mix, config.enabled) else {
        sounds.clear();
        return;
    };

    // Copies playing now, including ones started earlier in this loop
    let mut live: Vec<(Entity, String, u64)> = playing.iter().map(|(e, s)| (e, s.name.clone(), s.seq)).collect();
    for event in sounds.read().filter(|s| s.output == AudioOutput::Speakers) {
        if event.stop {
            mix.mixer.stop(&event.name);
            live.retain(|(entity, name, _)| {
                let keep = *name != event.name;
                if !keep {
                    commands.entity(*entity).despawn();
                }
                keep
            });
            continue;
        }
        if let Some(spec) = &event.inline {
            match catalog.voice_of(spec) {
                Ok(voice) => mix.mixer.play_voice(Some(&event.name), voice.with_volume(event.volume), event.max_parallel),
                Err(e) => warn!("inline sound `{}`: {e}", event.name),
            }
            continue;
        }
        let Some(handle) = sound_bank.handles.get(&event.name) else {
            if let Err(e) = mix.mixer.play(&catalog, &event.name, event.volume, event.max_parallel) {
                debug!("speakers: {e}");
            }
            continue;
        };
        loop {
            let same: Vec<usize> = (0..live.len()).filter(|&i| live[i].1 == event.name).collect();
            if same.len() < event.max_parallel.max(1) {
                break;
            }
            let oldest = *same.iter().min_by_key(|&&i| live[i].2).unwrap();
            commands.entity(live[oldest].0).despawn();
            live.swap_remove(oldest);
        }
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = Volume::Linear((config.volume * event.volume).clamp(0.0, 1.0));
        *next_seq += 1;
        let entity = commands
            .spawn((AudioPlayer::new(handle.clone()), settings, SpeakerSound { name: event.name.clone(), seq: *next_seq }))
            .id();
        live.push((entity, event.name.clone(), *next_seq));
    }
}

/// Automatically triggers a hit sound cue when the camera detects a laser spot appear on target
fn handle_vision_hit_audio(
    mut detections: MessageReader<VisionDetection>,
    mut sound_events: MessageWriter<PlaySoundEvent>,
    config: Res<AudioConfiguration>,
) {
    if !config.enabled {
        return;
    }

    for detection in detections.read() {
        if detection.appeared && (detection.label == "laser" || detection.label == "spot") {
            let mut event = PlaySoundEvent::new(SoundId::LaserHit);
            if config.spatial_panning {
                event = event.with_pan(detection.galvo_pos.x);
            }
            sound_events.write(event);
        }
    }
}

// ── Speaker mix: compositions and the live stream ───────────────────────────────────────────

/// Frames waiting for the sound card, shared with the audio thread
#[derive(Clone, Default)]
struct MixQueue(Arc<Mutex<VecDeque<StereoFrame>>>);

/// An endless audio source playing the speaker mix (silence when nothing sounds).
#[derive(Asset, TypePath, Clone)]
pub struct SpeakerMixAudio {
    queue: MixQueue,
}

impl Decodable for SpeakerMixAudio {
    type DecoderItem = f32;
    type Decoder = SpeakerMixDecoder;

    fn decoder(&self) -> Self::Decoder {
        SpeakerMixDecoder { queue: self.queue.clone(), local: VecDeque::new(), frame: [0.0; 2], channel: 0 }
    }
}

/// Runs on the audio thread. Takes frames in chunks with `try_lock`, so it never waits on the
/// game thread; on underrun the last frame fades out (~20 ms) instead of clicking.
pub struct SpeakerMixDecoder {
    queue: MixQueue,
    local: VecDeque<StereoFrame>,
    frame: StereoFrame,
    channel: usize,
}

impl Iterator for SpeakerMixDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.channel == 0 {
            if self.local.is_empty() {
                if let Ok(mut queue) = self.queue.0.try_lock() {
                    let n = queue.len().min(1024);
                    self.local.extend(queue.drain(..n));
                }
            }
            self.frame = self.local.pop_front().unwrap_or([self.frame[0] * 0.999, self.frame[1] * 0.999]);
        }
        let sample = self.frame[self.channel];
        self.channel ^= 1;
        // Never ends: an ended source would stop the player
        Some(sample)
    }
}

impl bevy::audio::Source for SpeakerMixDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Resource)]
pub struct SpeakerMix {
    pub mixer: SoundMixer,
    queue: MixQueue,
    handle: Handle<SpeakerMixAudio>,
    resampler: Resampler,
    scratch: Vec<StereoFrame>,
}

/// Marks the entity playing the speaker mix
#[derive(Component)]
struct SpeakerMixPlayer;

fn setup_speaker_mix(mut commands: Commands, mut assets: ResMut<Assets<SpeakerMixAudio>>) {
    let queue = MixQueue::default();
    let handle = assets.add(SpeakerMixAudio { queue: queue.clone() });
    commands.insert_resource(SpeakerMix {
        mixer: SoundMixer::new(RATE, Route::Both),
        queue,
        handle,
        resampler: Resampler::default(),
        scratch: Vec::new(),
    });
}

/// Live frames → the speaker mix, when the stream is routed to the speakers
fn feed_stream(routing: Res<AudioRouting>, config: Res<AudioConfiguration>, mut frames: MessageReader<LiveAudioFrame>, mix: Option<ResMut<SpeakerMix>>) {
    let Some(mut mix) = mix else {
        frames.clear();
        return;
    };
    if !config.enabled || !routing.streams_to(AudioOutput::Speakers) {
        frames.clear();
        return;
    }
    let mix = &mut *mix;
    mix.scratch.clear();
    for frame in frames.read() {
        mix.resampler.process(&frame.samples, frame.channels, frame.sample_rate, RATE, &mut mix.scratch);
    }
    mix.mixer.set_stream_gain(config.stream_volume);
    mix.mixer.push_stream(&mix.scratch);
}

/// Keeps `LATENCY_SECS` of mixed audio queued ahead of the sound card
fn render_speaker_mix(signals: Res<Signals>, mix: Option<ResMut<SpeakerMix>>) {
    let Some(mut mix) = mix else { return };
    let mix = &mut *mix;
    let queued = mix.queue.0.lock().map(|q| q.len()).unwrap_or(0);
    let n = ((LATENCY_SECS * RATE as f32) as usize).saturating_sub(queued);
    if n == 0 {
        return;
    }
    mix.scratch.clear();
    mix.mixer.render(n, &signals, &mut mix.scratch);
    if let Ok(mut queue) = mix.queue.0.lock() {
        queue.extend(mix.scratch.iter().copied());
    }
}

/// Starts the mix player, and restarts it when the master volume changes. It plays next to
/// recorded files, so everything overlaps.
fn manage_mix_player(mut commands: Commands, config: Res<AudioConfiguration>, mix: Option<Res<SpeakerMix>>, players: Query<Entity, With<SpeakerMixPlayer>>) {
    let Some(mix) = mix else { return };
    if !(config.is_changed() || mix.is_added()) {
        return;
    }
    for player in &players {
        commands.entity(player).despawn();
    }
    if config.enabled {
        let mut settings = PlaybackSettings::ONCE;
        settings.volume = Volume::Linear(config.volume.clamp(0.0, 1.0));
        commands.spawn((AudioPlayer(mix.handle.clone()), settings, SpeakerMixPlayer));
    }
}
