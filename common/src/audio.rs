use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::settings::ConfigSection;

/// Identifiers for audio sound effects and cues in LaserTargets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SoundId {
    /// Optical laser strike confirmation
    LaserHit,
    /// Minigame target spawned
    TargetSpawn,
    /// Minigame target successfully hit
    TargetHit,
    /// Minigame target timed out / missed
    TargetMiss,
    /// Match countdown ticks (3, 2, 1)
    CountdownTick,
    /// Match start / go sound
    CountdownGo,
    /// Round over / game over
    GameOver,
    /// Snake minigame pellet eaten
    PelletEat,
    /// Snake crash / collision
    SnakeCrash,
    /// Calibration target corner detected
    CalibrationCornerAcquired,
    /// Calibration sequence succeeded
    CalibrationSuccess,
    /// Warning / error tone
    Alert,
}

/// Global audio settings loaded from YAML (`audio:`)
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioConfiguration {
    /// Master enable switch for audio output
    pub enabled: bool,
    /// Master volume scale [0.0, 1.0]
    pub volume: f32,
    /// Enable dynamic stereo panning based on horizontal coordinates [-1.0, 1.0]
    pub spatial_panning: bool,
}

impl Default for AudioConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            spatial_panning: true,
        }
    }
}

impl ConfigSection for AudioConfiguration {
    const KEY: &'static str = "audio";
}

/// Message fired to trigger a one-shot audio sound effect.
#[derive(Message, Debug, Clone)]
pub struct PlaySoundEvent {
    /// Which sound cue to play
    pub sound: SoundId,
    /// Per-sound volume scale [0.0, 1.0]. Defaults to 1.0.
    pub volume: f32,
    /// Optional horizontal coordinate for stereo panning in [-1.0, 1.0].
    /// -1.0 is full Left, 0.0 is Center, 1.0 is full Right.
    pub pan: Option<f32>,
}

impl PlaySoundEvent {
    pub fn new(sound: SoundId) -> Self {
        Self {
            sound,
            volume: 1.0,
            pan: None,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_pan(mut self, pan: f32) -> Self {
        self.pan = Some(pan.clamp(-1.0, 1.0));
        self
    }
}

