use bevy::asset::uuid::Uuid;
use bincode;
use serde::{Deserialize, Serialize};

use crate::{
    config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration}, session::Session, scene::SceneSetup
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
    QueryCalibrationState,
    
    ServerStateUpdate(crate::state::ServerState),
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

    // Session Status Broadcasts
    SessionCreated(Session),
    SessionUpdate(Session),

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
    BroadcastScenePaths(crate::path::VectorFrame),

    // Live Camera Review Stream (Server -> Terminal)
    CameraPreviewFrame(crate::camera::CameraStreamFrame),
    SetCameraStreamState {
        enabled: bool,
        stream_type: crate::camera::CameraStreamType,
    },



    // Curator Workflow & Session Channel Messages
    /// Channel connection boundary (Server <-> Terminal)
    OpenSession {
        session_id: Uuid,
        channel_name: String,
    },
    CloseSession {
        session_id: Uuid,
    },
    CuratorPrompt(curator_core::ActorPrompt),
    CuratorPromptResponse(curator_core::ActorPromptResponse),
    CuratorEnvelope(curator_core::CuratorEventEnvelope),

    // Generic Receiver Mode (Terminal streams frames directly to Server DAC)
    SetReceiverMode {
        active: bool,
        source_name: Option<String>,
    },
    ReceiverModeUpdate {
        active: bool,
        source_name: Option<String>,
    },
    /// Live audio stream frame sent to server (used in Receiver Mode or for in-game audio reactivity)
    AudioStreamFrame {
        /// PCM audio samples in range [-1.0, 1.0]
        samples: Vec<f32>,
        /// Number of channels: 1 = Mono, 2 = Stereo (Interleaved: L, R, L, R)
        channels: u8,
        /// Sampling rate in Hz (e.g. 44100, 48000, 30000)
        sample_rate: u32,
    },

    // Lifecycle
    /// Broadcast server instance ID on connection/restart
    ServerInfo {
        instance_id: Uuid,
    },
    /// Command to shutdown the server
    ShutdownServer,

    // Camera ↔ projector calibration (Terminal -> Server, then status Server -> Terminals)
    /// Projects the calibration dots and measures them with the camera
    StartCameraCalibration,
    /// Stops a running calibration; the previous mapping stays in use
    CancelCameraCalibration,
    /// Forgets the measured mapping and goes back to the uncalibrated default
    ResetCameraCalibration,
    QueryCameraCalibration,
    CameraCalibrationStatus(crate::camera::CameraCalibrationStatus),
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

/// A message a terminal plugin wants sent to the server. Write it with `MessageWriter<ToServer>`;
/// the terminal's network plugin sends it, so feature plugins never touch the connection.
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct ToServer(pub NetworkMessage);

/// Server configuration
pub const SERVER_PORT: u16 = 6000;
pub const SERVER_HOST: &str = "0.0.0.0";

/// Event for mouse position updates from clients
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct MousePositionEvent {
    pub client_id: u64,
    pub position: Option<bevy::prelude::Vec3>,
}

/// Event for mouse button input from clients
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct MouseButtonInputEvent {
    pub client_id: u64,
    pub button: String,
    pub pressed: bool,
    pub position: Option<bevy::prelude::Vec3>,
}

/// Event for keyboard input from clients
#[derive(bevy::prelude::Message, Debug, Clone)]
pub struct KeyboardInputEvent {
    pub client_id: u64,
    pub key: String,
    pub pressed: bool,
}
