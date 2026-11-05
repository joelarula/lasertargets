//! Built-in pipeline stages. Coordinates are metres in the entity's local frame.
//!
//! Every stage is pure: it copies its input into the output and changes the copy. A stage is
//! dynamic when one of its parameters is bound to a signal or it animates with time.

use super::param::Param;
use super::registry::PipelineRegistry;
use super::stage::{map_points, PathStage, StageCtx, StageKind};
use crate::path::{PathPoint, UniversalPath};
use serde::Deserialize;
use std::f32::consts::TAU;

/// Registers every built-in stage
pub fn register(registry: &mut PipelineRegistry) {
    registry
        .register_stage::<Translate>("translate")
        .register_stage::<Scale>("scale")
        .register_stage::<Rotate>("rotate")
        .register_stage::<Fade>("fade")
        .register_stage::<ColorLerp>("color_lerp")
        .register_stage::<WaveDisplace>("wave_displace")
        .register_stage::<Wobble>("wobble")
        .register_stage::<Trim>("trim");
}

fn kind_of(params: &[&Param]) -> StageKind {
    if params.iter().any(|p| p.is_bound()) { StageKind::Dynamic } else { StageKind::Static }
}

fn one() -> Param {
    Param::Const(1.0)
}

/// Moves the path: `translate: { x: 0.1, y: { signal: lfo.bob, range: [-0.02, 0.02] } }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Translate {
    #[serde(default)]
    x: Param,
    #[serde(default)]
    y: Param,
}

impl PathStage for Translate {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.x, &self.y])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let (dx, dy) = (self.x.get(ctx), self.y.get(ctx));
        map_points(input, out, |p| {
            p.x += dx;
            p.y += dy;
        });
    }
}

/// Scales around the local origin: `scale: { factor: { signal: self.hit, range: [1, 1.25] } }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    #[serde(default = "one")]
    factor: Param,
}

impl PathStage for Scale {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.factor])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let k = self.factor.get(ctx);
        map_points(input, out, |p| {
            p.x *= k;
            p.y *= k;
        });
    }
}

/// Rotates around the local origin: `rotate: { deg: 45 }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rotate {
    #[serde(default)]
    deg: Param,
}

impl PathStage for Rotate {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.deg])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let (sin, cos) = self.deg.get(ctx).to_radians().sin_cos();
        map_points(input, out, |p| {
            let (x, y) = (p.x, p.y);
            p.x = x * cos - y * sin;
            p.y = x * sin + y * cos;
        });
    }
}

fn scale_color(p: &mut PathPoint, k: f32) {
    let k = k.clamp(0.0, 1.0);
    p.r = (p.r as f32 * k).round() as u8;
    p.g = (p.g as f32 * k).round() as u8;
    p.b = (p.b as f32 * k).round() as u8;
}

/// Brightness multiplier, 0..1: `fade: { amount: { signal: self.life, range: [1, 0] } }`.
/// At 0 every point is dark, so the projector skips the path.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fade {
    #[serde(default = "one")]
    amount: Param,
}

impl PathStage for Fade {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.amount])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let k = self.amount.get(ctx);
        map_points(input, out, |p| scale_color(p, k));
    }
}

/// Blends lit points towards a colour: `color_lerp: { to: [255, 40, 40], amount: 0.5 }`.
/// Dark (blanked) points stay dark.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorLerp {
    to: [u8; 3],
    #[serde(default)]
    amount: Param,
}

impl PathStage for ColorLerp {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.amount])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let t = self.amount.get(ctx).clamp(0.0, 1.0);
        let [r, g, b] = self.to;
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        map_points(input, out, |p| {
            if (p.r, p.g, p.b) != (0, 0, 0) {
                (p.r, p.g, p.b) = (lerp(p.r, r), lerp(p.g, g), lerp(p.b, b));
            }
        });
    }
}

#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    #[default]
    Y,
}

