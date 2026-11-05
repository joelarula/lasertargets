use common::audio::{GalvoAudioConfig, GalvoCarrierMode};
use crate::LaserPoint;
use crate::helios::HeliosPoint;
use std::collections::VecDeque;
use std::f32::consts::PI;

/// Maximum permissible DAC deflection units (out of 4095) for acoustic carrier vibration.
/// Hard safety limit to protect physical galvo coils and driver amplifiers from overheating.
pub const ABSOLUTE_MAX_EXCURSION_DAC: u16 = 80; // ~1.95% of full 12-bit range

/// Manages carrier injection and audio sample streaming onto physical galvanometer point streams.
/// Supports single-axis, quadrature orbit, and true two-voice duophonic modulation (X voice + Y voice).
#[derive(Debug, Clone)]
pub struct GalvoCarrierModulator {
    pub config: GalvoAudioConfig,
    phase: f32,
    sample_queue_x: VecDeque<f32>,
    sample_queue_y: VecDeque<f32>,
}

impl Default for GalvoCarrierModulator {
    fn default() -> Self {
        Self {
            config: GalvoAudioConfig::default(),
            phase: 0.0,
            sample_queue_x: VecDeque::new(),
            sample_queue_y: VecDeque::new(),
        }
    }
}

impl GalvoCarrierModulator {
    pub fn new(config: GalvoAudioConfig) -> Self {
        Self {
            config,
            phase: 0.0,
            sample_queue_x: VecDeque::new(),
            sample_queue_y: VecDeque::new(),
        }
    }

    /// Queue audio samples for Voice 1 (X galvanometer, or mono audio).
    pub fn queue_samples<I: IntoIterator<Item = f32>>(&mut self, samples: I) {
        self.sample_queue_x.extend(samples);
    }

    /// Queue audio samples for Voice 2 (Y galvanometer).
    pub fn queue_samples_y<I: IntoIterator<Item = f32>>(&mut self, samples: I) {
        self.sample_queue_y.extend(samples);
    }

    /// Queue synchronized two-voice duophonic audio samples for both Galvo X and Galvo Y.
    pub fn queue_duo_samples<Ix, Iy>(&mut self, samples_x: Ix, samples_y: Iy)
    where
        Ix: IntoIterator<Item = f32>,
        Iy: IntoIterator<Item = f32>,
    {
        self.sample_queue_x.extend(samples_x);
        self.sample_queue_y.extend(samples_y);
    }

    /// Clear all pending queued audio samples for both voices.
    pub fn clear_queue(&mut self) {
        self.sample_queue_x.clear();
        self.sample_queue_y.clear();
    }

    /// Number of queued audio samples waiting to be played.
    pub fn pending_samples(&self) -> usize {
        self.sample_queue_x.len().max(self.sample_queue_y.len())
    }

    /// Calculates safe clamped excursion amplitude in DAC units.
    fn clamped_amplitude(&self) -> f32 {
        let configured = self.config.max_amplitude_dac as f32 * self.config.master_volume;
        configured.clamp(0.0, ABSOLUTE_MAX_EXCURSION_DAC as f32)
    }

    /// Applies resonance notch attenuation if the given frequency is within the galvo's resonance band.
    pub fn resonance_attenuation(&self, freq_hz: f32) -> f32 {
        if freq_hz >= self.config.resonance_notch_min && freq_hz <= self.config.resonance_notch_max {
            // Attenuate by 70% to avoid rattling the physical scanner rotor on resonance
            0.30
        } else {
            1.0
        }
    }

