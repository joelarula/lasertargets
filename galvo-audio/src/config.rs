//! `audio.galvo` config section.
//!
//! ```yaml
//! audio:
//!   galvo:
//!     enabled: true
//!     master_volume: 0.8
//! ```
//!
//! What the galvos play is composed in `assets/sounds/*.sound.yaml` (`common::sound`), and
//! which sounds reach them is `audio.routing` (`common::audio`); this section only covers the
//! galvo output itself: hardware limits, latency and the stream mix.

use common::sound::Route;
use crate::guard::{GuardSettings, ABSOLUTE_MAX_HZ};
use bevy::prelude::Resource;
use common::settings::ConfigSection;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GalvoAudioConfig {
    /// Master switch. Off by default: galvo audio is extra load on the scanners and is enabled
    /// per rig
    pub enabled: bool,
    /// 0..1
    pub master_volume: f32,
    /// Offset of a full-scale sample in 12-bit DAC units; capped at
    /// [`crate::guard::ABSOLUTE_MAX_EXCURSION_DAC`] whatever is set here
    pub max_excursion_dac: u16,
    /// Route for mono sources that don't name one (`orbit` shares coil heat between X and Y)
    pub route: Route,
    /// How far rendering runs ahead of the DAC. Lower is snappier, higher survives slow ticks
    pub latency_ms: f32,
    /// Keep singing while nothing is lit (between rounds, laser switched off)
    pub play_when_blank: bool,
    /// Low-pass cutoff; capped at [`ABSOLUTE_MAX_HZ`]
    pub ceiling_hz: f32,
    /// Scanner resonance band to attenuate
    pub notch_min_hz: f32,
    pub notch_max_hz: f32,
    /// Attenuation in the resonance band (negative dB)
    pub notch_db: f32,
    /// Seconds of continuous loud drive before the thermal taper starts
    pub thermal_secs: f32,
    /// Gain the taper settles at, 0..1
    pub thermal_floor: f32,
    pub stream: StreamConfig,
}

/// The live audio stream on the galvos (when `audio.routing.stream` includes `galvo`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StreamConfig {
    pub gain: f32,
    /// Stream level while other sounds play (1 = no ducking)
    pub duck: f32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self { gain: 0.8, duck: 0.4 }
    }
}

impl Default for GalvoAudioConfig {
    fn default() -> Self {
        let guard = GuardSettings::default();
        Self {
            enabled: guard.enabled,
            master_volume: guard.master_volume,
            max_excursion_dac: guard.max_excursion_dac,
            route: Route::Orbit,
            latency_ms: 80.0,
            play_when_blank: guard.play_when_blank,
            ceiling_hz: ABSOLUTE_MAX_HZ,
            notch_min_hz: guard.notch_min_hz,
            notch_max_hz: guard.notch_max_hz,
            notch_db: guard.notch_db,
            thermal_secs: guard.thermal_secs,
            thermal_floor: guard.thermal_floor,
            stream: StreamConfig::default(),
        }
    }
}

impl GalvoAudioConfig {
    pub fn guard_settings(&self) -> GuardSettings {
        GuardSettings {
            enabled: self.enabled,
            master_volume: self.master_volume,
            max_excursion_dac: self.max_excursion_dac,
            ceiling_hz: self.ceiling_hz,
            notch_min_hz: self.notch_min_hz,
            notch_max_hz: self.notch_max_hz,
            notch_db: self.notch_db,
            thermal_secs: self.thermal_secs,
            thermal_floor: self.thermal_floor,
            play_when_blank: self.play_when_blank,
        }
    }
}

impl ConfigSection for GalvoAudioConfig {
    const KEY: &'static str = "audio.galvo";
}
