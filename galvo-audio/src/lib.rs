//! Galvo audio ("singing galvos"): the scanner's voice-coil motors play sound when small audio
//! offsets are added to the DAC points. See `docs/GALVO_AUDIO_PLAN.md`.
//!
//! This crate is one audio **output**, next to the speakers. It doesn't define sounds:
//! compositions live in `assets/sounds/*.sound.yaml` and are built from plugin-registered sources
//! and stages (`common::sound`), and `audio.routing` (`common::audio`) decides which sounds
//! reach the galvos. This crate renders them at the DAC point rate and keeps the hardware safe.
//!
//! ```text
//!  RoutedSound (galvo) ──▶ SoundMixer ──────▶ BUS ──────▶ GUARD ──────▶ + DAC point x/y
//!  LiveAudioFrame         (common::sound,    Bevy → DAC  ceiling,       (DAC thread, one
//!   (when routed)          at the DAC rate,  thread      notch, thermal, sample per point)
//!                          default orbit)                 clamp
//!                               │
//!                               ▼
//!          galvo.* signals, AudioScope → BasePath (visuals react to the sound)
//! ```
//!
//! | Module | Bevy? | Contents |
//! |---|---|---|
//! | [`guard`] | no | hard limits, applied per DAC point |
//! | [`bus`] | resource only | Bevy → DAC thread hand-off, [`GalvoAudioOutput`] |
//! | [`stream`], [`scope`] | no | windows and levels; scope geometry in metres |
//! | [`config`] | resource only | `audio.galvo` section |
//! | [`plugin`] | yes | [`GalvoAudioPlugin`], signals, [`AudioScope`] |
//!
//! The crate knows nothing about the DAC's point type: the projector asks the output for one
//! `(dx, dy)` per point and adds it.

pub mod bus;
pub mod config;
pub mod guard;
pub mod plugin;
pub mod scope;
pub mod stream;

pub use bus::{apply_offset, GalvoAudioBus, GalvoAudioOutput};
pub use config::{GalvoAudioConfig, StreamConfig};
pub use guard::{GuardSettings, ABSOLUTE_MAX_EXCURSION_DAC, ABSOLUTE_MAX_HZ};
pub use plugin::{AudioScope, GalvoAudioPlugin, GalvoAudioSet, GalvoSynth};
pub use scope::{ScopeInput, ScopeMode};
