use super::stage::StageCtx;
use crate::path::EasingFunction;
use serde::{Deserialize, Serialize};

/// A numeric stage parameter: a constant, or a live signal mapped onto a range.
///
/// ```yaml
/// radius_m: 0.02                                             # constant
/// amplitude_m: { signal: self.hit, range: [0.0, 0.03] }      # bound
/// scale: { signal: audio.bass, range: [1, 1.4], curve: EaseOutQuad, smooth_ms: 40 }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Param {
    Const(f32),
    Bind(Binding),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub signal: String,
    /// Output for signal 0 and 1; default [0, 1]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
    #[serde(default)]
    pub curve: EasingFunction,
    /// One-pole smoothing time constant
    #[serde(default)]
    pub smooth_ms: f32,
    #[serde(skip)]
    smoothed: Option<f32>,
}

impl Default for Param {
    fn default() -> Self {
        Self::Const(0.0)
    }
}

impl From<f32> for Param {
    fn from(value: f32) -> Self {
        Self::Const(value)
    }
}

impl Param {
    pub fn bind(signal: impl Into<String>, range: [f32; 2]) -> Self {
        Self::Bind(Binding {
            signal: signal.into(),
            range: Some(range),
            curve: EasingFunction::Linear,
            smooth_ms: 0.0,
            smoothed: None,
        })
    }

    /// True when the value follows a signal, which makes the owning stage dynamic
    pub fn is_bound(&self) -> bool {
        matches!(self, Self::Bind(_))
    }

    /// Current value. Advances smoothing, so call it once per frame per parameter.
    pub fn get(&mut self, ctx: &StageCtx) -> f32 {
        match self {
            Self::Const(value) => *value,
            Self::Bind(b) => {
                let s = b.curve.apply(ctx.signals.get(&b.signal).clamp(0.0, 1.0));
                let [lo, hi] = b.range.unwrap_or([0.0, 1.0]);
                let target = lo + (hi - lo) * s;
                if b.smooth_ms <= 0.0 {
                    return target;
                }
                let alpha = 1.0 - (-ctx.delta_secs * 1000.0 / b.smooth_ms).exp();
                let value = match b.smoothed {
                    Some(prev) => prev + (target - prev) * alpha,
                    None => target,
                };
                b.smoothed = Some(value);
                value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::signals::{EntitySignals, SignalView, Signals};

    fn ctx<'a>(entity: &'a EntitySignals, global: &'a Signals, dt: f32) -> StageCtx<'a> {
        StageCtx { delta_secs: dt, elapsed_secs: 0.0, signals: SignalView::new(entity, global) }
    }

    #[test]
    fn deserializes_const_and_binding() {
        let c: Param = serde_json::from_str("0.5").unwrap();
        assert_eq!(c, Param::Const(0.5));
        let b: Param = serde_json::from_str(r#"{"signal":"self.hit","range":[1.0,2.0]}"#).unwrap();
        assert!(b.is_bound());
        assert!(serde_json::from_str::<Param>(r#"{"signal":"x","bogus":1}"#).is_err());
    }

    #[test]
    fn maps_clamps_and_curves() {
        let mut entity = EntitySignals::default();
        let global = Signals::default();
        let mut p = Param::bind("self.hit", [10.0, 20.0]);

        entity.set("hit", 0.5);
        assert_eq!(p.get(&ctx(&entity, &global, 0.0)), 15.0);
        entity.set("hit", 3.0);
        assert_eq!(p.get(&ctx(&entity, &global, 0.0)), 20.0, "signal clamps to 0..1");

        let mut curved: Param =
            serde_json::from_str(r#"{"signal":"self.hit","curve":"EaseInQuad"}"#).unwrap();
        entity.set("hit", 0.5);
        assert_eq!(curved.get(&ctx(&entity, &global, 0.0)), 0.25);
    }

    #[test]
    fn smoothing_approaches_target() {
        let mut entity = EntitySignals::default();
        let global = Signals::default();
        let mut p: Param = serde_json::from_str(r#"{"signal":"self.x","smooth_ms":50}"#).unwrap();

        entity.set("x", 0.0);
        assert_eq!(p.get(&ctx(&entity, &global, 0.02)), 0.0);
        entity.set("x", 1.0);
        let first = p.get(&ctx(&entity, &global, 0.02));
        assert!(first > 0.0 && first < 1.0, "{first}");
        for _ in 0..50 {
            p.get(&ctx(&entity, &global, 0.02));
        }
        assert!(p.get(&ctx(&entity, &global, 0.02)) > 0.99);
    }
}
