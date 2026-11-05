use common::audio::{GalvoWaveform, SoundId};
use std::f32::consts::PI;

/// Attack, Decay, Sustain, Release envelope to eliminate inductive popping in galvo coils.
#[derive(Debug, Clone, Copy)]
pub struct AdsrEnvelope {
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain_level: f32,
    pub release_s: f32,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self {
            attack_s: 0.005,  // 5ms soft ramp prevents loud inductive click
            decay_s: 0.010,   // 10ms decay
            sustain_level: 0.85,
            release_s: 0.008, // 8ms release
        }
    }
}

impl AdsrEnvelope {
    /// Calculate envelope gain [0.0, 1.0] at time `t` given a total tone duration `total_s`.
    pub fn amplitude_at(&self, t: f32, total_s: f32) -> f32 {
        if t < 0.0 || t >= total_s {
            return 0.0;
        }

        // Release phase
        let release_start = (total_s - self.release_s).max(self.attack_s + self.decay_s);
        if t >= release_start {
            let rel_progress = (t - release_start) / (total_s - release_start).max(0.001);
            return (self.sustain_level * (1.0 - rel_progress)).clamp(0.0, 1.0);
        }

        // Attack phase
        if t < self.attack_s {
            return (t / self.attack_s.max(0.0001)).clamp(0.0, 1.0);
        }

        // Decay phase
        let decay_t = t - self.attack_s;
        if decay_t < self.decay_s {
            let dec_progress = decay_t / self.decay_s.max(0.0001);
            return 1.0 - dec_progress * (1.0 - self.sustain_level);
        }

        // Sustain phase
        self.sustain_level
    }
}

/// Synthesize a discrete audio waveform buffer for a given tone.
pub fn generate_tone(
    waveform: GalvoWaveform,
    freq_hz: f32,
    duration_s: f32,
    sample_rate: u32,
    volume: f32,
    envelope: Option<AdsrEnvelope>,
) -> Vec<f32> {
    let total_samples = (duration_s * sample_rate as f32).round() as usize;
    if total_samples == 0 || freq_hz <= 0.0 {
        return Vec::new();
    }

    let env = envelope.unwrap_or_default();
    let dt = 1.0 / sample_rate as f32;
    let mut samples = Vec::with_capacity(total_samples);

    for i in 0..total_samples {
        let t = i as f32 * dt;
        let phase = (2.0 * PI * freq_hz * t) % (2.0 * PI);

        let raw = match waveform {
            GalvoWaveform::Sine => phase.sin(),
            GalvoWaveform::Triangle => {
                // Triangle wave in [-1.0, 1.0]
                2.0 * ((phase / PI) - 1.0).abs() - 1.0
            }
            GalvoWaveform::Square => {
                // Softened square wave to minimize galvo driver stress
                let s = phase.sin();
                (s * 4.0).clamp(-1.0, 1.0)
            }
        };

        let gain = env.amplitude_at(t, duration_s) * volume;
        samples.push((raw * gain).clamp(-1.0, 1.0));
    }

    samples
}

/// Synthesize a frequency chirp (linear sweep from start_hz to end_hz).
pub fn generate_chirp(
    start_hz: f32,
    end_hz: f32,
    duration_s: f32,
    sample_rate: u32,
    volume: f32,
) -> Vec<f32> {
    let total_samples = (duration_s * sample_rate as f32).round() as usize;
    if total_samples == 0 {
        return Vec::new();
    }

    let dt = 1.0 / sample_rate as f32;
    let mut samples = Vec::with_capacity(total_samples);
    let mut phase = 0.0_f32;

    let env = AdsrEnvelope {
        attack_s: 0.002,
        decay_s: 0.005,
        sustain_level: 0.9,
        release_s: 0.005,
    };

    for i in 0..total_samples {
        let t = i as f32 * dt;
        let progress = (i as f32) / (total_samples as f32);
        let current_freq = start_hz + (end_hz - start_hz) * progress;

        phase += 2.0 * PI * current_freq * dt;
        let sample = phase.sin();

        let gain = env.amplitude_at(t, duration_s) * volume;
        samples.push((sample * gain).clamp(-1.0, 1.0));
    }

    samples
}

