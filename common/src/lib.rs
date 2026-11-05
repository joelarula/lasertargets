pub use session as actor;
pub mod audio;
pub mod calibration;
pub mod camera;
pub mod config;
pub use session as game;
pub mod network;
pub mod path;
pub mod pipeline;
pub mod scene; 
pub mod session;
pub mod settings;
pub mod state;
pub mod shapes;
pub mod shape_v2;
pub mod vision;


pub mod transport;

pub use audio::{
    AudioConfiguration, GalvoAudioConfig, GalvoCarrierMode, GalvoWaveform, PlayGalvoChordEvent,
    PlayGalvoToneEvent, PlaySoundEvent, SoundId,
};
pub use camera::{CameraStreamType, PhysicalCameraSettings};
pub use curator_core;
pub use session::{Actor, ActorLink, ActorMetaData, Session, SessionActors, SessionCreated, SessionState, SessionUpdate};

#[cfg(test)]
#[path = "../test/scenetest.rs"]
mod scenetest;
