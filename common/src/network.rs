use bevy::asset::uuid::Uuid;
use bincode;
use serde::{Deserialize, Serialize};

use crate::{

    actor::ActorMetaData, config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration}, game::GameSession, scene::SceneSetup
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

    //server
    QueryServerState,
    QueryGameState,
    
    ServerStateUpdate(crate::state::ServerState),
    GameStateUpdate(crate::state::GameState),

    // Projector Configuration
    QueryProjectorConfig,
    ProjectorConfigUpdate(ProjectorConfiguration),
    UpdateProjectorConfig(ProjectorConfiguration),

    // Camera Configuration
    QueryCameraConfig,
    CameraConfigUpdate(CameraConfiguration),
    UpdateCameraConfig(CameraConfiguration),

    // Scene Configuration
    QuerySceneConfig,
    SceneConfigUpdate(SceneConfiguration),
    UpdateSceneConfig(SceneConfiguration),

    // Scene Setup
    QuerySceneSetup,
    SceneSetupUpdate(SceneSetup),

    // Game Configuration
    QueryGameSession,
    GameSessionResponse(GameSession),
    InitGameSession(u16,String),
    StartGameSession(Uuid),
    PauseGameSession(Uuid),
    ResumeGameSession(Uuid),
    StopGameSession(Uuid),
    ReplyGameSession(Uuid),

    // Actor 
    RegisterActor(Uuid,String,Vec<String>),
    UnregisterActor(Uuid,Uuid),
    QueryActor,
    ActorResponse(ActorMetaData),
    ActorError(String),

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
