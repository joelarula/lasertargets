use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};

/// Type of physical camera stream available on the platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraStreamType {
    /// Physical Pi Camera (RGB visible spectrum feed for scene alignment & laser tracking)
    PiCamera,
    /// Physical Thermal Camera (IR heat spectrum feed for impact analysis, planned)
    ThermalCamera,
}

impl Default for CameraStreamType {
    fn default() -> Self {
        Self::PiCamera
    }
}

/// Configuration settings for physical camera stream overlay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalCameraSettings {
    pub enabled: bool,
    pub active_stream: CameraStreamType,
    pub opacity: f32,
    pub stream_width: u32,
    pub stream_height: u32,
    pub stream_fps: u32,
}

impl Default for PhysicalCameraSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            active_stream: CameraStreamType::PiCamera,
            opacity: 0.6,
            stream_width: 640,
            stream_height: 480,
            stream_fps: 30,
        }
    }
}

/// Telemetry marker for detected visual events sent alongside preview frames
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualEventMarker {
    /// Normalized coordinates in camera sensor space [0.0, 1.0]
    pub camera_norm_pos: Vec2,
    /// Projected coordinates in laser galvo space [-1.0, 1.0]
    pub galvo_pos: Vec2,
    /// Event classification (e.g. PlayerLaserHit, CalibrationAnchor)
    pub kind: VisualEventKind,
    /// Optical energy / confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualEventKind {
    PlayerLaserHit,
    CalibrationAnchor(u8),
    BeamVerification,
    TargetOcclusion,
}

/// A compressed preview frame with overlaid detection markers for review in Terminal UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraStreamFrame {
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    /// Encoded JPEG/MJPEG buffer from bcm2835-codec or software compressor
    pub jpeg_data: Vec<u8>,
    /// Live visual event annotations detected in this specific frame
    pub detected_events: Vec<VisualEventMarker>,
    /// Current 4-point homography calibration quad in camera normalized space [0.0, 1.0]
    pub calibration_quad: Option<[Vec2; 4]>,
}
