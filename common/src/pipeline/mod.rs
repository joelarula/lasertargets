//! Reactive laser pipeline: pipeline specs, compilation, stages, signals and the per-entity
//! evaluator, plus the Bevy plugin that binds them to entities ([`plugin`]). Everything except
//! [`plugin`] is plain Rust, so the server, `dac-test` and tests share one implementation.
//! See `docs/TRANSFORMER_PIPELINE.md`.
//!
//! Flow: YAML → [`PipelineSpec`] (parse) → [`CompiledPipeline`] (resolve and validate against the
//! [`PipelineRegistry`]) → [`PipelineInstance`] (per-entity state) → `UniversalPath` every frame.

pub mod compile;
pub mod instance;
pub mod param;
pub mod plugin;
pub mod registry;
pub mod signals;
pub mod sources;
pub mod spec;
pub mod stages;
pub mod stage;

pub use compile::{compile, CompiledPipeline, SpecError, StageDef};
pub use instance::{PipelineInstance, ReactionOutput};
pub use param::{Binding, Param};
pub use registry::PipelineRegistry;
pub use signals::{EntitySignals, SignalView, Signals};
pub use spec::{Clock, EmitSpec, EnvelopeMode, EnvelopeSpec, LocalSignal, PipelineSpec, ReactionAction, SetSpec};
pub use stage::{PathSource, PathStage, StageCtx, StageKind};
