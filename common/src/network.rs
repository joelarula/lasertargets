use bevy::asset::uuid::Uuid;
use bincode;
use serde::{Deserialize, Serialize};

use crate::{

    actor::ActorMetaData, config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration}, game::GameSession, path::UniversalPath, scene::SceneSetup, state::GameState
};

/// Message wrapper from a client containing the client ID and the NetworkMessage
#[derive(bevy::prelude::Message, Debug, Clone, Serialize, Deserialize)]
pub struct FromClientMessage {
    pub client_id: u64,
    pub message: NetworkMessage,
}

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
    QueryCalibrationState,
    
    ServerStateUpdate(crate::state::ServerState),
    GameStateUpdate(crate::state::GameState),
    CalibrationStateUpdate(crate::state::CalibrationState),
    UpdateCalibrationState(crate::state::CalibrationState),

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
    GameSessionCreated(GameSession),
    GameSessionUpdate(GameSession),
    ExitGameSession(Uuid),
    /// Initialize a new game session with session_id, game_id, and initial GameState
    InitGameSession(Uuid, u16, GameState),
    StartGameSession(Uuid),
    PauseGameSession(Uuid),
    ResumeGameSession(Uuid),
    FinishGameSession(Uuid),
    ReplyGameSession(Uuid),

    // Actor 
    RegisterActor(Uuid,String,Vec<String>),
    UnregisterActor(Uuid,Uuid),
    QueryActor,
    ActorResponse(ActorMetaData),
    ActorError(String),

    // Mouse Position
    UpdateMousePosition(Option<bevy::prelude::Vec3>),

    // Mouse Events
    MouseButtonInput {
        button: String,
        pressed: bool,
        position: Option<bevy::prelude::Vec3>,
    },

    // Keyboard Input
    KeyboardInput {
        key: String,
        pressed: bool,
    },

    // Abstract Minimal Path Scene Stream
    BroadcastScenePaths(Vec<crate::path::AbstractPathData>),

    /// Generic minigame payload broadcast (game_id, session_id, event_tag, payload_json)
    GameDataPayload {
        game_id: u16,
        session_id: Uuid,
        event_tag: String,
        payload_json: String,
    },

    // Lifecycle
    /// Broadcast server instance ID on connection/restart
    ServerInfo {
        instance_id: Uuid,
    },
    /// Command to shutdown the server
    ShutdownServer,
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
