//! Per-entity pipeline state and evaluation.
//!
//! Evaluation per frame: source → `base` (when it changes) → static prefix → `static_path`
//! (cached) → dynamic suffix → output. Every stage reads an input and writes a separate buffer,
//! so nothing accumulates across frames. The prefix ends at the first enabled dynamic stage.

use super::compile::{merge, CompiledPipeline};
use super::registry::PipelineRegistry;
use super::signals::{EntitySignals, SignalView, Signals};
use super::spec::{EnvelopeMode, EnvelopeSpec, ReactionAction};
use super::stage::{PathSource, PathStage, StageCtx, StageKind};
use crate::path::UniversalPath;
use curator_core::StepRunner;
use serde_json::Value;
use std::sync::Arc;

/// Something a reaction asks the host to do outside the pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum ReactionOutput {
    /// Send a curator event envelope
    Emit { tag: String, payload: Value },
}

struct StageInstance {
    id: String,
    key: String,
    enabled: bool,
    config: Value,
    stage: Box<dyn PathStage>,
    /// Configuration to restore when a timed `set` runs out
    restore: Option<(f32, bool, Value)>,
}

struct Envelope {
    spec: EnvelopeSpec,
    peak: f32,
    t: f32,
}

impl Envelope {
    fn value(&self) -> f32 {
        let (a, h, d) = (self.spec.attack_ms / 1000.0, self.spec.hold_ms / 1000.0, self.spec.decay_ms / 1000.0);
        let t = self.t;
        let shape = if t < a {
            t / a
        } else if t < a + h {
            1.0
        } else if t < a + h + d {
            1.0 - (t - a - h) / d
        } else {
            0.0
        };
        shape * self.peak
    }

    fn done(&self) -> bool {
        self.t >= (self.spec.attack_ms + self.spec.hold_ms + self.spec.decay_ms) / 1000.0
    }
}

pub struct PipelineInstance {
    plan: Arc<CompiledPipeline>,
    source: Option<Box<dyn PathSource>>,
    stages: Vec<StageInstance>,
    base: UniversalPath,
    static_path: UniversalPath,
    scratch: [UniversalPath; 2],
    base_dirty: bool,
    prefix_dirty: bool,
    /// Running reactions with the strength of the trigger that started them
    runners: Vec<(StepRunner<ReactionAction>, f32)>,
    envelopes: Vec<Envelope>,
    /// Envelope names ever started, published as 0 once they end
    envelope_names: Vec<String>,
    pending: Vec<(String, f32)>,
    age_secs: f32,
    started: bool,
    finished: bool,
}

