use bevy::color::Color;
use serde::{Deserialize, Serialize};

/// Network messages exchanged between server and terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HunterTarget {
    Basic(f32,Color), // size
    Baloon(f32,Color), // size
}