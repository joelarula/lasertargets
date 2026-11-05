//! The mix bus every audio output renders through: parallel voices, the live stream, ducking and
//! a limiter. Outputs only differ in sample rate, default route and where the frames go.

use super::catalog::SoundCatalog;
use super::node::Route;
use super::voice::SoundVoice;
use crate::audio_stream::StereoFrame;
use crate::pipeline::Signals;
use std::collections::VecDeque;

/// Voices sounding at once; beyond this the oldest is dropped
pub const MAX_VOICES: usize = 32;

/// Longest live-stream backlog kept, in seconds, before old samples are dropped
const MAX_STREAM_SECS: f32 = 0.5;

struct MixVoice {
    /// Sound name, for `stop` and the per-sound parallel limit; `None` for inline sounds
    name: Option<String>,
    instance: u64,
    voice: SoundVoice,
}

pub struct SoundMixer {
    sample_rate: f32,
    default_route: Route,
    voices: Vec<MixVoice>,
    next_instance: u64,
    stream: VecDeque<StereoFrame>,
    stream_gain: f32,
    stream_duck: f32,
    duck_now: f32,
    limiter: f32,
    loudest_hz: f32,
}

impl SoundMixer {
    pub fn new(sample_rate: u32, default_route: Route) -> Self {
        Self {
            sample_rate: sample_rate.max(1) as f32,
            default_route,
            voices: Vec::new(),
            next_instance: 0,
            stream: VecDeque::new(),
            stream_gain: 1.0,
            stream_duck: 1.0,
            duck_now: 1.0,
            limiter: 1.0,
            loudest_hz: 0.0,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate as u32
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1) as f32;
    }

    pub fn set_default_route(&mut self, route: Route) {
        self.default_route = route;
    }

    pub fn set_stream_gain(&mut self, gain: f32) {
        self.stream_gain = gain.clamp(0.0, 1.0);
    }

    /// Stream level while sounds play, 0..1 (1 = no ducking)
    pub fn set_stream_duck(&mut self, duck: f32) {
        self.stream_duck = duck.clamp(0.0, 1.0);
    }

    /// Starts sound `name` from the catalog on top of whatever is playing. If `max_parallel`
    /// copies of it are sounding, the oldest fades out first.
    pub fn play(&mut self, catalog: &SoundCatalog, name: &str, volume: f32, max_parallel: usize) -> Result<(), String> {
        let voice = catalog.voice(name)?;
        self.start(Some(name), voice.with_volume(volume), max_parallel);
        Ok(())
    }

    /// Starts a ready-made voice (inline sounds built in code)
    pub fn play_voice(&mut self, name: Option<&str>, voice: SoundVoice, max_parallel: usize) {
        self.start(name, voice, max_parallel);
    }

    fn start(&mut self, name: Option<&str>, voice: SoundVoice, max_parallel: usize) {
        if let Some(name) = name {
            let mut copies: Vec<usize> = (0..self.voices.len())
                .filter(|&i| self.voices[i].name.as_deref() == Some(name) && !self.voices[i].voice.is_stopping())
                .collect();
            copies.sort_by_key(|&i| self.voices[i].instance);
            let excess = (copies.len() + 1).saturating_sub(max_parallel.max(1));
            for &i in copies.iter().take(excess) {
                self.voices[i].voice.stop();
            }
        }
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.next_instance += 1;
        self.voices.push(MixVoice { name: name.map(str::to_string), instance: self.next_instance, voice });
    }

    /// Fades out every copy of `name` (instruments and drones end this way)
    pub fn stop(&mut self, name: &str) {
        for v in self.voices.iter_mut().filter(|v| v.name.as_deref() == Some(name)) {
            v.voice.stop();
        }
    }

    pub fn stop_all(&mut self) {
        self.voices.iter_mut().for_each(|v| v.voice.stop());
        self.stream.clear();
    }

    /// Copies of `name` sounding now (not counting ones fading out)
    pub fn copies_of(&self, name: &str) -> usize {
        self.voices.iter().filter(|v| v.name.as_deref() == Some(name) && !v.voice.is_stopping()).count()
    }

    pub fn active_voices(&self) -> usize {
        self.voices.len()
    }

    /// Queues live audio, already at this mixer's sample rate
    pub fn push_stream(&mut self, frames: &[StereoFrame]) {
        self.stream.extend(frames.iter().copied());
        let max = (MAX_STREAM_SECS * self.sample_rate) as usize;
        let excess = self.stream.len().saturating_sub(max);
        self.stream.drain(..excess);
    }

    /// Pitch of the loudest voice in the last block, 0 when silent
    pub fn loudest_hz(&self) -> f32 {
        self.loudest_hz
    }

    /// Appends `n` frames: all voices in parallel, plus the stream (ducked while voices sound),
    /// through the limiter
    pub fn render(&mut self, n: usize, signals: &Signals, out: &mut Vec<StereoFrame>) {
        if n == 0 {
            return;
        }
        let start = out.len();
        out.resize(start + n, [0.0; 2]);
        let block = &mut out[start..];

        let mut loudest = (0.0_f32, 0.0_f32);
        for v in &mut self.voices {
            v.voice.render(self.sample_rate, self.default_route, signals, block);
            if let Some(hz) = v.voice.pitch_hz() {
                if v.voice.level() > loudest.0 {
                    loudest = (v.voice.level(), hz);
                }
            }
        }
        let effects = !self.voices.is_empty();
        self.voices.retain(|v| !v.voice.is_finished());
        self.loudest_hz = if loudest.0 > 1e-3 { loudest.1 } else { 0.0 };

        let k = |ms: f32| 1.0 - (-1000.0 / (ms * self.sample_rate)).exp();
        let (duck_k, release_k) = (k(20.0), k(50.0));
        let duck_target = if effects { self.stream_duck } else { 1.0 };
        for frame in block.iter_mut() {
            self.duck_now += (duck_target - self.duck_now) * duck_k;
            if let Some([l, r]) = self.stream.pop_front() {
                let g = self.stream_gain * self.duck_now;
                frame[0] += l * g;
                frame[1] += r * g;
            }
            // Limiter: never past full scale, without the distortion of clipping
            let peak = frame[0].abs().max(frame[1].abs());
            if peak * self.limiter > 1.0 {
                self.limiter = 1.0 / peak;
            } else {
                self.limiter += (1.0 - self.limiter) * release_k;
            }
            frame[0] = (frame[0] * self.limiter).clamp(-1.0, 1.0);
            frame[1] = (frame[1] * self.limiter).clamp(-1.0, 1.0);
        }
    }
}
