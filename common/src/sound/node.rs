//! The plugin contract: sources make sound, stages shape it. Both are pure (no ECS access),
//! stateful per playing voice, and built from their YAML entry by a registered factory.

use super::dsp::Scale;
use crate::audio_stream::StereoFrame;
use crate::pipeline::{EntitySignals, Param, SignalView, Signals, StageCtx};
use serde::{Deserialize, Serialize};

/// Where a mono source puts its signal in the stereo frame. On the galvos left = X mirror and
/// right = Y mirror; on the speakers they are the left and right channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    #[serde(alias = "x")]
    Left,
    #[serde(alias = "y")]
    Right,
    /// Both channels in phase (centre / a diagonal micro-line on the galvos)
    Both,
    /// Left = sine, right = cosine of the same oscillator: a micro-circle on the galvos, which
    /// shares the heat between both coils; a wide stereo image on speakers
    Orbit,
}

/// What a source or stage may read while rendering one block.
pub struct SoundCtx<'a> {
    pub sample_rate: f32,
    /// Seconds since this source (or, for mix stages, the voice) started, at the block's first frame
    pub t: f32,
    /// Length of the block in seconds
    pub block_secs: f32,
    /// Length of the layer this node belongs to, `None` while it plays until stopped
    pub duration: Option<f32>,
    /// Route for sources that don't name one; set by the output (galvo: orbit, speakers: both)
    pub default_route: Route,
    pub signals: &'a Signals,
}

impl SoundCtx<'_> {
    /// Current value of a parameter (constant or signal-bound). Call once per block per param.
    pub fn param(&self, param: &mut Param) -> f32 {
        let none = EntitySignals::default();
        let ctx = StageCtx { delta_secs: self.block_secs, elapsed_secs: self.t, signals: SignalView::new(&none, self.signals) };
        param.get(&ctx)
    }
}

/// Generates sound. Writes (overwrites) every frame of `out`, which starts at `ctx.t`.
pub trait SoundSource: Send + Sync + 'static {
    /// Seconds until the source ends; `None` plays until the voice is stopped
    fn duration_secs(&self) -> Option<f32>;
    fn render(&mut self, ctx: &SoundCtx, out: &mut [StereoFrame]);
    /// Frequency it is sounding right now, if it has one (published as `galvo.pitch` …)
    fn pitch_hz(&self) -> Option<f32> {
        None
    }
}

/// Shapes sound in place.
pub trait SoundStage: Send + Sync + 'static {
    fn process(&mut self, ctx: &SoundCtx, buf: &mut [StereoFrame]);
    /// How long it keeps sounding after its input ends (echo, reverb)
    fn tail_secs(&self) -> f32 {
        0.0
    }
}

/// Adds a mono value to a frame according to `route`; `quadrature` is the same oscillator a
/// quarter cycle ahead, used by `Orbit`.
pub fn route_into(frame: &mut StereoFrame, route: Route, value: f32, quadrature: f32) {
    match route {
        Route::Left => frame[0] += value,
        Route::Right => frame[1] += value,
        Route::Both => {
            frame[0] += value;
            frame[1] += value;
        }
        Route::Orbit => {
            frame[0] += value;
            frame[1] += quadrature;
        }
    }
}

/// Maps 0..1 onto `range_hz` evenly in pitch, then snaps to `scale`
pub fn pitch_to_hz(pitch: f32, range_hz: [f32; 2], scale: Scale) -> f32 {
    let lo = range_hz[0].max(1.0);
    let hi = range_hz[1].max(lo);
    scale.quantize(lo * (hi / lo).powf(pitch.clamp(0.0, 1.0)))
}
