use common::audio::GalvoWaveform;
use super::harmonic::MusicalInterval;
use super::synth::AdsrEnvelope;
use std::f32::consts::PI;

/// A single synthesizer voice assigned to one physical galvanometer (X or Y).
#[derive(Debug, Clone)]
pub struct GalvoVoice {
    pub frequency_hz: f32,
    pub waveform: GalvoWaveform,
    pub volume: f32,
    pub envelope: AdsrEnvelope,
    phase: f32,
}

impl Default for GalvoVoice {
    fn default() -> Self {
        Self {
            frequency_hz: 440.0,
            waveform: GalvoWaveform::Sine,
            volume: 1.0,
            envelope: AdsrEnvelope::default(),
            phase: 0.0,
        }
    }
}

impl GalvoVoice {
    pub fn new(freq: f32, waveform: GalvoWaveform) -> Self {
        Self {
            frequency_hz: freq,
            waveform,
            volume: 1.0,
            envelope: AdsrEnvelope::default(),
            phase: 0.0,
        }
    }

    /// Reset voice phase accumulator.
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    /// Advance one sample step at sample rate `sample_rate`.
    pub fn next_sample(&mut self, dt: f32, elapsed_s: f32, total_duration_s: f32) -> f32 {
        self.phase = (self.phase + 2.0 * PI * self.frequency_hz * dt) % (2.0 * PI);

        let raw = match self.waveform {
            GalvoWaveform::Sine => self.phase.sin(),
            GalvoWaveform::Triangle => 2.0 * ((self.phase / PI) - 1.0).abs() - 1.0,
            GalvoWaveform::Square => {
                let s = self.phase.sin();
                (s * 4.0).clamp(-1.0, 1.0)
            }
        };

        let gain = self.envelope.amplitude_at(elapsed_s, total_duration_s) * self.volume;
        (raw * gain).clamp(-1.0, 1.0)
    }
}

/// Sound effect types for duophonic two-voice galvo counterpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuoCounterpointSfx {
    /// Sci-fi dual chirp: Galvo X sweeps upward (800 -> 2400 Hz) while Galvo Y sweeps downward (2400 -> 800 Hz)
    SciFiLaserLock,
    /// Resonant power chord blip (Root + Perfect 5th in a 2:3 frequency ratio)
    PowerChordHit,
    /// Microtonal acoustic beat pulsation: 440 Hz on X and 447 Hz on Y, producing a 7 Hz physical throbbing drone
    DissonantBeatWarning,
    /// Dual-voice victory fanfare
    MajorThirdChime,
}

/// Generates synchronized two-voice duophonic audio sample pairs for Galvo X and Galvo Y.
pub struct DuophonicSynth;

impl DuophonicSynth {
    /// Synthesize a two-voice chord (Voice 1 -> Galvo X, Voice 2 -> Galvo Y).
    pub fn generate_chord(
        voice_x_hz: f32,
        waveform_x: GalvoWaveform,
        voice_y_hz: f32,
        waveform_y: GalvoWaveform,
        duration_s: f32,
        sample_rate: u32,
        volume: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let total_samples = (duration_s * sample_rate as f32).round() as usize;
        if total_samples == 0 {
            return (Vec::new(), Vec::new());
        }

        let mut voice_x = GalvoVoice::new(voice_x_hz, waveform_x);
        voice_x.volume = volume;

        let mut voice_y = GalvoVoice::new(voice_y_hz, waveform_y);
        voice_y.volume = volume;

        let dt = 1.0 / sample_rate as f32;
        let mut samples_x = Vec::with_capacity(total_samples);
        let mut samples_y = Vec::with_capacity(total_samples);

        for i in 0..total_samples {
            let t = i as f32 * dt;
            samples_x.push(voice_x.next_sample(dt, t, duration_s));
            samples_y.push(voice_y.next_sample(dt, t, duration_s));
        }

        (samples_x, samples_y)
    }

    /// Synthesize a musical interval chord based on root frequency and `MusicalInterval`.
    pub fn generate_interval_chord(
        root_hz: f32,
        interval: MusicalInterval,
        duration_s: f32,
        sample_rate: u32,
        volume: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let voice_x_hz = root_hz;
        let voice_y_hz = root_hz * interval.frequency_ratio();
        Self::generate_chord(
            voice_x_hz,
            GalvoWaveform::Sine,
            voice_y_hz,
            GalvoWaveform::Sine,
            duration_s,
            sample_rate,
            volume,
        )
    }

    /// Synthesize specialized two-voice counterpoint sound effects.
    pub fn generate_counterpoint_sfx(
        sfx: DuoCounterpointSfx,
        sample_rate: u32,
        volume: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let dt = 1.0 / sample_rate as f32;

        match sfx {
            DuoCounterpointSfx::SciFiLaserLock => {
                // 60ms cross-frequency sweep: X sweeps up (800 -> 2400 Hz), Y sweeps down (2400 -> 800 Hz)
                let duration_s = 0.060;
                let total_samples = (duration_s * sample_rate as f32).round() as usize;
                let mut samples_x = Vec::with_capacity(total_samples);
                let mut samples_y = Vec::with_capacity(total_samples);

                let mut phase_x = 0.0_f32;
                let mut phase_y = 0.0_f32;
                let env = AdsrEnvelope {
                    attack_s: 0.003,
                    decay_s: 0.010,
                    sustain_level: 0.85,
                    release_s: 0.008,
                };

                for i in 0..total_samples {
                    let t = i as f32 * dt;
                    let prog = i as f32 / total_samples as f32;
                    let fx = 800.0 + (2400.0 - 800.0) * prog;
                    let fy = 2400.0 - (2400.0 - 800.0) * prog;

                    phase_x += 2.0 * PI * fx * dt;
                    phase_y += 2.0 * PI * fy * dt;

                    let gain = env.amplitude_at(t, duration_s) * volume;
                    samples_x.push((phase_x.sin() * gain).clamp(-1.0, 1.0));
                    samples_y.push((phase_y.sin() * gain).clamp(-1.0, 1.0));
                }

                (samples_x, samples_y)
            }
            DuoCounterpointSfx::PowerChordHit => {
                // 50ms Root (440 Hz) + Fifth (660 Hz)
                Self::generate_interval_chord(440.0, MusicalInterval::PerfectFifth, 0.050, sample_rate, volume)
            }
            DuoCounterpointSfx::MajorThirdChime => {
                // 70ms Root (523.25 Hz C5) + Major 3rd (654 Hz E5)
                Self::generate_interval_chord(523.25, MusicalInterval::MajorThird, 0.070, sample_rate, volume)
            }
            DuoCounterpointSfx::DissonantBeatWarning => {
                // 100ms microtonal interference: X = 440 Hz, Y = 447 Hz -> 7 Hz acoustic beat pulsation
                Self::generate_chord(440.0, GalvoWaveform::Triangle, 447.0, GalvoWaveform::Triangle, 0.100, sample_rate, volume)
            }
        }
    }
}

