pub mod corner;
pub mod optimize;
pub mod simplify;
pub mod text_optimize;
pub mod helios;
pub mod morph;
pub mod show;
pub mod stream;
pub mod ttf;

pub use ttf::{from_ttf_text, LaserTextOptions};
pub use optimize::compose_frame;

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
    /// Optional per-point hold override (extra repeats); replaces the computed corner hold.
    /// Empty means no overrides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holds: Vec<Option<u16>>,
}

impl LaserSegment {
    /// Check if segment forms a closed loop polygon (start and end points within 40 DAC units)
    pub fn is_closed(&self) -> bool {
        self.points.len() > 2 && {
            let dx = self.points[0].x as i32 - self.points.last().unwrap().x as i32;
            let dy = self.points[0].y as i32 - self.points.last().unwrap().y as i32;
            (dx * dx + dy * dy) <= 1600
        }
    }
    pub fn new(points: Vec<LaserPoint>) -> Self {
        Self { points, holds: Vec::new() }
    }

    /// Segment with a hold override per point (`holds` is as long as `points`)
    pub fn with_holds(points: Vec<LaserPoint>, holds: Vec<Option<u16>>) -> Self {
        debug_assert_eq!(points.len(), holds.len());
        Self { points, holds }
    }

    /// Hold override at point `i`, if any
    pub fn hold(&self, i: usize) -> Option<u16> {
        self.holds.get(i).copied().flatten()
    }
}

/// Configuration for laser path optimization.
///
/// Shapes (general segments) use the galvo and diode models, set in physical units: metres per
/// second on the scene plane and milliseconds. The point-count fields (`corner_dwell_points`,
/// `start_dwell_points`, `end_dwell_points`, `blank_*`, `interp_*`) apply to the text pipeline only.
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
    /// Galvo model: speed along lit lines on the scene plane. Sets the spacing of intermediate
    /// points so the galvo moves linearly at constant speed.
    pub lit_speed_m_s: f32,
    /// Galvo model: largest lit step per point, from the galvo rating. Caps `lit_speed_m_s`.
    pub max_lit_step_dac: f32,
    /// Galvo model: slow-down before and speed-up after a sharp corner or stroke end; points get
    /// denser towards it, matching mirror inertia.
    pub corner_ramp_ms: f32,
    /// Galvo model: hold at a 90 degree turn, scaled with the turn angle; gentle turns get none.
    pub corner_hold_ms: f32,
    /// Galvo model: lit hold at the first point of an open stroke.
    pub stroke_start_hold_ms: f32,
    /// Galvo model: lit hold at the last point of an open stroke, so the mirrors reach the end
    /// before the light goes off.
    pub stroke_end_hold_ms: f32,
    /// Galvo model: travel speed of laser-off jumps between strokes.
    pub blank_speed_m_s: f32,
    /// Galvo model: largest step per point during a laser-off jump. Caps `blank_speed_m_s`.
    pub max_blank_step_dac: f32,
    /// Galvo model: laser-off hold at a stroke end before the galvo leaves.
    pub depart_ms: f32,
    /// Galvo model: hold after a jump before the laser turns on ...
    pub settle_ms: f32,
    /// ... plus this much per metre of jump.
    pub settle_ms_per_m: f32,
    /// Light timing at stroke starts. Positive: light switched on this much earlier (diode
    /// latency). Negative: switched on this much later (mirrors lagging behind).
    pub laser_on_delay_ms: f32,
    /// Light timing at stroke ends. Positive: light cut this much earlier (diode afterglow).
    /// Negative: kept on this much longer, while lagging mirrors finish the stroke.
    pub laser_off_delay_ms: f32,
    /// Output point rate used to turn the millisecond values into point counts. Set at runtime
    /// from the projector configuration, not read from the config file.
    #[serde(skip, default = "default_pps")]
    pub pps: u32,
    /// DAC units per metre on the scene plane, used to turn metres into DAC units. Set at runtime
    /// from the scene setup, not read from the config file.
    #[serde(skip, default = "default_dac_per_m")]
    pub dac_per_m: f32,
}