    /// Get the next audio displacement delta (dx, dy) in DAC coordinates.
    fn next_displacement(&mut self, sample_rate: u32) -> (i32, i32) {
        if !self.config.enabled {
            return (0, 0);
        }

        let max_amp = self.clamped_amplitude();
        if max_amp <= 0.001 {
            return (0, 0);
        }

        match self.config.carrier_mode {
            GalvoCarrierMode::Duophonic => {
                // True two-voice polyphony: X voice drives Galvo X, Y voice drives Galvo Y
                let sample_x = self.sample_queue_x.pop_front().unwrap_or(0.0);
                let sample_y = self.sample_queue_y.pop_front().unwrap_or(0.0);

                let dx = (sample_x * max_amp).round() as i32;
                let dy = (sample_y * max_amp).round() as i32;
                (dx, dy)
            }
            GalvoCarrierMode::SingleAxisX => {
                let sample_val = self.pop_or_default_sample(sample_rate);
                let disp = (sample_val * max_amp).round() as i32;
                (disp, 0)
            }
            GalvoCarrierMode::SingleAxisY => {
                let sample_val = self.pop_or_default_sample(sample_rate);
                let disp = (sample_val * max_amp).round() as i32;
                (0, disp)
            }
            GalvoCarrierMode::QuadratureXY => {
                let sample_val = self.pop_or_default_sample(sample_rate);
                let disp_x = (sample_val * max_amp).round() as i32;

                // In quadrature mode, use Hilbert-shifted circular orbit to distribute heat
                let cos_sample = if let Some(s) = self.sample_queue_x.front() {
                    *s
                } else {
                    self.phase.cos()
                };
                let disp_y = (cos_sample * max_amp).round() as i32;
                (disp_x, disp_y)
            }
        }
    }

    /// Helper to get next sample or generate default carrier sine
    fn pop_or_default_sample(&mut self, sample_rate: u32) -> f32 {
        if let Some(s) = self.sample_queue_x.pop_front() {
            s
        } else if self.config.default_carrier_freq > 10.0 {
            let dt = 1.0 / sample_rate as f32;
            self.phase = (self.phase + 2.0 * PI * self.config.default_carrier_freq * dt) % (2.0 * PI);
            let atten = self.resonance_attenuation(self.config.default_carrier_freq);
            self.phase.sin() * atten
        } else {
            0.0
        }
    }

    /// Superimpose audio carrier oscillation directly onto a buffer of `LaserPoint`s.
    pub fn modulate_laser_points(&mut self, points: &mut [LaserPoint], sample_rate: u32) {
        if !self.config.enabled || points.is_empty() {
            return;
        }

        for pt in points.iter_mut() {
            let (dx, dy) = self.next_displacement(sample_rate);
            let nx = (pt.x as i32 + dx).clamp(0, 4095) as u16;
            let ny = (pt.y as i32 + dy).clamp(0, 4095) as u16;
            pt.x = nx;
            pt.y = ny;
        }
    }

    /// Superimpose audio carrier oscillation directly onto a buffer of `HeliosPoint`s.
    pub fn modulate_helios_points(&mut self, points: &mut [HeliosPoint], sample_rate: u32) {
        if !self.config.enabled || points.is_empty() {
            return;
        }

        for pt in points.iter_mut() {
            let (dx, dy) = self.next_displacement(sample_rate);
            let nx = (pt.x as i32 + dx).clamp(0, 4095) as u16;
            let ny = (pt.y as i32 + dy).clamp(0, 4095) as u16;
            pt.x = nx;
            pt.y = ny;
        }
    }

    /// Generates a blanked frame ($R=G=B=0$) that vibrates the galvos to play audio without projecting light.
    /// In Duophonic mode, plays both Voice X and Voice Y acoustically.
    pub fn generate_blanked_audio_frame(
        &mut self,
        point_count: usize,
        center_x: u16,
        center_y: u16,
        sample_rate: u32,
    ) -> Vec<LaserPoint> {
        if !self.config.enabled || !self.config.blanked_audio_enabled || point_count == 0 {
            return Vec::new();
        }

        let mut points = Vec::with_capacity(point_count);
        for _ in 0..point_count {
            let (dx, dy) = self.next_displacement(sample_rate);
            let x = (center_x as i32 + dx).clamp(0, 4095) as u16;
            let y = (center_y as i32 + dy).clamp(0, 4095) as u16;
            points.push(LaserPoint::blanked(x, y));
        }

        points
    }
}
