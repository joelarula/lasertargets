//! Phase-continuous oscillator. The phase is carried across render calls and frequency
//! changes, so sweeps and instruments never jump (a jump is an audible click on a galvo).

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Waveform {
    #[default]
    Sine,
    Triangle,
    /// Soft-clipped sine; a hard square stresses the drivers
    Square,
    /// Rounded sawtooth for buzzes
    Saw,
}

impl Waveform {
    /// Value -1..1 at `phase` (0..1 of a cycle)
    pub fn at(self, phase: f32) -> f32 {
        match self {
            Waveform::Sine => (phase * TAU).sin(),
            Waveform::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            Waveform::Square => ((phase * TAU).sin() * 4.0).clamp(-1.0, 1.0),
            // Sum of the first four harmonics: a saw without the razor edge
            Waveform::Saw => {
                let mut v = 0.0;
                for k in 1..=4 {
                    v += (phase * TAU * k as f32).sin() / k as f32;
                }
                v * 0.6
            }
        }
    }
}

/// Phase accumulator, phase in cycles (0..1).
#[derive(Debug, Clone, Copy, Default)]
pub struct Oscillator {
    pub phase: f32,
}

impl Oscillator {
    /// Advances by one sample at `hz` and returns the waveform value before the step
    pub fn next(&mut self, waveform: Waveform, hz: f32, sample_rate: f32) -> f32 {
        let value = waveform.at(self.phase);
        self.phase = (self.phase + hz / sample_rate).fract();
        value
    }

    /// Value of the same oscillator a quarter cycle ahead (cosine of a sine), for orbits
    pub fn quadrature(&self, waveform: Waveform) -> f32 {
        waveform.at((self.phase + 0.25).fract())
    }
}