fn default_dac_per_m() -> f32 {
    // A 60 degree projector 10 m from the scene
    355.0
}

fn default_pps() -> u32 {
    30000
}

impl OptimizeConfig {
    /// Number of output points that last `ms` milliseconds at the current point rate
    pub fn ms_to_points(&self, ms: f32) -> usize {
        (ms.max(0.0) / 1000.0 * self.pps as f32).round() as usize
    }

    /// [`Self::ms_to_points`] keeping the sign
    pub fn ms_to_points_signed(&self, ms: f32) -> i32 {
        (ms / 1000.0 * self.pps as f32).round() as i32
    }

    /// DAC distance covered per output point at `speed_m_s`, capped at `max_step_dac`
    pub fn step_dac(&self, speed_m_s: f32, max_step_dac: f32) -> f32 {
        let step = speed_m_s.max(0.0) / self.pps.max(1) as f32 * self.dac_per_m;
        step.min(max_step_dac).max(1.0)
    }
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
            // ~0.33 m per point at 30 kpps, about 120 DAC units on a scene 10 m away
            lit_speed_m_s: 10000.0,
            max_lit_step_dac: 150.0,
            corner_ramp_ms: 0.1,
            corner_hold_ms: 0.2,
            stroke_start_hold_ms: 0.2,
            stroke_end_hold_ms: 0.2,
            blank_speed_m_s: 20000.0,
            max_blank_step_dac: 300.0,
            depart_ms: 0.5,
            settle_ms: 0.5,
            settle_ms_per_m: 0.05,
            laser_on_delay_ms: 0.0,
            laser_off_delay_ms: 0.0,
            pps: default_pps(),
            dac_per_m: default_dac_per_m(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: u16, y: u16) -> LaserPoint {
        LaserPoint::new(x, y, 255, 255, 255, 255)
    }

    #[test]
    fn blanked_point_is_blanked() {
        let p = LaserPoint::blanked(10, 20);
        assert!(p.is_blanked());
        assert_eq!((p.x, p.y), (10, 20));
    }

    #[test]
    fn any_lit_channel_is_not_blanked() {
        assert!(!LaserPoint::new(0, 0, 1, 0, 0, 0).is_blanked());
        assert!(!LaserPoint::new(0, 0, 0, 1, 0, 0).is_blanked());
        assert!(!LaserPoint::new(0, 0, 0, 0, 1, 0).is_blanked());
        assert!(!LaserPoint::new(0, 0, 0, 0, 0, 1).is_blanked());
    }

    #[test]
    fn segment_closed_within_40_units() {
        // Exactly 40 units apart is still closed (inclusive threshold)
        let seg = LaserSegment::new(vec![pt(0, 0), pt(500, 0), pt(500, 500), pt(40, 0)]);
        assert!(seg.is_closed());
        let seg = LaserSegment::new(vec![pt(0, 0), pt(500, 0), pt(500, 500), pt(41, 0)]);
        assert!(!seg.is_closed());
    }

    #[test]
    fn two_point_segment_never_closed() {
        let seg = LaserSegment::new(vec![pt(0, 0), pt(0, 0)]);
        assert!(!seg.is_closed());
    }

    #[test]
    fn segment_and_corner_closed_checks_agree() {
        let shapes = [
            vec![pt(0, 0), pt(500, 0), pt(500, 500), pt(0, 0)],
            vec![pt(0, 0), pt(500, 0), pt(500, 500), pt(100, 100)],
            vec![pt(0, 0), pt(10, 0)],
        ];
        for points in shapes {
            let seg = LaserSegment::new(points.clone());
            assert_eq!(seg.is_closed(), corner::is_closed_points(&points));
        }
    }

    #[test]
    fn optimize_config_roundtrips_through_json() {
        let mut config = OptimizeConfig::default();
        config.corner_dwell_points = 7;
        config.simplify_min_distance = 12.5;
        let json = serde_json::to_string(&config).unwrap();
        let back: OptimizeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.corner_dwell_points, 7);
        assert_eq!(back.simplify_min_distance, 12.5);
        assert_eq!(back.blank_jump_steps, config.blank_jump_steps);
    }
}
