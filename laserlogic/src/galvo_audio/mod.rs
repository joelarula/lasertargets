pub mod carrier;
pub mod duo;
pub mod harmonic;
pub mod notes;
pub mod synth;
pub mod theremin;

pub use carrier::{GalvoCarrierModulator, ABSOLUTE_MAX_EXCURSION_DAC};
pub use duo::{DuoCounterpointSfx, DuophonicSynth, GalvoVoice};
pub use harmonic::{
    generate_harmonic_circle, pitch_for_points, points_for_pitch, resample_closed_loop,
    MusicalInterval,
};
pub use notes::{midi_to_frequency, Note, ScaleQuantization};
pub use synth::{generate_chirp, generate_sound_fx, generate_tone, AdsrEnvelope};
pub use theremin::LaserTheremin;

