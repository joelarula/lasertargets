//! `.sound.yaml` syntax: the parsed, not yet validated form of a composed sound.
//!
//! ```yaml
//! name: countdown_go
//! description: Rising arpeggio C5 E5 G5 C6
//! layers:                                   # play in parallel, each from its own start time
//!   - at_ms: 0
//!     source: { tone: { hz: 523.25, ms: 40 } }
//!     stages:
//!       - envelope: { attack_ms: 5, release_ms: 8 }
//!   - at_ms: 40
//!     source: { tone: { hz: 659.25, ms: 40 } }
//!   - at_ms: 120
//!     source: { sound: target_hit }         # another composed sound, nested
//! stages:                                   # applied to the sum of the layers
//!   - echo: { ms: 60, feedback: 0.3 }
//! gain: { signal: game.intensity, range: [0.6, 1.0] }
//! ```

use crate::pipeline::spec::Keyed;
use crate::pipeline::Param;
use serde::{Deserialize, Serialize};

fn one() -> Param {
    Param::Const(1.0)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundSpec {
    /// Equals the file path under `assets/sounds` without `.sound.yaml`, and the sound name used
    /// by routing (`laser_hit`)
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub layers: Vec<LayerSpec>,
    /// Stages over the mix of all layers
    #[serde(default)]
    pub stages: Vec<Keyed>,
    /// Overall level, 0..1
    #[serde(default = "one")]
    pub gain: Param,
}

/// One source with its own stages, starting `at_ms` into the sound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerSpec {
    #[serde(default)]
    pub at_ms: f32,
    /// Extra copies of the layer, `every_ms` apart (warbles, rolls)
    #[serde(default)]
    pub repeat: u32,
    #[serde(default)]
    pub every_ms: f32,
    /// A registered source (`tone`, `chirp`, `noise` …) or `sound: <name>` for a nested sound
    pub source: Keyed,
    #[serde(default)]
    pub stages: Vec<Keyed>,
}

impl SoundSpec {
    pub fn from_yaml(text: &str) -> Result<Self, String> {
        use figment::providers::{Format, Yaml};
        figment::Figment::from(Yaml::string(text)).extract().map_err(|e| e.to_string())
    }
}
