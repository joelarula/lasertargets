//! Musical pitch: 12-TET notes, intervals, scale quantisation and the loop-length ↔ pitch
//! relation of a repeating scan.

use serde::{Deserialize, Serialize};

/// Pitch classes of 12-tone equal temperament.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Note {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl Note {
    /// Semitones above C (0..=11)
    pub fn semitone(self) -> i32 {
        self as i32
    }

    /// Frequency of this note in `octave`, with A4 = 440 Hz (`Note::A.frequency(4)`)
    pub fn frequency(self, octave: i8) -> f32 {
        let midi = (octave as i32 + 1) * 12 + self.semitone();
        midi_to_frequency(midi.clamp(0, 127) as u8)
    }
}

/// MIDI note number to Hz (69 = A4 = 440 Hz)
pub fn midi_to_frequency(midi_note: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0)
}

/// Just-intonation intervals, used for two-voice chords (X voice = root, Y voice = interval).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    Unison,
    MinorThird,
    MajorThird,
    PerfectFourth,
    PerfectFifth,
    MajorSixth,
    Octave,
}

impl Interval {
    /// Frequency ratio to the root
    pub fn ratio(self) -> f32 {
        match self {
            Interval::Unison => 1.0,
            Interval::MinorThird => 6.0 / 5.0,
            Interval::MajorThird => 5.0 / 4.0,
            Interval::PerfectFourth => 4.0 / 3.0,
            Interval::PerfectFifth => 3.0 / 2.0,
            Interval::MajorSixth => 5.0 / 3.0,
            Interval::Octave => 2.0,
        }
    }
}

/// Snaps a continuous frequency onto a scale (theremin, instruments).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Scale {
    #[default]
    Continuous,
    Chromatic,
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
}

impl Scale {
    fn steps(self) -> Option<&'static [i32]> {
        match self {
            Scale::Continuous | Scale::Chromatic => None,
            Scale::Major => Some(&[0, 2, 4, 5, 7, 9, 11]),
            Scale::Minor => Some(&[0, 2, 3, 5, 7, 8, 10]),
            Scale::MajorPentatonic => Some(&[0, 2, 4, 7, 9]),
            Scale::MinorPentatonic => Some(&[0, 3, 5, 7, 10]),
        }
    }

    /// Nearest frequency on this scale (C-rooted); `Continuous` returns the input
    pub fn quantize(self, hz: f32) -> f32 {
        if self == Scale::Continuous || hz <= 10.0 {
            return hz;
        }
        let midi = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
        let Some(steps) = self.steps() else {
            return midi_to_frequency(midi.clamp(0, 127) as u8);
        };
        let octave = midi.div_euclid(12);
        let within = midi.rem_euclid(12);
        // The next octave's root is a candidate too, so B snaps up to C when that is nearer
        let best = steps
            .iter()
            .copied()
            .chain(std::iter::once(12))
            .min_by_key(|step| (step - within).abs())
            .unwrap_or(0);
        midi_to_frequency((octave * 12 + best).clamp(0, 127) as u8)
    }
}

/// Points a closed loop needs so that scanning it repeatedly at `pps` hums at `pitch_hz`.
///
/// Only meaningful when that loop is the whole DAC frame; the frame composer adds dwell and
/// travel points, so pitch-locking shapes belongs to the frame tier, not to a per-entity stage.
pub fn loop_points_for_pitch(pitch_hz: f32, pps: u32) -> usize {
    if pitch_hz <= 0.0 || pps == 0 {
        return 0;
    }
    ((pps as f32 / pitch_hz).round() as usize).clamp(4, 4096)
}

/// Fundamental of a loop of `points` scanned repeatedly at `pps`
pub fn loop_pitch(points: usize, pps: u32) -> f32 {
    if points == 0 { 0.0 } else { pps as f32 / points as f32 }
}
