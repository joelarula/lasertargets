//! Audio as geometry: oscilloscope figures in metres in the entity's local frame. The plugin
//! writes them into an entity's `BasePath`, and its pipeline file styles and animates them like
//! any other shape (colour, fade, reactions), so the scope goes through the scene transform,
//! culling and the frame safety rules like everything else.

use common::audio_stream::StereoFrame as Frame;
use common::path::{PathPoint, PathSegment, UniversalPath};
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScopeMode {
    /// Vector scope: left/X → x, right/Y → y. Draws stereo Lissajous figures and the exact
    /// micro-figure the galvos are singing
    #[default]
    Xy,
    /// Ring whose radius follows the waveform: r(θ) = R·(0.7 + 0.3·s(θ))
    Ring,
    /// Waveform ribbon drawn left to right
    Line,
}

/// Which signal a scope shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScopeInput {
    /// What the galvos are being driven with (cues, instruments, sung stream)
    #[default]
    Galvo,
    /// The live incoming audio stream
    Stream,
}

/// Builds the scope figure from `frames` (oldest first), `size_m` across, at most `points`
/// points. Samples are scaled by `gain` and clamped to -1..1.
pub fn scope_path(frames: &[Frame], mode: ScopeMode, size_m: f32, points: usize, gain: f32, color: [u8; 3]) -> UniversalPath {
    let points = points.max(8);
    if frames.is_empty() {
        return UniversalPath::empty();
    }
    let step = (frames.len() / points).max(1);
    let picked: Vec<Frame> = frames.iter().step_by(step).take(points).copied().collect();
    let half = size_m * 0.5;
    let s = |v: f32| (v * gain).clamp(-1.0, 1.0);
    let [r, g, b] = color;
    let point = |x: f32, y: f32| PathPoint::new(x, y, r, g, b, 0);
    let count = picked.len();

    let mut out: Vec<PathPoint> = match mode {
        ScopeMode::Xy => picked.iter().map(|f| point(s(f[0]) * half, s(f[1]) * half)).collect(),
        ScopeMode::Ring => picked
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let theta = i as f32 / count as f32 * TAU;
                let radius = half * (0.7 + 0.3 * s(0.5 * (f[0] + f[1])));
                point(radius * theta.cos(), radius * theta.sin())
            })
            .collect(),
        ScopeMode::Line => picked
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let t = i as f32 / (count.max(2) - 1) as f32;
                point(-half + t * size_m, s(0.5 * (f[0] + f[1])) * half * 0.5)
            })
            .collect(),
    };
    if mode == ScopeMode::Ring {
        if let Some(first) = out.first().cloned() {
            out.push(first);
        }
    }
    UniversalPath::from_segment(PathSegment::new(out))
}