/// Sine ripple across the path: points move along `axis` by
/// `sin(TAU * (other_coord * frequency + t * speed_hz)) * amplitude_m`.
///
/// `wave_displace: { axis: y, frequency: 16, speed_hz: 2, amplitude_m: { signal: audio.bass, range: [0, 0.03] } }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaveDisplace {
    #[serde(default)]
    axis: Axis,
    /// Cycles per metre along the other axis
    frequency: f32,
    #[serde(default)]
    speed_hz: f32,
    amplitude_m: Param,
}

impl PathStage for WaveDisplace {
    fn kind(&self) -> StageKind {
        if self.speed_hz != 0.0 { StageKind::Dynamic } else { kind_of(&[&self.amplitude_m]) }
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let amplitude = self.amplitude_m.get(ctx);
        let phase = ctx.elapsed_secs * self.speed_hz;
        let (axis, frequency) = (self.axis, self.frequency);
        map_points(input, out, |p| match axis {
            Axis::Y => p.y += (TAU * (p.x * frequency + phase)).sin() * amplitude,
            Axis::X => p.x += (TAU * (p.y * frequency + phase)).sin() * amplitude,
        });
    }
}

/// Shake: every point circles around its place with its own phase.
/// `wobble: { amplitude_m: { signal: self.near, range: [0, 0.01] }, frequency_hz: 12 }`
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wobble {
    amplitude_m: Param,
    #[serde(default = "default_wobble_hz")]
    frequency_hz: f32,
}

fn default_wobble_hz() -> f32 {
    12.0
}

impl PathStage for Wobble {
    fn kind(&self) -> StageKind {
        StageKind::Dynamic
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let amplitude = self.amplitude_m.get(ctx);
        let base = ctx.elapsed_secs * self.frequency_hz;
        let mut i = 0u32;
        map_points(input, out, |p| {
            // Golden-ratio spacing keeps neighbouring points out of phase
            let phase = TAU * (base + i as f32 * 0.618_034);
            p.x += phase.cos() * amplitude;
            p.y += (phase * 1.3).sin() * amplitude;
            i += 1;
        });
    }
}

/// Draws only the first `amount` (0..1) of the path's length, for draw-on and draw-off effects:
/// `trim: { amount: { signal: self.life } }`. Segments are measured in order as one path.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trim {
    #[serde(default = "one")]
    amount: Param,
}

impl PathStage for Trim {
    fn kind(&self) -> StageKind {
        kind_of(&[&self.amount])
    }

    fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
        let amount = self.amount.get(ctx).clamp(0.0, 1.0);
        out.clone_from(input);
        if amount >= 1.0 {
            return;
        }
        let length = |pts: &[PathPoint]| pts.windows(2).map(|w| dist(&w[0], &w[1])).sum::<f32>();
        let total: f32 = input.segments.iter().map(|s| length(&s.points)).sum();
        let mut budget = total * amount;
        for segment in &mut out.segments {
            let points = &mut segment.points;
            // (points kept, interpolated end point)
            let mut cut = None;
            for j in 0..points.len().saturating_sub(1) {
                if budget <= 0.0 {
                    cut = Some((j + 1, None));
                    break;
                }
                let d = dist(&points[j], &points[j + 1]);
                if budget >= d {
                    budget -= d;
                } else {
                    cut = Some((j + 1, Some(points[j].lerp(&points[j + 1], budget / d))));
                    budget = 0.0;
                    break;
                }
            }
            if let Some((keep, end)) = cut {
                points.truncate(keep);
                points.extend(end);
            }
        }
        out.segments.retain(|s| s.points.len() >= 2);
    }
}

