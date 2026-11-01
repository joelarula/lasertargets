pub use session as actor;
pub mod camera;
pub mod config;
pub use session as game;
pub mod network;
pub mod path;
pub mod scene; 
pub mod session;
pub mod state;
pub mod shapes;


pub mod transport;

pub use camera::{CameraStreamType, PhysicalCameraSettings};
pub use curator_core;
pub use session::{Actor, ActorLink, ActorMetaData, Session, SessionActors, SessionCreated, SessionState, SessionUpdate};

#[cfg(test)]
#[path = "../test/scenetest.rs"]
mod scenetest;
