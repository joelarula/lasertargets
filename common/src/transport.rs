use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::path::UniversalPath;

/// Abstract minimal visual path data for client terminal rendering and DAC dispatch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AbstractPathData {
    pub path: UniversalPath,
    pub position: Vec3,
}

impl AbstractPathData {
    pub fn new(path: UniversalPath, position: Vec3) -> Self {
        Self { path, position }
    }
}

/// A temporal snapshot frame containing all active scene vector polylines at a single tick.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct VectorFrame {
    /// Frame sequence index / timestamp counter
    pub frame_id: u64,
    /// Collection of all active polylines and their world positions in the frame
    pub paths: Vec<AbstractPathData>,
}

impl VectorFrame {
    pub fn new(frame_id: u64, paths: Vec<AbstractPathData>) -> Self {
        Self { frame_id, paths }
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }
}

/// Single aggregated message broadcasting all active abstract scene paths every tick
#[derive(Message, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BroadcastScenePaths {
    pub frame: VectorFrame,
}

impl BroadcastScenePaths {
    pub fn new(frame: VectorFrame) -> Self {
        Self { frame }
    }
}

/// SystemSet identifying abstract path extraction and dispatch pipeline
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransportSystemSet;