fn dist(a: &PathPoint, b: &PathPoint) -> f32 {
    ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathSegment;
    use crate::pipeline::{EntitySignals, SignalView, Signals};
    use serde_json::json;

    fn line() -> UniversalPath {
        UniversalPath::from_segment(PathSegment::new(vec![
            PathPoint::new(0.0, 0.0, 200, 100, 0, 0),
            PathPoint::new(1.0, 0.0, 200, 100, 0, 0),
            PathPoint::new(1.0, 1.0, 200, 100, 0, 0),
        ]))
    }

    fn run(key: &str, config: serde_json::Value, input: &UniversalPath) -> (UniversalPath, StageKind) {
        let mut registry = PipelineRegistry::default();
        register(&mut registry);
        let mut stage = registry.build_stage(key, &config).unwrap();
        let (entity, global) = (EntitySignals::default(), Signals::default());
        let ctx = StageCtx { delta_secs: 0.02, elapsed_secs: 0.0, signals: SignalView::new(&entity, &global) };
        let mut out = UniversalPath::default();
        stage.apply(input, &ctx, &mut out);
        (out, stage.kind())
    }

    fn xy(path: &UniversalPath) -> Vec<(f32, f32)> {
        path.segments.iter().flat_map(|s| s.points.iter()).map(|p| ((p.x * 1e4).round() / 1e4, (p.y * 1e4).round() / 1e4)).collect()
    }

    #[test]
    fn affine_stages() {
        let (t, kind) = run("translate", json!({ "x": 1, "y": -1 }), &line());
        assert_eq!(xy(&t)[0], (1.0, -1.0));
        assert_eq!(kind, StageKind::Static);
        let (s, _) = run("scale", json!({ "factor": 2 }), &line());
        assert_eq!(xy(&s)[2], (2.0, 2.0));
        let (r, _) = run("rotate", json!({ "deg": 90 }), &line());
        assert_eq!(xy(&r)[1], (0.0, 1.0));
        let (_, bound) = run("scale", json!({ "factor": { "signal": "self.hit" } }), &line());
        assert_eq!(bound, StageKind::Dynamic);
    }

    #[test]
    fn color_stages_keep_dark_points_dark() {
        let (f, _) = run("fade", json!({ "amount": 0.5 }), &line());
        assert_eq!((f.segments[0].points[0].r, f.segments[0].points[0].g), (100, 50));
        let mut input = line();
        input.segments[0].points[1] = PathPoint::new(1.0, 0.0, 0, 0, 0, 0);
        let (c, _) = run("color_lerp", json!({ "to": [0, 0, 255], "amount": 1 }), &input);
        let p = &c.segments[0].points;
        assert_eq!((p[0].r, p[0].g, p[0].b), (0, 0, 255));
        assert_eq!((p[1].r, p[1].g, p[1].b), (0, 0, 0));
    }

    #[test]
    fn wave_displace_follows_other_axis() {
        let (w, kind) = run("wave_displace", json!({ "frequency": 0.25, "amplitude_m": 0.1 }), &line());
        assert_eq!(kind, StageKind::Static);
        assert_eq!(xy(&w), vec![(0.0, 0.0), (1.0, 0.1), (1.0, 1.1)]);
        let (_, moving) = run("wave_displace", json!({ "frequency": 1, "speed_hz": 2, "amplitude_m": 0.1 }), &line());
        assert_eq!(moving, StageKind::Dynamic);
    }

    #[test]
    fn trim_cuts_by_length() {
        let (half, _) = run("trim", json!({ "amount": 0.5 }), &line());
        assert_eq!(xy(&half), vec![(0.0, 0.0), (1.0, 0.0)]);
        let (quarter, _) = run("trim", json!({ "amount": 0.25 }), &line());
        assert_eq!(xy(&quarter), vec![(0.0, 0.0), (0.5, 0.0)]);
        let (three, _) = run("trim", json!({ "amount": 0.75 }), &line());
        assert_eq!(xy(&three), vec![(0.0, 0.0), (1.0, 0.0), (1.0, 0.5)]);
        let (none, _) = run("trim", json!({ "amount": 0.0 }), &line());
        assert!(none.segments.is_empty());
        let (all, _) = run("trim", json!({}), &line());
        assert_eq!(all, line());
    }

    #[test]
    fn unknown_parameters_are_rejected() {
        let mut registry = PipelineRegistry::default();
        register(&mut registry);
        assert!(registry.build_stage("scale", &json!({ "factr": 2 })).is_err());
        assert!(registry.build_stage("wave_displace", &json!({ "frequency": 1 })).is_err(), "amplitude_m is required");
    }
}
