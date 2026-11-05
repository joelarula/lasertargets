//! Built-in sound sources. Plugins add more with `SoundAppExt::register_sound_source`.

use super::catalog::SoundCatalog;
use super::dsp::{Oscillator, Scale, Waveform};
use super::node::{pitch_to_hz, route_into, Route, SoundCtx, SoundSource};
use crate::audio_stream::StereoFrame;
use crate::pipeline::Param;
use serde::Deserialize;

pub fn register(catalog: &mut SoundCatalog) {
    catalog
        .register_source::<Tone>("tone")
        .register_source::<Chirp>("chirp")
        .register_source::<Noise>("noise")
        .register_source::<Silence>("silence");
}

fn one() -> Param {
    Param::Const(1.0)
}

fn default_range() -> [f32; 2] {
    [130.81, 2093.0] // C3..C7
}

/// A steady (or signal-driven) note.
///
/// ```yaml
/// tone: { hz: 1760, ms: 35 }                                        # fixed note
/// tone: { pitch: { signal: gamepad.lx, smooth_ms: 20 }, scale: major_pentatonic }   # instrument
/// ```
/// Without `ms` it plays until the sound is stopped.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tone {
    /// Frequency in Hz
    #[serde(default)]
    hz: Option<Param>,
    /// 0..1 across `range_hz`, even in pitch; use instead of `hz` for signal-driven notes
    #[serde(default)]
    pitch: Option<Param>,
    #[serde(default = "default_range")]
    range_hz: [f32; 2],
    #[serde(default)]
    scale: Scale,
    #[serde(default)]
    ms: Option<f32>,
    #[serde(default)]
    wave: Waveform,
    #[serde(default = "one")]
    gain: Param,
    #[serde(default)]
    route: Option<Route>,
    #[serde(skip)]
    osc: Oscillator,
    #[serde(skip)]
    last_hz: Option<f32>,
}

impl SoundSource for Tone {
    fn duration_secs(&self) -> Option<f32> {
        self.ms.map(|ms| ms / 1000.0)
    }

    fn render(&mut self, ctx: &SoundCtx, out: &mut [StereoFrame]) {
        let target = match (&mut self.hz, &mut self.pitch) {
            (Some(hz), _) => self.scale.quantize(ctx.param(hz)),
            (None, Some(pitch)) => pitch_to_hz(ctx.param(pitch), self.range_hz, self.scale),
            (None, None) => 440.0,
        };
        let gain = ctx.param(&mut self.gain);
        let route = self.route.unwrap_or(ctx.default_route);
        // Glide from the previous block's frequency so signal-driven notes don't step
        let from = self.last_hz.unwrap_or(target);
        let n = out.len().max(1) as f32;
        for (i, frame) in out.iter_mut().enumerate() {
            let hz = from + (target - from) * (i as f32 / n);
            let quadrature = self.osc.quadrature(self.wave) * gain;
            let value = self.osc.next(self.wave, hz, ctx.sample_rate) * gain;
            *frame = [0.0; 2];
            route_into(frame, route, value, quadrature);
        }
        self.last_hz = Some(target);
    }

    fn pitch_hz(&self) -> Option<f32> {
        self.last_hz
    }
}

/// A sweep: `chirp: { from_hz: 2800, to_hz: 1100, ms: 45 }`. Exponential (even in pitch) unless
/// `curve: linear`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chirp {
    from_hz: f32,
    to_hz: f32,
    ms: f32,
    #[serde(default)]
    curve: Curve,
    #[serde(default)]
    wave: Waveform,
    #[serde(default = "one")]
    gain: Param,
    #[serde(default)]
    route: Option<Route>,
    #[serde(skip)]
    osc: Oscillator,
    #[serde(skip)]
    now_hz: f32,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Curve {
    #[default]
    Exponential,
    Linear,
}

impl SoundSource for Chirp {
    fn duration_secs(&self) -> Option<f32> {
        Some(self.ms / 1000.0)
    }

    fn render(&mut self, ctx: &SoundCtx, out: &mut [StereoFrame]) {
        let gain = ctx.param(&mut self.gain);
        let route = self.route.unwrap_or(ctx.default_route);
        let len = (self.ms / 1000.0).max(1e-4);
        for (i, frame) in out.iter_mut().enumerate() {
            let p = ((ctx.t + i as f32 / ctx.sample_rate) / len).clamp(0.0, 1.0);
            let hz = match self.curve {
                Curve::Exponential if self.from_hz > 0.0 && self.to_hz > 0.0 => self.from_hz * (self.to_hz / self.from_hz).powf(p),
                _ => self.from_hz + (self.to_hz - self.from_hz) * p,
            };
            self.now_hz = hz;
            let quadrature = self.osc.quadrature(self.wave) * gain;
            let value = self.osc.next(self.wave, hz, ctx.sample_rate) * gain;
            *frame = [0.0; 2];
            route_into(frame, route, value, quadrature);
        }
    }

    fn pitch_hz(&self) -> Option<f32> {
        Some(self.now_hz)
    }
}

/// White noise, for hits and whooshes (shape it with `lowpass`): `noise: { ms: 80, gain: 0.5 }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Noise {
    #[serde(default)]
    ms: Option<f32>,
    #[serde(default = "one")]
    gain: Param,
    #[serde(default)]
    route: Option<Route>,
    #[serde(skip)]
    state: u32,
}

impl SoundSource for Noise {
    fn duration_secs(&self) -> Option<f32> {
        self.ms.map(|ms| ms / 1000.0)
    }

    fn render(&mut self, ctx: &SoundCtx, out: &mut [StereoFrame]) {
        let gain = ctx.param(&mut self.gain);
        let route = self.route.unwrap_or(ctx.default_route);
        if self.state == 0 {
            self.state = 0x9E37_79B9;
        }
        for frame in out.iter_mut() {
            // xorshift32: cheap, deterministic, good enough for audio noise
            let mut next = || {
                self.state ^= self.state << 13;
                self.state ^= self.state >> 17;
                self.state ^= self.state << 5;
                self.state as f32 / u32::MAX as f32 * 2.0 - 1.0
            };
            let (a, b) = (next(), next());
            *frame = [0.0; 2];
            route_into(frame, route, a * gain, b * gain);
        }
    }
}

/// A rest of `ms`, to hold a slot in a sequence: `silence: { ms: 100 }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Silence {
    ms: f32,
}

impl SoundSource for Silence {
    fn duration_secs(&self) -> Option<f32> {
        Some(self.ms / 1000.0)
    }

    fn render(&mut self, _ctx: &SoundCtx, out: &mut [StereoFrame]) {
        out.fill([0.0; 2]);
    }
}
