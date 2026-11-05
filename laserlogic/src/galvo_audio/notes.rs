use serde::{Deserialize, Serialize};

/// Standard 12-tone equal temperament (12-TET) musical pitch classes.
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
    /// Semitone offset from C (0 to 11).
    pub fn semitone_offset(self) -> i32 {
        match self {
            Note::C => 0,
            Note::CSharp => 1,
            Note::D => 2,
            Note::DSharp => 3,
            Note::E => 4,
            Note::F => 5,
            Note::FSharp => 6,
            Note::G => 7,
            Note::GSharp => 8,
            Note::A => 9,
            Note::ASharp => 10,
            Note::B => 11,
        }
    }

    /// Calculate frequency in Hertz for a note at given octave (e.g. Note::A, 4 -> 440.0 Hz).
    /// Standard concert pitch A4 = 440 Hz (MIDI 69).
    pub fn frequency(self, octave: i8) -> f32 {
        // MIDI note number: C-1 is 0, C4 is 60, A4 is 69
        let midi = (octave as i32 + 1) * 12 + self.semitone_offset();
        midi_to_frequency(midi.clamp(0, 127) as u8)
    }
}

/// Convert a standard MIDI note number (0..127) to frequency in Hertz.
/// A4 is MIDI note 69 = 440.0 Hz.
pub fn midi_to_frequency(midi_note: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0)
}

/// Musical scale mode for quantizing continuous frequencies (e.g. for Laser Theremin).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ScaleQuantization {
    #[default]
    Continuous,
    Chromatic,
    Major,
    Minor,
    MajorPentatonic,
    MinorPentatonic,
}

impl ScaleQuantization {
    /// Quantize a continuous frequency to the nearest note in this scale.
    pub fn quantize(self, raw_freq_hz: f32) -> f32 {
        if self == ScaleQuantization::Continuous || raw_freq_hz <= 10.0 {
            return raw_freq_hz;
        }

        // Convert raw freq to continuous fractional MIDI note
        let midi_frac = 69.0 + 12.0 * (raw_freq_hz / 440.0).log2();
        let rounded_midi = midi_frac.round() as i32;

        let scale_intervals = match self {
            ScaleQuantization::Continuous => return raw_freq_hz,
            ScaleQuantization::Chromatic => return midi_to_frequency(rounded_midi.clamp(0, 127) as u8),
            ScaleQuantization::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleQuantization::Minor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleQuantization::MajorPentatonic => &[0, 2, 4, 7, 9],
            ScaleQuantization::MinorPentatonic => &[0, 3, 5, 7, 10],
        };

        // Find closest interval in scale
        let octave = rounded_midi.div_euclid(12);
        let note_in_octave = rounded_midi.rem_euclid(12);

        let mut best_diff = i32::MAX;
        let mut best_interval = scale_intervals[0];

        for &interval in scale_intervals {
            let diff = (interval - note_in_octave).abs();
            if diff < best_diff {
                best_diff = diff;
                best_interval = interval;
            }
        }

        let quantized_midi = (octave * 12 + best_interval).clamp(0, 127) as u8;
        midi_to_frequency(quantized_midi)
    }
}

