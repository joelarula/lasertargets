//! Pipeline file syntax: the parsed, not yet validated form of a `.pipeline.yaml`.

use curator_core::Step;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// A parsed pipeline file.
///
/// ```yaml
/// name: hunter/balloon
/// clock: game
/// source:
///   shape: { builtin: balloon, size_m: 0.3 }
/// stages:
///   - scale: { id: flinch, factor: { signal: self.hit, range: [1.0, 1.25] } }
/// reactions:
///   hit:
///     - envelope: { name: hit, decay_ms: 250 }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub clock: Clock,
    /// After this long the pipeline raises `finished` (effects despawn on it)
    #[serde(default)]
    pub lifetime_ms: Option<f32>,
    /// Local signals, visible to this pipeline only
    #[serde(default)]
    pub signals: BTreeMap<String, LocalSignal>,
    /// Generator of the base geometry; without one, the entity's own path is the base
    #[serde(default)]
    pub source: Option<Keyed>,
    #[serde(default)]
    pub stages: Vec<Keyed>,
    /// Trigger key → step tree of reaction actions; a plain list is a `sequence`
    #[serde(default, deserialize_with = "reactions_or_lists")]
    pub reactions: BTreeMap<String, Step<ReactionAction>>,
}

/// Reads each reaction as a step, or a list of steps as a sequence. Goes through a JSON value
/// rather than an untagged enum so errors name the actual problem.
fn reactions_or_lists<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BTreeMap<String, Step<ReactionAction>>, D::Error> {
    let raw = BTreeMap::<String, Value>::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(key, value)| {
            let step = match value {
                Value::Array(_) => serde_json::from_value::<Vec<Step<ReactionAction>>>(value).map(Step::Sequence),
                other => serde_json::from_value::<Step<ReactionAction>>(other),
            };
            step.map(|step| (key.clone(), step)).map_err(|e| serde::de::Error::custom(format!("reactions.{key}: {e}")))
        })
        .collect()
}

impl PipelineSpec {
    /// A pipeline that passes the entity's own shape through unchanged
    pub fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            clock: Clock::Game,
            lifetime_ms: None,
            signals: BTreeMap::new(),
            source: None,
            stages: Vec::new(),
            reactions: BTreeMap::new(),
        }
    }

    /// Parses pipeline YAML. Syntax only; [`super::compile`] validates names and parameters.
    pub fn from_yaml(text: &str) -> Result<Self, String> {
        use figment::providers::{Format, Yaml};
        figment::Figment::from(Yaml::string(text)).extract().map_err(|e| e.to_string())
    }
}

/// Which time a pipeline runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Clock {
    /// Pauses with the game
    #[default]
    Game,
    /// Keeps running while paused (menus, calibration)
    Real,
}

/// A signal computed by the pipeline itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LocalSignal {
    /// 0..1 sine
    Sine {
        frequency_hz: f32,
        #[serde(default)]
        phase: f32,
    },
    /// 0..1 triangle
    Triangle { frequency_hz: f32 },
    /// 0 → 1 over the duration, then holds (or wraps when `repeat`)
    Ramp {
        duration_secs: f32,
        #[serde(default)]
        repeat: bool,
    },
}

impl LocalSignal {
    pub fn sample(&self, t: f32) -> f32 {
        use std::f32::consts::TAU;
        match self {
            Self::Sine { frequency_hz, phase } => ((t * frequency_hz + phase) * TAU).sin() * 0.5 + 0.5,
            Self::Triangle { frequency_hz } => {
                let x = (t * frequency_hz).fract();
                if x < 0.5 { x * 2.0 } else { (1.0 - x) * 2.0 }
            }
            Self::Ramp { duration_secs, repeat } => {
                let x = t / duration_secs.max(1e-4);
                if *repeat { x.fract() } else { x.min(1.0) }
            }
        }
    }
}

/// What a reaction does. Control flow (sequence, parallel, delay, wait, repeat) comes from the
/// curator step tree around these actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAction {
    /// Start an attack/hold/decay curve published as `self.<name>`
    Envelope(EnvelopeSpec),
    /// Change a stage's `enabled` or parameters by stage id, optionally for a while
    Set(SetSpec),
    /// Send a curator event envelope back to the game
    Emit(EmitSpec),
    /// Raise another trigger on the same entity
    Trigger(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvelopeSpec {
    pub name: String,
    #[serde(default)]
    pub attack_ms: f32,
    #[serde(default)]
    pub hold_ms: f32,
    #[serde(default = "default_decay_ms")]
    pub decay_ms: f32,
    #[serde(default = "one")]
    pub peak: f32,
    /// Multiply the peak by the trigger's strength
    #[serde(default = "yes")]
    pub use_strength: bool,
    #[serde(default)]
    pub mode: EnvelopeMode,
}

/// What a new envelope does to running envelopes with the same name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeMode {
    /// Replace them
    #[default]
    Restart,
    /// Run alongside; values add up (clamped to 1)
    Add,
    /// Run alongside; the largest value wins
    Max,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSpec {
    /// Stage id
    pub stage: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Parameters merged into the stage's configuration
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
    /// Restore the previous configuration after this long
    #[serde(default)]
    pub for_ms: Option<f32>,
}

/// `emit: visual_finished` or `emit: { tag: ..., payload: {...} }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmitSpec {
    Tag(String),
    Full {
        tag: String,
        #[serde(default)]
        payload: Value,
    },
}

