//! # Laser-Vision
//!
//! Camera analysis for LaserTargets, as plain Rust with no ECS: frames come from a
//! [`FrameSource`], pluggable [`VisionAnalyzer`]s turn them into detections and signals, a
//! [`Tracker`] gives detections a stable identity, and a [`Homography`] maps them into projector
//! coordinates. `server-core`'s vision plugin runs a [`VisionEngine`] on a worker thread and
//! feeds the results into Bevy.
//!
//! - **Analyzers** are the extension point: a classic algorithm ([`SpotAnalyzer`] for laser dots,
//!   [`MotionAnalyzer`] for scene motion) or a model (object detector). Each adds labelled
//!   [`Detection`]s and/or frame-wide signals.
//! - **Coordinates**: camera space is normalised (0, 0) top-left to (1, 1) bottom-right;
//!   projector space is `common::scene::projector_ndc`, -1..1 with +Y down.
//! - **Calibration** ([`calibration`]): projected dots seen by the camera give a least-squares
//!   camera → projector [`Homography`].

pub mod analyzer;
pub mod calibration;
pub mod capture;
pub mod detector;
pub mod engine;
pub mod homography;
pub mod tracker;

pub use analyzer::{AnalysisOutput, Detection, MotionAnalyzer, VisionAnalyzer};
pub use calibration::{fit_calibration, grid_pattern, median, CalibrationFit, MIN_CALIBRATION_POINTS};
pub use capture::{FrameSource, MockFrameSource, RawFrame};
#[cfg(target_os = "linux")]
pub use capture::ProcessPipeFrameSource;
pub use detector::{detect_laser_spots, DetectedSpot, DetectionConfig, LaserColor, SpotAnalyzer};
pub use engine::{VisionEngine, VisionFrame, VisionObject};
pub use homography::Homography;
pub use tracker::{LostTrack, TrackId, TrackedDetection, Tracker, TrackerConfig, TrackerUpdate};
