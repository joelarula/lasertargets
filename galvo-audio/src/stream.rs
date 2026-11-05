//! Rolling windows for scopes and level analysis. The live stream itself and its resampler are
//! shared by all outputs: `common::audio_stream`.

pub use common::audio_stream::Resampler;

use common::audio_stream::StereoFrame as Frame;
use std::collections::VecDeque;

/// The most recent frames of a signal, for scopes and analysis.
#[derive(Debug, Clone)]
pub struct AudioWindow {
    frames: VecDeque<Frame>,
    capacity: usize,
}

impl AudioWindow {
    pub fn new(capacity: usize) -> Self {
        Self { frames: VecDeque::with_capacity(capacity), capacity: capacity.max(1) }
    }

    pub fn push(&mut self, frames: &[Frame]) {
        let skip = frames.len().saturating_sub(self.capacity);
        self.frames.extend(frames[skip..].iter().copied());
        while self.frames.len() > self.capacity {
            self.frames.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The newest `n` frames, oldest first
    pub fn latest(&self, n: usize) -> impl Iterator<Item = Frame> + '_ {
        self.frames.iter().skip(self.frames.len().saturating_sub(n)).copied()
    }
}

/// RMS and peak of a block, per axis and combined.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Levels {
    pub rms: [f32; 2],
    pub peak: f32,
}

impl Levels {
    pub fn of(frames: &[Frame]) -> Self {
        if frames.is_empty() {
            return Self::default();
        }
        let mut sum = [0.0_f32; 2];
        let mut peak = 0.0_f32;
        for f in frames {
            sum[0] += f[0] * f[0];
            sum[1] += f[1] * f[1];
            peak = peak.max(f[0].abs()).max(f[1].abs());
        }
        let n = frames.len() as f32;
        Self { rms: [(sum[0] / n).sqrt(), (sum[1] / n).sqrt()], peak }
    }

    /// Combined level 0..1; a full-scale sine reads 1
    pub fn level(&self) -> f32 {
        (self.rms[0].max(self.rms[1]) * std::f32::consts::SQRT_2).clamp(0.0, 1.0)
    }

    pub fn is_silent(&self) -> bool {
        self.peak < 1e-3
    }
}
