//! # Laser-Vision
//!
//! A generic, cross-platform computer vision and optical tracking library for
//! LaserTargets.
//!
//! ## Core Architecture
//! - **Generic Platform Independence**: Operates on pure pixel buffers (`RawFrame`)
//!   and abstract frame producers ([`FrameSource`]). Zero hard dependencies on Pi hardware,
//!   compiling cleanly across Windows, macOS, and Linux (including ARM64 Docker builds).
//! - **Spectral Spot Detection**: Wavelength isolation (Green 520–532nm, Red 635–650nm,
//!   or high-contrast luminance) with sub-pixel centroid moment analysis.
//! - **Homography & Perspective Calibration**: 3x3 Direct Linear Transformation (DLT) solver
//!   mapping camera sensor coordinates into normalized galvo laser projector space `[-1.0, 1.0]`.
//! - **End-to-End Pipeline**: Unified [`VisionEngine`] managing frame intake, optical
//!   filtering, and event generation.

pub mod capture;
pub mod detector;
pub mod homography;
pub mod pipeline;

// Re-export primary types for clean consumer ergonomics
pub use capture::{FrameSource, MockFrameSource, RawFrame};
#[cfg(target_os = "linux")]
pub use capture::ProcessPipeFrameSource;

pub use detector::{detect_laser_spots, DetectionConfig, DetectedSpot, LaserColor};
pub use homography::Homography;
pub use pipeline::{VisionEngine, VisionFrameResult};

