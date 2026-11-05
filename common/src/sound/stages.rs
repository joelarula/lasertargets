//! Built-in sound stages. Plugins add more with `SoundAppExt::register_sound_stage`.

use super::catalog::SoundCatalog;
use super::dsp::{Biquad, Envelope as Adsr};
use super::node::{SoundCtx, SoundStage};
use crate::audio_stream::StereoFrame;
use crate::pipeline::Param;
use serde::Deserialize;
use std::f32::consts::{FRAC_PI_4, TAU};

pub fn register(catalog: &mut SoundCatalog) {
    catalog
        .register_stage::<Envelope>("envelope")
        .register_stage::<Gain>("gain")
        .register_stage::<Pan>("pan")
        .register_stage::<Tremolo>("tremolo")
        .register_stage::<Lowpass>("lowpass")
        .register_stage::<Echo>("echo");
}

fn one() -> Param {
    Param::Const(1.0)
}

/// Attack/decay/sustain/release over the layer's length, so tones start and end without a
/// click: `envelope: { attack_ms: 2, decay_ms: 5, sustain: 0.9, release_ms: 5 }`.
/// On a layer that plays until stopped, only attack and decay apply.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct Envelope(Adsr);

impl SoundStage for Envelope {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let total_ms = ctx.duration.map(|d| d * 1000.0).unwrap_or(f32::MAX);
        for (i, frame) in buf.iter_mut().enumerate() {
            let t_ms = (ctx.t + i as f32 / ctx.sample_rate) * 1000.0;
            let g = self.0.gain(t_ms, total_ms);
            frame[0] *= g;
            frame[1] *= g;
        }
    }
}

/// Level, 0..1 or more: `gain: { amount: { signal: game.combo, range: [0.5, 1.0] } }`.
/// Ramps across each block, so signal-driven levels don't zipper.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gain {
    #[serde(default = "one")]
    amount: Param,
    #[serde(skip)]
    last: Option<f32>,
}

impl SoundStage for Gain {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let target = ctx.param(&mut self.amount);
        ramp(buf, self.last.unwrap_or(target), target, |f, g| {
            f[0] *= g;
            f[1] *= g;
        });
        self.last = Some(target);
    }
}

/// Stereo position, -1 (left / X galvo) .. 1 (right / Y galvo), constant power:
/// `pan: { position: { signal: hunter.reticle_x, range: [-1, 1] } }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pan {
    #[serde(default)]
    position: Param,
    #[serde(skip)]
    last: Option<f32>,
}

impl SoundStage for Pan {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let target = ctx.param(&mut self.position).clamp(-1.0, 1.0);
        ramp(buf, self.last.unwrap_or(target), target, |f, p| {
            let angle = (p + 1.0) * FRAC_PI_4;
            let mono = 0.5 * (f[0] + f[1]);
            *f = [mono * angle.cos() * std::f32::consts::SQRT_2, mono * angle.sin() * std::f32::consts::SQRT_2];
        });
        self.last = Some(target);
    }
}

/// Periodic level wobble: `tremolo: { rate_hz: 7, depth: 0.5 }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tremolo {
    rate_hz: Param,
    #[serde(default = "half")]
    depth: Param,
}

fn half() -> Param {
    Param::Const(0.5)
}

impl SoundStage for Tremolo {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let rate = ctx.param(&mut self.rate_hz);
        let depth = ctx.param(&mut self.depth).clamp(0.0, 1.0);
        for (i, frame) in buf.iter_mut().enumerate() {
            let t = ctx.t + i as f32 / ctx.sample_rate;
            let g = 1.0 - depth * 0.5 * (1.0 - (TAU * rate * t).cos());
            frame[0] *= g;
            frame[1] *= g;
        }
    }
}

/// Low-pass filter: `lowpass: { cutoff_hz: { signal: self.dark, range: [8000, 400] } }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lowpass {
    cutoff_hz: Param,
    #[serde(skip)]
    filters: Option<([Biquad; 2], f32)>,
}

impl SoundStage for Lowpass {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let cutoff = ctx.param(&mut self.cutoff_hz);
        let rebuild = match &self.filters {
            Some((_, current)) => (current - cutoff).abs() > 1.0,
            None => true,
        };
        if rebuild {
            // Rebuilding resets the filter state; keep the old history for continuity
            let mut next = [Biquad::lowpass(cutoff, ctx.sample_rate); 2];
            if let Some((old, _)) = &self.filters {
                for axis in 0..2 {
                    next[axis].carry_state(&old[axis]);
                }
            }
            self.filters = Some((next, cutoff));
        }
        let (filters, _) = self.filters.as_mut().unwrap();
        for frame in buf.iter_mut() {
            frame[0] = filters[0].process(frame[0]);
            frame[1] = filters[1].process(frame[1]);
        }
    }
}

/// Repeats: `echo: { ms: 80, feedback: 0.4, mix: 0.5 }`. Keeps the voice alive until the
/// repeats have died away.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Echo {
    ms: f32,
    #[serde(default = "default_feedback")]
    feedback: f32,
    #[serde(default = "half_f32")]
    mix: f32,
    #[serde(skip)]
    line: Vec<StereoFrame>,
    #[serde(skip)]
    pos: usize,
}

fn default_feedback() -> f32 {
    0.35
}

fn half_f32() -> f32 {
    0.5
}

impl SoundStage for Echo {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]) {
        let len = ((self.ms / 1000.0) * ctx.sample_rate).max(1.0) as usize;
        if self.line.len() != len {
            self.line = vec![[0.0; 2]; len];
            self.pos = 0;
        }
        let feedback = self.feedback.clamp(0.0, 0.95);
        for frame in buf.iter_mut() {
            let delayed = self.line[self.pos];
            self.line[self.pos] = [frame[0] + delayed[0] * feedback, frame[1] + delayed[1] * feedback];
            self.pos = (self.pos + 1) % len;
            frame[0] += delayed[0] * self.mix;
            frame[1] += delayed[1] * self.mix;
        }
    }

    fn tail_secs(&self) -> f32 {
        // Repeats until they fall below -60 dB
        let feedback = self.feedback.clamp(0.01, 0.95);
        let repeats = (1e-3_f32.ln() / feedback.ln()).ceil().max(1.0);
        repeats * self.ms / 1000.0
    }
}

/// Applies `f(frame, value)` with `value` ramping linearly from `from` to `to` across `buf`
fn ramp(buf: &mut [StereoFrame], from: f32, to: f32, mut f: impl FnMut(&mut StereoFrame, f32)) {
    let n = buf.len().max(1) as f32;
    for (i, frame) in buf.iter_mut().enumerate() {
        f(frame, from + (to - from) * (i as f32 / n));
    }
}
