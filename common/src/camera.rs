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
