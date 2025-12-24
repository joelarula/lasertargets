use serde::{Deserialize, Serialize};

/// Network messages exchanged between server and terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Simple ping message from server
    Ping { timestamp: u64 },
    /// Pong response from client
    Pong { timestamp: u64 },
}

/// Server configuration
pub const SERVER_PORT: u16 = 6000;
pub const SERVER_HOST: &str = "0.0.0.0";
