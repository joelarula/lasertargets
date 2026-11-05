//! Attack/decay/sustain/release envelope. Galvo coils are inductive: a step in drive current
//! clicks loudly, so every tone starts and ends through an envelope.

use serde::{Deserialize, Serialize};

/// ADSR in milliseconds, as written in cue files: `env: { attack_ms: 2, release_ms: 5 }`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Envelope {
    pub attack_ms: f32,
    pub decay_ms: f32,
    /// Level held after the decay, 0..1
    pub sustain: f32,
    pub release_ms: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self { attack_ms: 5.0, decay_ms: 10.0, sustain: 0.85, release_ms: 8.0 }
    }
}

impl Envelope {
    /// Gain 0..1 at `t_ms` into a tone lasting `total_ms`; 0 outside the tone
    pub fn gain(&self, t_ms: f32, total_ms: f32) -> f32 {
        if t_ms < 0.0 || t_ms >= total_ms {
            return 0.0;
        }
        // Tones shorter than attack + release squeeze both proportionally
        let release = self.release_ms.max(0.01);
        let fit = (total_ms / (self.attack_ms.max(0.01) + release)).min(1.0);
        let attack = self.attack_ms.max(0.01) * fit;
        let release = release * fit;
        let decay = self.decay_ms.max(0.01);
        // Level without release: attack ramp, decay to sustain, hold
        let held = |t: f32| -> f32 {
            if t < attack {
                t / attack
            } else if t < attack + decay {
                1.0 - (t - attack) / decay * (1.0 - self.sustain)
            } else {
                self.sustain
            }
        };
        // The release ends exactly at the end of the tone
        let release_start = total_ms - release;
        if t_ms < release_start {
            return held(t_ms).clamp(0.0, 1.0);
        }
        let progress = (t_ms - release_start) / (total_ms - release_start).max(0.01);
        (held(release_start) * (1.0 - progress)).clamp(0.0, 1.0)
    }
}
