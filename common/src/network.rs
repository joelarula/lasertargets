use serde::{Deserialize, Serialize};

/// Network messages exchanged between server and terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Simple ping message from server
    Ping { timestamp: u64 },
    /// Pong response from client
    Pong { timestamp: u64 },
}

impl NetworkMessage {
    /// Serialize the message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize bytes to a NetworkMessage
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Server configuration
pub const SERVER_PORT: u16 = 6000;
pub const SERVER_HOST: &str = "0.0.0.0";