/// Generate acoustic sound effects mapped directly from `common::audio::SoundId`.
pub fn generate_sound_fx(sound: SoundId, sample_rate: u32, volume: f32) -> Vec<f32> {
    match sound {
        SoundId::LaserHit => {
            // Fast downward chirp confirmation: 2800 Hz -> 1100 Hz in 45ms
            generate_chirp(2800.0, 1100.0, 0.045, sample_rate, volume)
        }
        SoundId::TargetHit => {
            // Crisp high-pitch confirmed blip: 1760 Hz (A6) in 35ms
            generate_tone(GalvoWaveform::Sine, 1760.0, 0.035, sample_rate, volume, None)
        }
        SoundId::TargetSpawn => {
            // Soft two-tone rising pip: 1046 Hz (C6) -> 1318 Hz (E6)
            let mut out = generate_tone(GalvoWaveform::Sine, 1046.0, 0.025, sample_rate, volume * 0.8, None);
            out.extend(generate_tone(GalvoWaveform::Sine, 1318.0, 0.035, sample_rate, volume * 0.9, None));
            out
        }
        SoundId::TargetMiss => {
            // Low sawtooth dissonance buzz: 160 Hz in 80ms
            let env = AdsrEnvelope {
                attack_s: 0.004,
                decay_s: 0.010,
                sustain_level: 0.8,
                release_s: 0.015,
            };
            generate_tone(GalvoWaveform::Triangle, 160.0, 0.080, sample_rate, volume * 0.85, Some(env))
        }
        SoundId::CountdownTick => {
            // High percussive tick: 2200 Hz in 15ms
            let env = AdsrEnvelope {
                attack_s: 0.001,
                decay_s: 0.004,
                sustain_level: 0.4,
                release_s: 0.005,
            };
            generate_tone(GalvoWaveform::Sine, 2200.0, 0.015, sample_rate, volume, Some(env))
        }
        SoundId::CountdownGo => {
            // Ascending triad chime: C5 (523 Hz) -> E5 (659 Hz) -> G5 (784 Hz) -> C6 (1046 Hz)
            let freqs = [523.25, 659.25, 783.99, 1046.50];
            let mut out = Vec::new();
            for f in freqs {
                out.extend(generate_tone(GalvoWaveform::Sine, f, 0.040, sample_rate, volume, None));
            }
            out
        }
        SoundId::GameOver => {
            // Descending tone sequence
            let freqs = [784.0, 659.0, 523.0, 392.0];
            let mut out = Vec::new();
            for f in freqs {
                out.extend(generate_tone(GalvoWaveform::Triangle, f, 0.060, sample_rate, volume, None));
            }
            out
        }
        SoundId::PelletEat => {
            // Quick high chirp: 1400 Hz -> 2000 Hz in 25ms
            generate_chirp(1400.0, 2000.0, 0.025, sample_rate, volume)
        }
        SoundId::SnakeCrash => {
            // Harsh low impact noise
            generate_chirp(400.0, 90.0, 0.120, sample_rate, volume)
        }
        SoundId::CalibrationCornerAcquired => {
            // Gentle high bell confirmation: 2093 Hz (C7)
            generate_tone(GalvoWaveform::Sine, 2093.0, 0.040, sample_rate, volume * 0.7, None)
        }
        SoundId::CalibrationSuccess => {
            // Major triad fanfare: 1046 Hz -> 1318 Hz -> 1567 Hz
            let freqs = [1046.5, 1318.5, 1567.98];
            let mut out = Vec::new();
            for f in freqs {
                out.extend(generate_tone(GalvoWaveform::Sine, f, 0.050, sample_rate, volume, None));
            }
            out
        }
        SoundId::Alert => {
            // Alternating two-tone warble
            let mut out = Vec::new();
            for _ in 0..2 {
                out.extend(generate_tone(GalvoWaveform::Square, 1800.0, 0.040, sample_rate, volume * 0.8, None));
                out.extend(generate_tone(GalvoWaveform::Square, 2400.0, 0.040, sample_rate, volume * 0.8, None));
            }
            out
        }
    }
}