impl PipelineInstance {
    pub fn new(plan: Arc<CompiledPipeline>, registry: &PipelineRegistry) -> Result<Self, String> {
        let source = match &plan.source {
            Some((key, config)) => Some(registry.build_source(key, config)?),
            None => None,
        };
        let stages = plan
            .stages
            .iter()
            .map(|def| {
                Ok(StageInstance {
                    id: def.id.clone(),
                    key: def.key.clone(),
                    enabled: def.enabled,
                    config: def.config.clone(),
                    stage: registry.build_stage(&def.key, &def.config)?,
                    restore: None,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self {
            plan,
            source,
            stages,
            base: UniversalPath::default(),
            static_path: UniversalPath::default(),
            scratch: Default::default(),
            base_dirty: true,
            prefix_dirty: true,
            runners: Vec::new(),
            envelopes: Vec::new(),
            envelope_names: Vec::new(),
            pending: Vec::new(),
            age_secs: 0.0,
            started: false,
            finished: false,
        })
    }

    pub fn plan(&self) -> &Arc<CompiledPipeline> {
        &self.plan
    }

    /// Switches to a new version of the pipeline (hot reload). Stages whose id, type and
    /// parameters are unchanged keep their instance and state; reactions and envelopes keep
    /// running.
    pub fn rebind(&mut self, plan: Arc<CompiledPipeline>, registry: &PipelineRegistry) -> Result<(), String> {
        let mut next = Self::new(plan, registry)?;
        for stage in &mut next.stages {
            if let Some(i) = self.stages.iter().position(|old| old.id == stage.id && old.key == stage.key && old.config == stage.config) {
                let old = self.stages.swap_remove(i);
                stage.stage = old.stage;
            }
        }
        if next.plan.source == self.plan.source {
            next.source = self.source.take();
            next.base = std::mem::take(&mut self.base);
            next.base_dirty = self.base_dirty;
        }
        next.runners = std::mem::take(&mut self.runners);
        next.envelopes = std::mem::take(&mut self.envelopes);
        next.envelope_names = std::mem::take(&mut self.envelope_names);
        next.age_secs = self.age_secs;
        next.started = self.started;
        next.finished = self.finished;
        *self = next;
        Ok(())
    }

    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    /// Replaces the base geometry of a pipeline without a source (the entity's own shape)
    pub fn set_base(&mut self, base: &UniversalPath) {
        if self.source.is_none() {
            self.base.clone_from(base);
            self.prefix_dirty = true;
        }
    }

    /// Queues a trigger; it is handled at the next [`Self::tick`]
    pub fn trigger(&mut self, key: impl Into<String>, strength: f32) {
        self.pending.push((key.into(), strength));
    }

    /// True once the lifetime has run out (`finished` has been raised)
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether the pipeline has a reaction for `key`
    pub fn reacts_to(&self, key: &str) -> bool {
        self.plan.reactions.contains_key(key)
    }

    /// Advances clocks, envelopes, timed `set`s and reactions by `dt`, handling queued triggers,
    /// and publishes this pipeline's signals into `signals`.
    pub fn tick(&mut self, dt: f32, signals: &mut EntitySignals, registry: &PipelineRegistry) -> Vec<ReactionOutput> {
        let mut outputs = Vec::new();
        if !self.started {
            self.started = true;
            self.pending.insert(0, ("bound".into(), 1.0));
        } else {
            self.age_secs += dt;
        }
        if let Some(lifetime) = self.plan.lifetime_secs {
            if !self.finished && self.age_secs >= lifetime {
                self.finished = true;
                self.pending.push(("finished".into(), 1.0));
            }
        }

        // Timed `set`s run out
        for stage in &mut self.stages {
            if let Some((left, _, _)) = &mut stage.restore {
                *left -= dt;
                if *left <= 0.0 {
                    let (_, enabled, config) = stage.restore.take().unwrap();
                    if let Ok(rebuilt) = registry.build_stage(&stage.key, &config) {
                        stage.stage = rebuilt;
                    }
                    stage.enabled = enabled;
                    stage.config = config;
                    self.prefix_dirty = true;
                }
            }
        }

        for envelope in &mut self.envelopes {
            envelope.t += dt;
        }

        // Each pass: queued triggers notify waiting runners and start new reactions, then every
        // runner advances (by dt in the first pass, 0 afterwards, so instant steps chain within
        // this frame) and its actions apply. Actions can raise triggers; the cascade is bounded.
        let mut dt_left = dt;
        for _ in 0..8 {
            let mut started = Vec::new();
            for (key, strength) in std::mem::take(&mut self.pending) {
                for (runner, _) in &mut self.runners {
                    runner.notify(&key);
                }
                if let Some(step) = self.plan.reactions.get(&key) {
                    started.push((StepRunner::new(step), strength));
                }
            }
            let mut actions = Vec::new();
            for (runner, strength) in &mut self.runners {
                runner.tick(dt_left, |a| actions.push(with_strength(a.clone(), *strength)));
            }
            for (runner, strength) in &mut started {
                runner.tick(0.0, |a| actions.push(with_strength(a.clone(), *strength)));
            }
            self.runners.append(&mut started);
            dt_left = 0.0;
            for action in actions {
                self.apply(action, registry, &mut outputs);
            }
            if self.pending.is_empty() {
                break;
            }
        }
        self.runners.retain(|(runner, _)| runner.is_running());

        self.publish(signals);
        outputs
    }

    fn apply(&mut self, action: ReactionAction, registry: &PipelineRegistry, outputs: &mut Vec<ReactionOutput>) {
        match action {
            ReactionAction::Envelope(spec) => {
                // The trigger strength is already folded into `peak` (with_strength)
                let peak = spec.peak;
                if spec.mode == EnvelopeMode::Restart {
                    self.envelopes.retain(|e| e.spec.name != spec.name);
                }
                if !self.envelope_names.contains(&spec.name) {
                    self.envelope_names.push(spec.name.clone());
                }
                self.envelopes.push(Envelope { spec, peak, t: 0.0 });
            }
            ReactionAction::Set(set) => {
                let Some(stage) = self.stages.iter_mut().find(|s| s.id == set.stage) else { return };
                let previous = (stage.enabled, stage.config.clone());
                if !set.params.is_empty() {
                    let merged = merge(&stage.config, &set.params);
                    match registry.build_stage(&stage.key, &merged) {
                        Ok(rebuilt) => {
                            stage.stage = rebuilt;
                            stage.config = merged;
                        }
                        Err(e) => bevy::log::warn!("pipeline `{}`: set on `{}`: {e}", self.plan.name, set.stage),
                    }
                }
                if let Some(enabled) = set.enabled {
                    stage.enabled = enabled;
                }
                if let Some(ms) = set.for_ms {
                    // Keep the oldest configuration when timed sets overlap
                    let (enabled, config) = stage.restore.take().map(|(_, e, c)| (e, c)).unwrap_or(previous);
                    stage.restore = Some((ms / 1000.0, enabled, config));
                }
                self.prefix_dirty = true;
            }
            ReactionAction::Emit(emit) => {
                outputs.push(ReactionOutput::Emit { tag: emit.tag().to_string(), payload: emit.payload() });
            }
            ReactionAction::Trigger(key) => self.pending.push((key, 1.0)),
        }
    }

    fn publish(&mut self, signals: &mut EntitySignals) {
        signals.set("age_secs", self.age_secs);
        let life = match self.plan.lifetime_secs {
            Some(lifetime) => (self.age_secs / lifetime.max(1e-4)).min(1.0),
            None => 0.0,
        };
        signals.set("life", life);
        for (name, local) in &self.plan.signals {
            signals.set(name.clone(), local.sample(self.age_secs));
        }
        for name in &self.envelope_names {
            let values = self.envelopes.iter().filter(|e| &e.spec.name == name);
            let value = match values.clone().next().map(|e| e.spec.mode) {
                Some(EnvelopeMode::Add) => values.map(Envelope::value).sum::<f32>(),
                Some(_) => values.map(Envelope::value).fold(0.0, f32::max),
                None => 0.0,
            };
            signals.set(name.clone(), value.clamp(0.0, 1.0));
        }
        self.envelopes.retain(|e| !e.done());
    }

    /// Evaluates the pipeline into `out`. Returns false when `out` was left unchanged because
    /// nothing in the pipeline changed.
    pub fn evaluate(&mut self, signals: &EntitySignals, global: &Signals, elapsed_secs: f32, delta_secs: f32, out: &mut UniversalPath) -> bool {
        let ctx = StageCtx { delta_secs, elapsed_secs, signals: SignalView::new(signals, global) };
        let Self { source, stages, base, static_path, scratch, base_dirty, prefix_dirty, .. } = self;

        if let Some(source) = source {
            if *base_dirty || source.kind() == StageKind::Dynamic {
                source.generate(&ctx, base);
                *base_dirty = false;
                *prefix_dirty = true;
            }
        }

        let enabled: Vec<usize> = (0..stages.len()).filter(|&i| stages[i].enabled).collect();
        let split = enabled.iter().position(|&i| stages[i].stage.kind() == StageKind::Dynamic).unwrap_or(enabled.len());
        let (prefix, suffix) = enabled.split_at(split);

        let prefix_changed = std::mem::take(prefix_dirty);
        if prefix_changed {
            let [a, b] = scratch;
            run_chain(stages, prefix, base, &ctx, a, b, static_path);
        }
        if suffix.is_empty() {
            if prefix_changed {
                out.clone_from(static_path);
            }
            return prefix_changed;
        }
        let [a, b] = scratch;
        run_chain(stages, suffix, static_path, &ctx, a, b, out);
        true
    }
}

/// Folds the strength of the trigger that started a reaction into its envelope peaks
fn with_strength(action: ReactionAction, strength: f32) -> ReactionAction {
    match action {
        ReactionAction::Envelope(mut spec) if spec.use_strength => {
            spec.peak *= strength;
            ReactionAction::Envelope(spec)
        }
        other => other,
    }
}

/// Runs `stages[order]` from `input` into `output`, ping-ponging through `a` and `b`.
fn run_chain(
    stages: &mut [StageInstance],
    order: &[usize],
    input: &UniversalPath,
    ctx: &StageCtx,
    a: &mut UniversalPath,
    b: &mut UniversalPath,
    output: &mut UniversalPath,
) {
    let n = order.len();
    if n == 0 {
        output.clone_from(input);
        return;
    }
    for (k, &i) in order.iter().enumerate() {
        let stage = &mut stages[i].stage;
        let last = k + 1 == n;
        // Step k reads what step k-1 wrote: a for even k-1, b for odd; it writes a for even k,
        // b for odd, and the output when last
        match (k == 0, k % 2 == 0, last) {
            (true, _, true) => stage.apply(input, ctx, output),
            (true, _, false) => stage.apply(input, ctx, a),
            (false, false, true) => stage.apply(a, ctx, output),
            (false, false, false) => stage.apply(a, ctx, b),
            (false, true, true) => stage.apply(b, ctx, output),
            (false, true, false) => stage.apply(b, ctx, a),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{PathPoint, PathSegment};
    use crate::pipeline::compile::compile;
    use crate::pipeline::param::Param;
    use crate::pipeline::spec::PipelineSpec;
    use serde::Deserialize;
    use std::cell::Cell;

    thread_local! {
        /// Static Shift runs on this test's thread (tests run in parallel)
        static STATIC_RUNS: Cell<usize> = const { Cell::new(0) };
    }

    /// x += dx; dynamic when dx is bound
    #[derive(Deserialize)]
    struct Shift {
        dx: Param,
    }

    impl PathStage for Shift {
        fn kind(&self) -> StageKind {
            if self.dx.is_bound() { StageKind::Dynamic } else { StageKind::Static }
        }
        fn apply(&mut self, input: &UniversalPath, ctx: &StageCtx, out: &mut UniversalPath) {
            if !self.dx.is_bound() {
                STATIC_RUNS.with(|n| n.set(n.get() + 1));
            }
            let dx = self.dx.get(ctx);
            super::super::stage::map_points(input, out, |p| p.x += dx);
        }
    }

    /// x *= k, always static
    #[derive(Deserialize)]
    struct Mul {
        k: f32,
    }

    impl PathStage for Mul {
        fn kind(&self) -> StageKind {
            StageKind::Static
        }
        fn apply(&mut self, input: &UniversalPath, _ctx: &StageCtx, out: &mut UniversalPath) {
            super::super::stage::map_points(input, out, |p| p.x *= self.k);
        }
    }

    fn registry() -> PipelineRegistry {
        let mut r = PipelineRegistry::default();
        r.register_stage::<Shift>("shift").register_stage::<Mul>("mul");
        r
    }

    fn instance(yaml: &str) -> (PipelineInstance, PipelineRegistry) {
        let registry = registry();
        let plan = compile(&PipelineSpec::from_yaml(yaml).unwrap(), &registry).unwrap();
        let mut inst = PipelineInstance::new(Arc::new(plan), &registry).unwrap();
        inst.set_base(&UniversalPath::from_segment(PathSegment::new(vec![PathPoint::new(1.0, 0.0, 255, 255, 255, 0)])));
        (inst, registry)
    }

    fn x(path: &UniversalPath) -> f32 {
        path.segments[0].points[0].x
    }

    struct Frame {
        signals: EntitySignals,
        global: Signals,
        out: UniversalPath,
    }

    impl Frame {
        fn new() -> Self {
            Self { signals: EntitySignals::default(), global: Signals::default(), out: UniversalPath::default() }
        }

        fn step(&mut self, inst: &mut PipelineInstance, registry: &PipelineRegistry, dt: f32) -> (bool, Vec<ReactionOutput>) {
            let outputs = inst.tick(dt, &mut self.signals, registry);
            (inst.evaluate(&self.signals, &self.global, 0.0, dt, &mut self.out), outputs)
        }
    }

    #[test]
    fn stages_apply_in_order_without_accumulating() {
        let (mut inst, reg) = instance("name: t\nstages:\n  - shift: { dx: 1 }\n  - mul: { k: 10 }\n  - shift: { dx: { signal: self.hit, range: [0, 5] } }\n");
        let mut f = Frame::new();
        for _ in 0..100 {
            f.signals.set("hit", 0.2);
            f.step(&mut inst, &reg, 0.02);
        }
        // (1 + 1) * 10 + 0.2 * 5, every frame, not growing
        assert_eq!(x(&f.out), 21.0);
    }

    #[test]
    fn static_prefix_runs_once_until_invalidated() {
        STATIC_RUNS.with(|n| n.set(0));
        let (mut inst, reg) = instance("name: t\nstages:\n  - shift: { dx: 3 }\n  - shift: { dx: { signal: self.x } }\n");
        let mut f = Frame::new();
        for _ in 0..10 {
            assert!(f.step(&mut inst, &reg, 0.02).0, "dynamic suffix rewrites the output every frame");
        }
        assert_eq!(STATIC_RUNS.with(Cell::get), 1);

        inst.set_base(&UniversalPath::from_segment(PathSegment::new(vec![PathPoint::new(5.0, 0.0, 255, 255, 255, 0)])));
        f.step(&mut inst, &reg, 0.02);
        assert_eq!(STATIC_RUNS.with(Cell::get), 2);
        assert_eq!(x(&f.out), 8.0);
    }

    #[test]
    fn fully_static_pipeline_reports_no_change() {
        let (mut inst, reg) = instance("name: t\nstages:\n  - mul: { k: 2 }\n");
        let mut f = Frame::new();
        assert!(f.step(&mut inst, &reg, 0.02).0);
        assert!(!f.step(&mut inst, &reg, 0.02).0);
        assert_eq!(x(&f.out), 2.0);
    }

    #[test]
    fn envelope_reaction_drives_bound_param() {
        let (mut inst, reg) = instance(
            "name: t
stages:
  - shift: { dx: { signal: self.hit, range: [0, 10] } }
reactions:
  hit:
    - envelope: { name: hit, attack_ms: 0, hold_ms: 0, decay_ms: 100 }
",
        );
        let mut f = Frame::new();
        f.step(&mut inst, &reg, 0.02);
        assert_eq!(x(&f.out), 1.0);

        inst.trigger("hit", 1.0);
        f.step(&mut inst, &reg, 0.02);
        assert_eq!(x(&f.out), 11.0, "full flash in the trigger's own frame");
        f.step(&mut inst, &reg, 0.05);
        assert!((x(&f.out) - 6.0).abs() < 1e-4, "half way through the decay: {}", x(&f.out));
        f.step(&mut inst, &reg, 0.1);
        assert_eq!(x(&f.out), 1.0);
        assert_eq!(f.signals.get("hit"), Some(0.0));
    }

    #[test]
    fn trigger_strength_scales_envelope_and_modes_combine() {
        let (mut inst, reg) = instance(
            "name: t
reactions:
  a:
    - envelope: { name: e, decay_ms: 1000, mode: add }
",
        );
        let mut f = Frame::new();
        inst.trigger("a", 0.3);
        inst.trigger("a", 0.3);
        f.step(&mut inst, &reg, 0.0);
        assert!((f.signals.get("e").unwrap() - 0.6).abs() < 1e-5);
    }

    #[test]
    fn timed_set_disables_stage_then_restores() {
        let (mut inst, reg) = instance(
            "name: t
stages:
  - mul: { id: m, k: 3 }
reactions:
  hit:
    - set: { stage: m, params: { k: 5 }, for_ms: 100 }
",
        );
        let mut f = Frame::new();
        f.step(&mut inst, &reg, 0.02);
        assert_eq!(x(&f.out), 3.0);
        inst.trigger("hit", 1.0);
        f.step(&mut inst, &reg, 0.02);
        assert_eq!(x(&f.out), 5.0);
        f.step(&mut inst, &reg, 0.1);
        assert_eq!(x(&f.out), 3.0);
    }

    #[test]
    fn sequence_waits_for_finished_then_emits() {
        let (mut inst, reg) = instance(
            "name: t
lifetime_ms: 200
reactions:
  bound:
    - wait: finished
    - emit: { tag: visual_finished, payload: { ok: true } }
",
        );
        let mut f = Frame::new();
        assert!(f.step(&mut inst, &reg, 0.0).1.is_empty());
        assert!(f.step(&mut inst, &reg, 0.1).1.is_empty());
        assert!((f.signals.get("life").unwrap() - 0.5).abs() < 1e-5);
        let (_, outputs) = f.step(&mut inst, &reg, 0.1);
        assert_eq!(outputs, vec![ReactionOutput::Emit { tag: "visual_finished".into(), payload: serde_json::json!({ "ok": true }) }]);
        assert!(inst.is_finished());
    }

    #[test]
    fn trigger_action_chains_reactions() {
        let (mut inst, reg) = instance(
            "name: t
reactions:
  pop:
    - trigger: boom
  boom:
    - emit: boomed
",
        );
        let mut f = Frame::new();
        inst.trigger("pop", 1.0);
        let (_, outputs) = f.step(&mut inst, &reg, 0.0);
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn rebind_keeps_unchanged_stage_state() {
        let registry = registry();
        let v1 = compile(&PipelineSpec::from_yaml("name: t\nstages:\n  - shift: { id: s, dx: { signal: self.x, smooth_ms: 100 } }\n").unwrap(), &registry).unwrap();
        let v2 = compile(&PipelineSpec::from_yaml("name: t\nstages:\n  - shift: { id: s, dx: { signal: self.x, smooth_ms: 100 } }\n  - mul: { k: 1 }\n").unwrap(), &registry).unwrap();
        let mut inst = PipelineInstance::new(Arc::new(v1), &registry).unwrap();
        inst.set_base(&UniversalPath::from_segment(PathSegment::new(vec![PathPoint::new(0.0, 0.0, 255, 255, 255, 0)])));
        let mut f = Frame::new();
        f.signals.set("x", 0.0);
        f.step(&mut inst, &registry, 0.02);
        f.signals.set("x", 1.0);
        f.step(&mut inst, &registry, 0.02);
        let before = x(&f.out);

        inst.rebind(Arc::new(v2), &registry).unwrap();
        f.step(&mut inst, &registry, 0.02);
        // Smoothing continued from its previous value instead of restarting at the target
        assert!(x(&f.out) > before && x(&f.out) < 1.0, "{before} -> {}", x(&f.out));
    }
}