impl EmitSpec {
    pub fn tag(&self) -> &str {
        match self {
            Self::Tag(tag) | Self::Full { tag, .. } => tag,
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::Tag(_) => Value::Null,
            Self::Full { payload, .. } => payload.clone(),
        }
    }
}

fn default_decay_ms() -> f32 {
    250.0
}

fn one() -> f32 {
    1.0
}

fn yes() -> bool {
    true
}

/// A single-key map naming a registered stage or source: `wave_displace: { ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyed {
    pub key: String,
    pub config: Value,
}

impl<'de> Deserialize<'de> for Keyed {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, Value>::deserialize(deserializer)?;
        let mut entries = map.into_iter();
        match (entries.next(), entries.next()) {
            (Some((key, config)), None) => Ok(Self { key, config }),
            _ => Err(serde::de::Error::custom("expected a single-key map such as `scale: { factor: 2 }`")),
        }
    }
}

impl Serialize for Keyed {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        BTreeMap::from([(&self.key, &self.config)]).serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curator_core::WaitSpec;

    const BALLOON: &str = r#"
name: hunter/balloon
description: test
signals:
  lfo.bob: { sine: { frequency_hz: 0.8 } }
source:
  shape: { builtin: balloon, size_m: 0.3 }
stages:
  - translate: { id: bob, y: { signal: lfo.bob, range: [-0.02, 0.02] } }
  - scale: { id: flinch, factor: 1.0 }
reactions:
  hit:
    - envelope: { name: hit, attack_ms: 5, decay_ms: 250 }
  pop:
    - sequence:
        - parallel:
            - set: { stage: flinch, enabled: false }
            - delay: 0.1
        - wait: finished
        - emit: visual_finished
"#;

    #[test]
    fn parses_full_pipeline_yaml() {
        let spec = PipelineSpec::from_yaml(BALLOON).unwrap();
        assert_eq!(spec.name, "hunter/balloon");
        assert_eq!(spec.clock, Clock::Game);
        assert_eq!(spec.signals["lfo.bob"], LocalSignal::Sine { frequency_hz: 0.8, phase: 0.0 });
        assert_eq!(spec.source.as_ref().unwrap().key, "shape");
        assert_eq!(spec.stages.len(), 2);
        assert_eq!(spec.stages[0].key, "translate");
        assert_eq!(spec.stages[0].config["y"]["range"][1].as_f64(), Some(0.02));

        let Step::Sequence(hit) = &spec.reactions["hit"] else { panic!("hit is a list") };
        let Step::Action(ReactionAction::Envelope(env)) = &hit[0] else { panic!("{hit:?}") };
        assert_eq!((env.name.as_str(), env.attack_ms, env.decay_ms, env.peak), ("hit", 5.0, 250.0, 1.0));

        let Step::Sequence(pop) = &spec.reactions["pop"] else { panic!() };
        let Step::Sequence(inner) = &pop[0] else { panic!("{pop:?}") };
        assert!(matches!(&inner[0], Step::Parallel(p) if p.len() == 2));
        assert_eq!(inner[1], Step::Wait(WaitSpec::Tag("finished".into())));
        assert_eq!(inner[2], Step::Action(ReactionAction::Emit(EmitSpec::Tag("visual_finished".into()))));
    }

    #[test]
    fn rejects_unknown_top_level_keys_and_multi_key_stages() {
        assert!(PipelineSpec::from_yaml("name: x\nbogus: 1\n").is_err());
        let err = PipelineSpec::from_yaml("name: x\nstages:\n  - { scale: {}, rotate: {} }\n").unwrap_err();
        assert!(err.contains("single-key"), "{err}");
    }

    #[test]
    fn local_signals_sample_in_unit_range() {
        let sine = LocalSignal::Sine { frequency_hz: 1.0, phase: 0.0 };
        assert!((sine.sample(0.0) - 0.5).abs() < 1e-5);
        assert!((sine.sample(0.25) - 1.0).abs() < 1e-5);
        let ramp = LocalSignal::Ramp { duration_secs: 2.0, repeat: false };
        assert_eq!(ramp.sample(1.0), 0.5);
        assert_eq!(ramp.sample(5.0), 1.0);
        let tri = LocalSignal::Triangle { frequency_hz: 1.0 };
        assert_eq!(tri.sample(0.25), 0.5);
        assert_eq!(tri.sample(0.5), 1.0);
    }
}
