pub mod corner;
pub mod optimize;
pub mod simplify;

use serde::{Deserialize, Serialize};

/// A single laser output point in DAC coordinate space (0-4095 for 12-bit DACs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaserPoint {
    pub x: u16,
    pub y: u16,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub i: u8,
}

impl LaserPoint {
    pub fn new(x: u16, y: u16, r: u8, g: u8, b: u8, i: u8) -> Self {
        Self { x, y, r, g, b, i }
    }

    /// Create a blanked point (laser off) at the given DAC position.
    pub fn blanked(x: u16, y: u16) -> Self {
        Self {
            x,
            y,
            r: 0,
            g: 0,
            b: 0,
            i: 0,
        }
    }

    pub fn is_blanked(&self) -> bool {
        self.r == 0 && self.g == 0 && self.b == 0 && self.i == 0
    }
}

/// A contiguous drawn segment of laser points (laser on between consecutive points).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaserSegment {
    pub points: Vec<LaserPoint>,
}

impl LaserSegment {
    pub fn new(points: Vec<LaserPoint>) -> Self {
        Self { points }
    }
}

/// Configuration for laser path optimization. All values that were previously hardcoded
/// in projector.rs are now exposed here with sensible defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizeConfig {
    /// Extra repeated points at detected corners (default: 3)
    pub corner_dwell_points: u16,
    /// Angle in degrees below which a point is considered a corner (default: 135.0)
    pub corner_angle_threshold: f32,
    /// Blanked dwell points at segment start (default: 3)
    pub start_dwell_points: u16,
    /// Blanked dwell points at segment end (default: 3)
    pub end_dwell_points: u16,
    /// Blanked dwell points before leaving previous segment during jump (default: 15)
    pub blank_end_dwell: u16,
    /// Blanked dwell points before starting next segment during jump (default: 15)
    pub blank_start_dwell: u16,
    /// Number of interpolated blanked points during a jump between segments (default: 60)
    pub blank_jump_steps: u16,
    /// Max DAC-space distance between consecutive points before adding interpolation (default: 200.0)
    pub interp_distance_threshold: f32,
    /// Target spacing for interpolated points in DAC-space (default: 100.0)
    pub interp_spacing: f32,
    /// Remove points closer than this distance (0.0 = disabled)
    pub simplify_min_distance: f32,
    /// Remove near-collinear points with angle above this threshold in degrees (0.0 = disabled)
    pub simplify_collinear_angle: f32,
    /// Enable dynamic dwell calculation based on point-to-point distance
    pub dynamic_dwell: bool,
    /// Minimum dwell (repeats) for dynamic dwell
    pub min_dwell: u8,
    /// Maximum dwell (repeats) for dynamic dwell
    pub max_dwell: u8,
    /// Distance below which dwell is max_dwell, above which dwell is min_dwell
    pub dwell_distance_threshold: f32,
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            corner_dwell_points: 3,
            corner_angle_threshold: 135.0,
            start_dwell_points: 3,
            end_dwell_points: 3,
            blank_end_dwell: 15,
            blank_start_dwell: 15,
            blank_jump_steps: 60,
            interp_distance_threshold: 200.0,
            interp_spacing: 100.0,
            simplify_min_distance: 0.0,
            simplify_collinear_angle: 0.0,
            dynamic_dwell: true,
            min_dwell: 1,
            max_dwell: 8,
            dwell_distance_threshold: 20.0,
        }
    }
}
