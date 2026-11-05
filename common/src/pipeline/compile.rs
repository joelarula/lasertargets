//! Resolves a parsed [`PipelineSpec`] against the registry: every stage and source name must be
//! registered, every parameter must deserialize, stage ids must be unique and every reaction
//! must name existing stages. All problems are reported at once, each with its location.

use super::registry::PipelineRegistry;
use super::spec::{Clock, Keyed, LocalSignal, PipelineSpec, ReactionAction};
use curator_core::Step;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::fmt;

/// One validation problem, located like `stages[2].wave_displace` or `reactions.hit`.
#[derive(Debug, Clone, PartialEq)]
pub struct SpecError {
    pub at: String,
    pub message: String,
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.at, self.message)
    }
}

/// A stage entry with its id and switches split from its parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct StageDef {
    pub id: String,
    pub key: String,
    pub enabled: bool,
    /// Parameters only, as passed to the stage factory
    pub config: Value,
}

/// A validated pipeline, ready to instantiate per entity.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledPipeline {
    pub name: String,
    pub clock: Clock,
    pub lifetime_secs: Option<f32>,
    pub signals: BTreeMap<String, LocalSignal>,
    pub source: Option<(String, Value)>,
    pub stages: Vec<StageDef>,
    pub reactions: BTreeMap<String, Step<ReactionAction>>,
}

impl CompiledPipeline {
    pub fn stage_index(&self, id: &str) -> Option<usize> {
        self.stages.iter().position(|s| s.id == id)
    }
}

pub fn compile(spec: &PipelineSpec, registry: &PipelineRegistry) -> Result<CompiledPipeline, Vec<SpecError>> {
    let mut errors = Vec::new();
    let mut error = |at: String, message: String| errors.push(SpecError { at, message });

    let source = spec.source.as_ref().and_then(|Keyed { key, config }| {
        let config = params_object(config);
        match registry.build_source(key, &config) {
            Ok(_) => Some((key.clone(), config)),
            Err(e) => {
                error(format!("source.{key}"), e);
                None
            }
        }
    });

    let mut stages = Vec::new();
    let mut ids = HashSet::new();
    for (i, Keyed { key, config }) in spec.stages.iter().enumerate() {
        let at = format!("stages[{i}].{key}");
        let mut params = match config {
            Value::Object(map) => map.clone(),
            Value::Null => Map::new(),
            other => {
                error(at, format!("expected a map of parameters, found {other}"));
                continue;
            }
        };
        let id = match params.remove("id") {
            Some(Value::String(id)) => id,
            Some(other) => {
                error(at.clone(), format!("`id` must be a string, found {other}"));
                format!("{key}#{i}")
            }
            None => format!("{key}#{i}"),
        };
        let enabled = match params.remove("enabled") {
            Some(Value::Bool(b)) => b,
            Some(other) => {
                error(at.clone(), format!("`enabled` must be true or false, found {other}"));
                true
            }
            None => true,
        };
        if !ids.insert(id.clone()) {
            error(at.clone(), format!("duplicate stage id `{id}`"));
        }
        let config = Value::Object(params);
        if let Err(e) = registry.build_stage(key, &config) {
            error(at, e);
        }
        stages.push(StageDef { id, key: key.clone(), enabled, config });
    }

    for (trigger, step) in &spec.reactions {
        step.for_each_action(&mut |action| {
            let ReactionAction::Set(set) = action else { return };
            let at = format!("reactions.{trigger}");
            match stages.iter().find(|s| s.id == set.stage) {
                None => error(at, format!("`set` names unknown stage id `{}`", set.stage)),
                Some(stage) if !set.params.is_empty() => {
                    let merged = merge(&stage.config, &set.params);
                    if let Err(e) = registry.build_stage(&stage.key, &merged) {
                        error(at, format!("`set` on `{}`: {e}", set.stage));
                    }
                }
                Some(_) => {}
            }
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(CompiledPipeline {
        name: spec.name.clone(),
        clock: spec.clock,
        lifetime_secs: spec.lifetime_ms.map(|ms| ms / 1000.0),
        signals: spec.signals.clone(),
        source,
        stages,
        reactions: spec.reactions.clone(),
    })
}

/// `config` with `params` merged over it (shallow)
pub fn merge(config: &Value, params: &Map<String, Value>) -> Value {
    let mut merged = match config {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    merged.extend(params.iter().map(|(k, v)| (k.clone(), v.clone())));
    Value::Object(merged)
}

fn params_object(config: &Value) -> Value {
    match config {
        Value::Null => Value::Object(Map::new()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::UniversalPath;
    use crate::pipeline::stage::{PathStage, StageCtx, StageKind};
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Nop {
        #[serde(default)]
        #[allow(dead_code)]
        amount: f32,
    }

    impl PathStage for Nop {
        fn kind(&self) -> StageKind {
            StageKind::Static
        }
        fn apply(&mut self, input: &UniversalPath, _ctx: &StageCtx, out: &mut UniversalPath) {
            out.clone_from(input);
        }
    }

    fn registry() -> PipelineRegistry {
        let mut r = PipelineRegistry::default();
        r.register_stage::<Nop>("nop");
        r
    }

    fn compile_yaml(yaml: &str) -> Result<CompiledPipeline, Vec<SpecError>> {
        compile(&PipelineSpec::from_yaml(yaml).unwrap(), &registry())
    }

    #[test]
    fn splits_id_and_enabled_from_params() {
        let plan = compile_yaml("name: t\nstages:\n  - nop: { id: a, enabled: false, amount: 2 }\n  - nop: {}\n").unwrap();
        assert_eq!(plan.stages[0].id, "a");
        assert!(!plan.stages[0].enabled);
        assert_eq!(plan.stages[0].config, serde_json::json!({ "amount": 2 }));
        assert_eq!(plan.stages[1].id, "nop#1");
        assert_eq!(plan.stage_index("nop#1"), Some(1));
    }

    #[test]
    fn reports_every_problem_with_its_location() {
        let errors = compile_yaml(
            "name: t
stages:
  - nop: { id: a }
  - nop: { id: a }
  - nope: {}
  - nop: { amount: high }
reactions:
  hit:
    - set: { stage: ghost }
    - set: { stage: a, params: { bogus: 1 } }
",
        )
        .unwrap_err();
        let text: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(errors.len(), 5, "{text:#?}");
        assert!(text[0].starts_with("stages[1].nop: duplicate stage id `a`"));
        assert!(text[1].starts_with("stages[2].nope: unknown stage `nope` (known: nop)"));
        assert!(text[2].starts_with("stages[3].nop: "));
        assert!(text[3].contains("unknown stage id `ghost`"));
        assert!(text[4].contains("`set` on `a`") && text[4].contains("bogus"));
    }
}
