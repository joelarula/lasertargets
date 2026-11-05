use common::audio::GalvoWaveform;
use crate::LaserPoint;
use super::notes::ScaleQuantization;
use super::synth::generate_tone;
use std::f32::consts::PI;

/// Interactive Laser Theremin state.
/// Translates 2D input (gamepad thumbstick, mouse, or tracked wand) into simultaneous acoustic
/// sound and projected visual feedback.
#[derive(Debug, Clone)]
pub struct LaserTheremin {
    /// Normalized horizontal input [-1.0, 1.0] -> Controls pitch
    pub input_x: f32,
    /// Normalized vertical input [-1.0, 1.0] -> Controls volume / carrier amplitude
    pub input_y: f32,
    /// Minimum frequency at x = -1.0 (Hz)
    pub min_freq_hz: f32,
    /// Maximum frequency at x = +1.0 (Hz)
    pub max_freq_hz: f32,
    /// Scale quantization mode (Continuous, Chromatic, Pentatonic, etc.)
    pub quantization: ScaleQuantization,
    /// Waveform timbre
    pub waveform: GalvoWaveform,
    /// Color of projected theremin indicator
    pub color_rgb: (u8, u8, u8),
    /// Phase accumulator for visual Lissajous ripples
    visual_phase: f32,
}

impl Default for LaserTheremin {
    fn default() -> Self {
        Self {
            input_x: 0.0,
            input_y: 0.0,
            min_freq_hz: 130.81, // C3
            max_freq_hz: 2093.0, // C7
            quantization: ScaleQuantization::MajorPentatonic,
            waveform: GalvoWaveform::Sine,
            color_rgb: (0, 255, 180), // Cyan/Teal
            visual_phase: 0.0,
        }
    }
}

impl LaserTheremin {
    /// Update normalized input positions [-1.0, 1.0].
    pub fn set_input(&mut self, x: f32, y: f32) {
        self.input_x = x.clamp(-1.0, 1.0);
        self.input_y = y.clamp(-1.0, 1.0);
    }

    /// Calculate the current acoustic frequency based on horizontal input and scale quantization.
    pub fn current_frequency(&self) -> f32 {
        let norm_x = (self.input_x + 1.0) * 0.5; // [0.0, 1.0]
        // Exponential pitch mapping feels natural to human hearing
        let log_min = self.min_freq_hz.ln();
        let log_max = self.max_freq_hz.ln();
        let raw_freq = (log_min + norm_x * (log_max - log_min)).exp();

        self.quantization.quantize(raw_freq)
    }

    /// Calculate the current volume scale [0.0, 1.0] based on vertical input.
    pub fn current_volume(&self) -> f32 {
        ((self.input_y + 1.0) * 0.5).clamp(0.0, 1.0)
    }

    /// Generates audio waveform samples for the current frame step.
    pub fn generate_audio_samples(&self, duration_s: f32, sample_rate: u32) -> Vec<f32> {
        let freq = self.current_frequency();
        let vol = self.current_volume();
        generate_tone(self.waveform, freq, duration_s, sample_rate, vol, None)
    }

    /// Generates visual laser points for the theremin cursor and acoustic ripples in 12-bit DAC coordinates.
    pub fn generate_visual_points(&mut self, point_count: usize) -> Vec<LaserPoint> {
        let mut points = Vec::with_capacity(point_count);
        let center_x = ((self.input_x + 1.0) * 0.5 * 3600.0 + 248.0) as u16;
        let center_y = ((self.input_y + 1.0) * 0.5 * 3600.0 + 248.0) as u16;

        let (r, g, b) = self.color_rgb;
        let vol = self.current_volume();
        let base_radius = 60.0 + vol * 160.0;

        self.visual_phase = (self.visual_phase + 0.08) % (2.0 * PI);

        // Draw an acoustic ripple ring expanding with volume and frequency
        let ripple_points = (point_count * 3 / 4).max(12);
        for i in 0..ripple_points {
            let theta = (i as f32 / ripple_points as f32) * 2.0 * PI;
            let wobble = (theta * 4.0 + self.visual_phase).sin() * (vol * 15.0);
            let radius = base_radius + wobble;

            let x = (center_x as f32 + radius * theta.cos()).clamp(0.0, 4095.0) as u16;
            let y = (center_y as f32 + radius * theta.sin()).clamp(0.0, 4095.0) as u16;
            points.push(LaserPoint::new(x, y, r, g, b, 255));
        }

        // Center crosshair pip
        points.push(LaserPoint::new(center_x, center_y, 255, 255, 255, 255));

        points
    }
}

