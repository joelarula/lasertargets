//! Sound composition, built like the laser pipeline: plugins register **sources** (make sound)
//! and **stages** (shape it), and sounds are composed from them in `.sound.yaml` data files whose
//! parameters can follow live signals with the same `Param` syntax as pipeline stages.
//!
//! ```text
//!  .sound.yaml ─▶ SoundSpec ─▶ SoundCatalog (validated against registered nodes)
//!                                   │ voice(name)
//!                                   ▼
//!   layers (in parallel, each at its own time):  source ─▶ stage ─▶ stage ─┐
//!                                                source ─▶ stage ──────────┼─▶ mix stages ─▶ gain
//!                                                sound: <other> (nested) ──┘
//!                                   │
//!                                   ▼
//!  SoundMixer (per output): parallel voices + live stream, ducking, limiter ─▶ output frames
//! ```
//!
//! Composition is output-independent: the speakers (48 kHz) and the galvos (DAC point rate) render
//! the same sound files through their own [`SoundMixer`]. An output only chooses its sample rate
//! and the default [`Route`] for mono sources.
//!
//! | Module | Contents |
//! |---|---|
//! | [`node`] | [`SoundSource`], [`SoundStage`], [`SoundCtx`], [`Route`] — the plugin contract |
//! | [`spec`] | `.sound.yaml` syntax |
//! | [`catalog`] | node registry + sounds by name, builds [`SoundVoice`]s |
//! | [`voice`] | one playing sound: layers, stages, gain, stop fade |
//! | [`mixer`] | parallel voices, stream, ducking, limiter |
//! | [`sources`], [`stages`] | built-in nodes |
//! | [`dsp`] | notes, scales, envelope, oscillator, filters |
//! | [`plugin`] | [`SoundPlugin`], [`SoundAppExt`], asset loading and hot reload |

pub mod catalog;
pub mod dsp;
pub mod mixer;
pub mod node;
pub mod plugin;
pub mod sources;
pub mod spec;
pub mod stages;
pub mod voice;

pub use catalog::SoundCatalog;
pub use mixer::SoundMixer;
pub use node::{Route, SoundCtx, SoundSource, SoundStage};
pub use plugin::{SoundAppExt, SoundPlugin};
pub use spec::{LayerSpec, SoundSpec};
pub use voice::SoundVoice;
