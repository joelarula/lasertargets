use bincode;
use serde::{Deserialize, Serialize};

use crate::{
    actor::ActorMetaData,
    config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration}, game::Game, scene::SceneSetup,
};

/// Network messages exchanged between server and terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Simple ping message from server
    Ping {
        timestamp: u64,
    },
    /// Pong response from client
    Pong {
        timestamp: u64,
    },

    // Projector Configuration
    QueryProjectorConfig,
    ProjectorConfigResponse(ProjectorConfiguration),
    UpdateProjectorConfig(ProjectorConfiguration),

    // Camera Configuration
    QueryCameraConfig,
    CameraConfigResponse(CameraConfiguration),
    UpdateCameraConfig(CameraConfiguration),

    // Scene Configuration
    QuerySceneConfig,
    SceneConfigResponse(SceneConfiguration),
    UpdateSceneConfig(SceneConfiguration),

    // Scene Setup
    QuerySceneSetup,
    SceneSetupResponse(SceneSetup),

    // Actor Configuration
    QueryActor,
    ActorResponse(ActorMetaData),
    RegisterActor(ActorMetaData),
    UnregisterActor(ActorMetaData),

    // Game Configuration
    QueryGameTag,
    GameTagResponse(Game),
}

impl NetworkMessage {
    /// Serialize the message to bytes using bincode
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize bytes to a NetworkMessage using bincode
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Server configuration
pub const SERVER_PORT: u16 = 6000;
pub const SERVER_HOST: &str = "0.0.0.0";
