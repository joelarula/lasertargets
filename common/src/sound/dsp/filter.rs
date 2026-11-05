//! Biquad filters (RBJ cookbook) used by the output guard.

use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    /// Passes everything unchanged
    pub fn identity() -> Self {
        Self::from_coefficients(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }

    fn from_coefficients(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    /// Second-order low-pass (Butterworth Q) at `cutoff_hz`
    pub fn lowpass(cutoff_hz: f32, sample_rate: f32) -> Self {
        let cutoff = cutoff_hz.clamp(10.0, sample_rate * 0.45);
        let w0 = TAU * cutoff / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * std::f32::consts::FRAC_1_SQRT_2);
        Self::from_coefficients((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha)
    }

    /// Peaking cut of `gain_db` (negative) over the band `min_hz..max_hz`
    pub fn band_cut(min_hz: f32, max_hz: f32, gain_db: f32, sample_rate: f32) -> Self {
        if max_hz <= min_hz || min_hz <= 0.0 || gain_db >= 0.0 || max_hz >= sample_rate * 0.5 {
            return Self::identity();
        }
        let center = (min_hz * max_hz).sqrt();
        let q = center / (max_hz - min_hz);
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = TAU * center / sample_rate;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        Self::from_coefficients(1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a)
    }

    /// Takes over another filter's history, so changing coefficients mid-sound doesn't click
    pub fn carry_state(&mut self, other: &Biquad) {
        (self.x1, self.x2, self.y1, self.y2) = (other.x1, other.x2, other.y1, other.y2);
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}
