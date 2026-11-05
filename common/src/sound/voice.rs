//! One playing instance of a composed sound.
//!
//! Time is kept in whole samples, so layers start on exactly the same sample whatever block
//! sizes the output renders in.

use super::node::{Route, SoundCtx, SoundSource, SoundStage};
use crate::audio_stream::StereoFrame;
use crate::pipeline::{Param, Signals};

/// A stopped voice fades out over this long instead of cutting (a cut clicks)
pub const STOP_FADE_SECS: f32 = 0.03;

pub(crate) struct LayerVoice {
    /// Seconds into the sound
    start: f32,
    /// Source length; `None` plays until stopped
    duration: Option<f32>,
    /// When the layer falls silent, stage tails included
    end: Option<f32>,
    source: Box<dyn SoundSource>,
    stages: Vec<Box<dyn SoundStage>>,
}

impl LayerVoice {
    pub(crate) fn new(start: f32, source: Box<dyn SoundSource>, stages: Vec<Box<dyn SoundStage>>) -> Self {
        let duration = source.duration_secs();
        let tail = stages.iter().map(|s| s.tail_secs()).fold(0.0, f32::max);
        let end = duration.map(|d| start + d + tail);
        Self { start, duration, end, source, stages }
    }
}

pub struct SoundVoice {
    layers: Vec<LayerVoice>,
    stages: Vec<Box<dyn SoundStage>>,
    gain: Param,
    last_gain: Option<f32>,
    /// Extra level from the request (event volume, trigger strength)
    volume: f32,
    /// Samples rendered so far
    pos: u64,
    /// Rate of the last render
    rate: f32,
    /// End of the whole sound in seconds; `None` while any layer plays until stopped
    end: Option<f32>,
    /// Sample position when `stop` was called
    stopped_at: Option<u64>,
    layer_buf: Vec<StereoFrame>,
    mix_buf: Vec<StereoFrame>,
    level: f32,
}

fn samples(secs: f32, rate: f32) -> u64 {
    (secs.max(0.0) * rate).round() as u64
}

impl SoundVoice {
    pub(crate) fn new(layers: Vec<LayerVoice>, stages: Vec<Box<dyn SoundStage>>, gain: Param) -> Self {
        let tail = stages.iter().map(|s| s.tail_secs()).fold(0.0, f32::max);
        let end = layers.iter().try_fold(0.0_f32, |acc, l| l.end.map(|e| acc.max(e))).map(|e| e + tail);
        Self {
            layers,
            stages,
            gain,
            last_gain: None,
            volume: 1.0,
            pos: 0,
            rate: 1.0,
            end,
            stopped_at: None,
            layer_buf: Vec::new(),
            mix_buf: Vec::new(),
            level: 0.0,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.max(0.0);
        self
    }

    /// Length in seconds, `None` if it plays until stopped
    pub fn duration(&self) -> Option<f32> {
        self.end
    }

    /// Seconds rendered so far
    pub fn elapsed(&self) -> f32 {
        self.pos as f32 / self.rate
    }

    /// Fades out over [`STOP_FADE_SECS`] and finishes
    pub fn stop(&mut self) {
        if self.stopped_at.is_none() {
            self.stopped_at = Some(self.pos);
        }
    }

    pub fn is_stopping(&self) -> bool {
        self.stopped_at.is_some()
    }

    pub fn is_finished(&self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.stopped_at.is_some_and(|s| self.pos >= s + samples(STOP_FADE_SECS, self.rate))
            || self.end.is_some_and(|e| self.pos >= samples(e, self.rate))
    }

    /// Loudness of the last block: RMS over both channels, so a full-scale tone on both
    /// channels (or an orbit) reads 1
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Pitch of the first sounding layer that has one
    pub fn pitch_hz(&self) -> Option<f32> {
        let t = self.elapsed();
        self.layers
            .iter()
            .filter(|l| l.start <= t && l.duration.is_none_or(|d| t < l.start + d))
            .find_map(|l| l.source.pitch_hz())
    }

    /// Renders the next `out.len()` frames and adds them to `out`
    pub fn render(&mut self, sample_rate: f32, default_route: Route, signals: &Signals, out: &mut [StereoFrame]) {
        let n = out.len();
        if n == 0 {
            return;
        }
        self.rate = sample_rate;
        let (pos, block_end) = (self.pos, self.pos + n as u64);
        let block_secs = n as f32 / sample_rate;
        self.mix_buf.clear();
        self.mix_buf.resize(n, [0.0; 2]);

        for layer in &mut self.layers {
            let start = samples(layer.start, sample_rate);
            let end = layer.end.map(|e| samples(e, sample_rate));
            if start >= block_end || end.is_some_and(|e| e <= pos) {
                continue;
            }
            // Frames of this block the layer covers
            let first = start.max(pos);
            let last = end.map_or(block_end, |e| e.min(block_end));
            let (offset, count) = ((first - pos) as usize, (last - first) as usize);
            let into_layer = first - start;
            self.layer_buf.clear();
            self.layer_buf.resize(count, [0.0; 2]);
            let ctx = SoundCtx {
                sample_rate,
                t: into_layer as f32 / sample_rate,
                block_secs,
                duration: layer.duration,
                default_route,
                signals,
            };
            // After the source ends only stage tails (echo) still sound
            let sounding = layer.duration.map_or(count as u64, |d| samples(d, sample_rate).saturating_sub(into_layer).min(count as u64)) as usize;
            if sounding > 0 {
                layer.source.render(&ctx, &mut self.layer_buf[..sounding]);
            }
            for stage in &mut layer.stages {
                stage.process(&ctx, &mut self.layer_buf);
            }
            for (mix, frame) in self.mix_buf[offset..offset + count].iter_mut().zip(&self.layer_buf) {
                mix[0] += frame[0];
                mix[1] += frame[1];
            }
        }

        let ctx = SoundCtx { sample_rate, t: pos as f32 / sample_rate, block_secs, duration: self.end, default_route, signals };
        for stage in &mut self.stages {
            stage.process(&ctx, &mut self.mix_buf);
        }

        let gain = ctx.param(&mut self.gain);
        let from = self.last_gain.unwrap_or(gain);
        self.last_gain = Some(gain);
        let fade_len = samples(STOP_FADE_SECS, sample_rate).max(1) as f32;
        let mut sum = 0.0;
        for (i, (o, m)) in out.iter_mut().zip(&self.mix_buf).enumerate() {
            let fade = match self.stopped_at {
                Some(s) => (1.0 - (pos + i as u64).saturating_sub(s) as f32 / fade_len).clamp(0.0, 1.0),
                None => 1.0,
            };
            let g = (from + (gain - from) * (i as f32 / n as f32)) * self.volume * fade;
            let (l, r) = (m[0] * g, m[1] * g);
            sum += l * l + r * r;
            o[0] += l;
            o[1] += r;
        }
        self.level = (sum / n as f32).sqrt().min(1.0);
        self.pos = block_end;
    }
}
