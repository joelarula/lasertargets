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

/// Waveform shape for galvo acoustic audio synthesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GalvoWaveform {
    #[default]
    Sine,
    Triangle,
    Square,
}

/// Axis modulation mode for superimposing acoustic carrier oscillations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum GalvoCarrierMode {
    /// Modulate X galvanometer mirror only
    SingleAxisX,
    /// Modulate Y galvanometer mirror only
    SingleAxisY,
    /// Circular quadrature orbit (X = sin, Y = cos), balancing coil heat across both galvos
    #[default]
    QuadratureXY,
    /// True two-voice duophonic mode (Voice 1 -> Galvo X, Voice 2 -> Galvo Y)
    Duophonic,
}

/// Configuration for physical galvanometer acoustic sound synthesis.
#[derive(Resource, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GalvoAudioConfig {
    /// Master enable for galvo acoustic vibration audio
    pub enabled: bool,
    /// Master volume scale [0.0, 1.0]
    pub master_volume: f32,
    /// Maximum excursion clamp in 12-bit DAC units (0-4095 range, default 40 ~ 1% of scale)
    pub max_amplitude_dac: u16,
    /// Preferred carrier modulation mode
    pub carrier_mode: GalvoCarrierMode,
    /// Default carrier frequency (Hz)
    pub default_carrier_freq: f32,
    /// Enable blanked audio (synthesizing tones when laser diode is blanked RGB=0)
    pub blanked_audio_enabled: bool,
    /// Notch filter lower bound to prevent driving mechanical resonance (Hz)
    pub resonance_notch_min: f32,
    /// Notch filter upper bound to prevent driving mechanical resonance (Hz)
    pub resonance_notch_max: f32,
}

impl Default for GalvoAudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 0.8,
            max_amplitude_dac: 45, // ~1.1% of 4095
            carrier_mode: GalvoCarrierMode::QuadratureXY,
            default_carrier_freq: 1000.0,
            blanked_audio_enabled: true,
            resonance_notch_min: 3200.0,
            resonance_notch_max: 4200.0,
        }
    }
}

impl ConfigSection for GalvoAudioConfig {
    const KEY: &'static str = "audio.galvo";
}

/// Message fired to command the galvo synthesizer to play a specific musical note or tone.
#[derive(Message, Debug, Clone)]
pub struct PlayGalvoToneEvent {
    /// Frequency of tone in Hertz
    pub frequency_hz: f32,
    /// Duration of tone in milliseconds
    pub duration_ms: u32,
    /// Relative volume scale [0.0, 1.0]
    pub volume: f32,
    /// Waveform shape
    pub waveform: GalvoWaveform,
}

impl PlayGalvoToneEvent {
    pub fn new(frequency_hz: f32, duration_ms: u32) -> Self {
        Self {
            frequency_hz,
            duration_ms,
            volume: 1.0,
            waveform: GalvoWaveform::Sine,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_waveform(mut self, waveform: GalvoWaveform) -> Self {
        self.waveform = waveform;
        self
    }
}

/// Message fired to command the galvo synthesizer to play a two-voice duophonic chord or counterpoint.
/// Galvo X reproduces Voice 1 and Galvo Y reproduces Voice 2 simultaneously.
#[derive(Message, Debug, Clone)]
pub struct PlayGalvoChordEvent {
    /// Frequency for Voice 1 (Galvo X) in Hertz
    pub voice_x_hz: f32,
    /// Frequency for Voice 2 (Galvo Y) in Hertz
    pub voice_y_hz: f32,
    /// Duration of chord in milliseconds
    pub duration_ms: u32,
    /// Overall volume scale [0.0, 1.0]
    pub volume: f32,
    /// Waveform for Voice 1 (X)
    pub waveform_x: GalvoWaveform,
    /// Waveform for Voice 2 (Y)
    pub waveform_y: GalvoWaveform,
}

impl PlayGalvoChordEvent {
    pub fn new(voice_x_hz: f32, voice_y_hz: f32, duration_ms: u32) -> Self {
        Self {
            voice_x_hz,
            voice_y_hz,
            duration_ms,
            volume: 1.0,
            waveform_x: GalvoWaveform::Sine,
            waveform_y: GalvoWaveform::Sine,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_waveforms(mut self, x: GalvoWaveform, y: GalvoWaveform) -> Self {
        self.waveform_x = x;
        self.waveform_y = y;
        self
    }
}



