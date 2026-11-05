//! Live audio input (receiver mode, `NetworkMessage::AudioStreamFrame`), shared by every audio
//! output. Each output plays it when `audio.routing.stream` lists that output; scopes and
//! `audio.*` signals use it either way.

use bevy::prelude::*;

/// Stereo frame `[left, right]`, -1..1
pub type StereoFrame = [f32; 2];

/// One chunk of live PCM audio. The server maps `NetworkMessage::AudioStreamFrame` onto it.
#[derive(Message, Debug, Clone)]
pub struct LiveAudioFrame {
    /// -1..1, interleaved when stereo
    pub samples: Vec<f32>,
    pub channels: u8,
    pub sample_rate: u32,
}

/// Linear resampler from any input rate to an output's rate. Keeps its position across calls,
/// so consecutive network frames join without a seam.
#[derive(Debug, Clone, Default)]
pub struct Resampler {
    /// Position in input frames, relative to `prev`
    pos: f64,
    prev: Option<StereoFrame>,
}

impl Resampler {
    /// Interleaved samples with `channels` channels at `in_rate` → stereo frames at `out_rate`,
    /// appended to `out`. Mono is copied to both channels; channels beyond the second are ignored.
    pub fn process(&mut self, samples: &[f32], channels: u8, in_rate: u32, out_rate: u32, out: &mut Vec<StereoFrame>) {
        let channels = channels.max(1) as usize;
        if in_rate == 0 || out_rate == 0 || samples.len() < channels {
            return;
        }
        let frames = samples.chunks_exact(channels).map(|c| if channels == 1 { [c[0], c[0]] } else { [c[0], c[1]] });
        let input: Vec<StereoFrame> = self.prev.into_iter().chain(frames).collect();
        let step = in_rate as f64 / out_rate as f64;
        while self.pos + 1.0 < input.len() as f64 {
            let i = self.pos.floor() as usize;
            let f = (self.pos - i as f64) as f32;
            let (a, b) = (input[i], input[i + 1]);
            out.push([a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]);
            self.pos += step;
        }
        self.pos -= (input.len() - 1) as f64;
        self.prev = input.last().copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_rate_and_joins_chunks() {
        let mut resampler = Resampler::default();
        let mut out = Vec::new();
        let mut t = 0.0_f32;
        for _ in 0..10 {
            let chunk: Vec<f32> = (0..480)
                .flat_map(|_| {
                    t += 1.0 / 4800.0;
                    [t, -t]
                })
                .collect();
            resampler.process(&chunk, 2, 48_000, 30_000, &mut out);
        }
        assert!((out.len() as i32 - 3000).abs() <= 2, "{} frames", out.len());
        assert!(out.windows(2).all(|w| w[1][0] >= w[0][0] && w[1][1] <= w[0][1]), "monotonic across chunks");

        let mut mono = Vec::new();
        Resampler::default().process(&[0.5; 100], 1, 30_000, 30_000, &mut mono);
        assert!(mono.iter().all(|f| f == &[0.5, 0.5]), "mono → both channels");
    }
}
