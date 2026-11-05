//! Plain signal-processing building blocks. No Bevy, no I/O.

pub mod envelope;
pub mod filter;
pub mod notes;
pub mod osc;

pub use envelope::Envelope;
pub use filter::Biquad;
pub use notes::{loop_pitch, loop_points_for_pitch, midi_to_frequency, Interval, Note, Scale};
pub use osc::{Oscillator, Waveform};
