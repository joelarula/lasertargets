use bevy::prelude::*;
use lyon_path::{Path, PathEvent};
use serde::{Deserialize, Serialize};
fn default_color_val() -> u8 {
    255
}

/// A single point in a path with color and dwell information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PathPoint {
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_color_val")]
    pub r: u8,
    #[serde(default = "default_color_val")]
    pub g: u8,
    #[serde(default = "default_color_val")]
    pub b: u8,
    #[serde(default)]
    pub dwell: u8, // 0 = normal, 1-7 = dwell count (also used as dwell hint)
}

impl PathPoint {
    pub fn new(x: f32, y: f32, r: u8, g: u8, b: u8, dwell: u8) -> Self {
        Self { x, y, r, g, b, dwell }
    }
    
    /// Convert Bevy Color to RGB u8 tuple
    pub fn color_to_rgb(color: Color) -> (u8, u8, u8) {
        let srgba = color.to_srgba();
        (
            (srgba.red * 255.0) as u8,
            (srgba.green * 255.0) as u8,
            (srgba.blue * 255.0) as u8,
        )
    }
    
    pub fn from_vec2_color(pos: Vec2, color: Color, dwell: u8) -> Self {
        let (r, g, b) = Self::color_to_rgb(color);
        Self {
            x: pos.x,
            y: pos.y,
            r,
            g,
            b,
            dwell,
        }
    }
}

/// Line style pattern for UniversalPath segments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LineStyle {
    #[default]
    Continuous,
    Dashed,
    Dotted,
    Independent,
}

/// Optimization hint for UniversalPath segments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PathHint {
    #[default]
    General,
    /// Use the galvo text-optimized pipeline (TSP sort, D-P simplify, dwell injection)
    Text,
}

/// A segment of a path with simple point representation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PathSegment {
    pub points: Vec<PathPoint>,
    #[serde(default)]
    pub line_style: LineStyle,
    #[serde(default)]
    pub hint: PathHint,
}

impl PathSegment {
    pub fn new(points: Vec<PathPoint>) -> Self {
        Self {
            points,
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
    
    pub fn empty() -> Self {
        Self {
            points: Vec::new(),
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }

    pub fn empty_text() -> Self {
        Self {
            points: Vec::new(),
            line_style: LineStyle::Continuous,
            hint: PathHint::Text,
        }
    }

    /// Expand line style (Continuous, Dashed, Dotted, Independent) into styled points with lit and blanked sections
    pub fn expand_line_style(&self) -> Vec<PathPoint> {
        if self.points.len() < 2 || self.line_style == LineStyle::Continuous || self.line_style == LineStyle::Independent {
            return self.points.clone();
        }

        let (dash_period, lit_length, step_size) = match self.line_style {
            LineStyle::Continuous | LineStyle::Independent => return self.points.clone(),
            LineStyle::Dashed => (2.0, 1.2, 0.25),   // 1.2m dash, 0.8m gap -> ~16 points per 32m perimeter
            LineStyle::Dotted => (1.5, 0.15, 0.15),  // 0.15m dot, 1.35m gap -> ~20 points per 32m perimeter
        };

        let mut styled_points = Vec::new();
        let mut accum_dist = 0.0;

        for window in self.points.windows(2) {
            let p1 = &window[0];
            let p2 = &window[1];

            let dx = p2.x - p1.x;
            let dy = p2.y - p1.y;
            let edge_len = (dx * dx + dy * dy).sqrt();

            if edge_len <= 0.0001 {
                let phase = accum_dist % dash_period;
                let is_lit = phase < lit_length;
                let (r, g, b) = if is_lit { (p1.r, p1.g, p1.b) } else { (0, 0, 0) };
                styled_points.push(PathPoint::new(p1.x, p1.y, r, g, b, p1.dwell));
                continue;
            }

            let steps = (edge_len / step_size).ceil() as usize;
            for i in 0..steps {
                let t = i as f32 / steps as f32;
                let x = p1.x + dx * t;
                let y = p1.y + dy * t;
                let curr_dist = accum_dist + edge_len * t;

                let phase = curr_dist % dash_period;
                let is_lit = phase < lit_length;

                let (r, g, b) = if is_lit {
                    (p1.r, p1.g, p1.b)
                } else {
                    (0, 0, 0)
                };

                styled_points.push(PathPoint::new(x, y, r, g, b, p1.dwell));
            }

            accum_dist += edge_len;
        }

        if let Some(last) = self.points.last() {
            let phase = accum_dist % dash_period;
            let is_lit = phase < lit_length;
            let (r, g, b) = if is_lit { (last.r, last.g, last.b) } else { (0, 0, 0) };
            styled_points.push(PathPoint::new(last.x, last.y, r, g, b, last.dwell));
        }

        // Preserve closed loop seam integrity
        let is_closed_loop = self.points.len() > 2 && {
            let dx = self.points[0].x - self.points.last().unwrap().x;
            let dy = self.points[0].y - self.points.last().unwrap().y;
            (dx * dx + dy * dy) <= 0.0001
        };

        if is_closed_loop && styled_points.len() > 1 {
            let first = styled_points[0].clone();
            let last_idx = styled_points.len() - 1;
            styled_points[last_idx].x = first.x;
            styled_points[last_idx].y = first.y;
            styled_points[last_idx].r = first.r;
            styled_points[last_idx].g = first.g;
            styled_points[last_idx].b = first.b;
        }

        styled_points
    }
    
    /// Create a builder for constructing path segments point by point
    pub fn builder() -> PathSegmentBuilder {
        PathSegmentBuilder {
            points: Vec::new(),
        }
    }
    
    /// Add a point to this segment
    pub fn push_point(&mut self, point: PathPoint) {
        self.points.push(point);
    }
    
    /// Add a point with position, color, and dwell
    pub fn push(&mut self, x: f32, y: f32, color: Color, dwell: u8) {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        self.points.push(PathPoint::new(x, y, r, g, b, dwell));
    }
    
    /// Add a point from Vec2
    pub fn push_vec2(&mut self, pos: Vec2, color: Color, dwell: u8) {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        self.points.push(PathPoint::new(pos.x, pos.y, r, g, b, dwell));
    }
    
    /// Create a line segment from start to end with color
    pub fn line(start: Vec2, end: Vec2, color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        Self {
            points: vec![
                PathPoint::new(start.x, start.y, r, g, b, dwell),
                PathPoint::new(end.x, end.y, r, g, b, dwell),
            ],
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
    
    /// Create multiple connected line segments (polyline)
    pub fn polyline(points: &[Vec2], color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        let path_points = points
            .iter()
            .map(|p| PathPoint::new(p.x, p.y, r, g, b, dwell))
            .collect();
        Self {
            points: path_points,
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
    
    /// Create a closed polygon (last point connects to first)
    pub fn polygon(points: &[Vec2], color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        let mut path_points: Vec<PathPoint> = points
            .iter()
            .map(|p| PathPoint::new(p.x, p.y, r, g, b, dwell))
            .collect();
        // Add first point again to close the loop
        if !points.is_empty() {
            path_points.push(PathPoint::new(points[0].x, points[0].y, r, g, b, dwell));
        }
        Self {
            points: path_points,
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
    
    /// Create from Lyon path (for backward compatibility)
    pub fn from_lyon_path(path: &Path, color: Color, _line_width: f32) -> Self {
        let mut points = Vec::new();
        let (r, g, b) = PathPoint::color_to_rgb(color);
        for event in path.iter() {
            match event {
                PathEvent::Begin { at } => {
                    points.push(PathPoint::new(at.x, at.y, r, g, b, 0));
                }
                PathEvent::Line { to, .. } => {
                    points.push(PathPoint::new(to.x, to.y, r, g, b, 0));
                }
                PathEvent::Quadratic { ctrl, to, .. } => {
                    points.push(PathPoint::new(ctrl.x, ctrl.y, r, g, b, 0));
                    points.push(PathPoint::new(to.x, to.y, r, g, b, 0));
                }
                PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                    points.push(PathPoint::new(ctrl1.x, ctrl1.y, r, g, b, 0));
                    points.push(PathPoint::new(ctrl2.x, ctrl2.y, r, g, b, 0));
                    points.push(PathPoint::new(to.x, to.y, r, g, b, 0));
                }
                PathEvent::End { .. } => {}
            }
        }
        Self {
            points,
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
}

/// Builder for creating PathSegments point by point
pub struct PathSegmentBuilder {
    points: Vec<PathPoint>,
}

impl PathSegmentBuilder {
    /// Add a point with full control
    pub fn point(mut self, x: f32, y: f32, r: u8, g: u8, b: u8, dwell: u8) -> Self {
        self.points.push(PathPoint::new(x, y, r, g, b, dwell));
        self
    }
    
    /// Add a point with position and color
    pub fn add(mut self, x: f32, y: f32, color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        self.points.push(PathPoint::new(x, y, r, g, b, dwell));
        self
    }
    
    /// Add a point from Vec2
    pub fn add_vec2(mut self, pos: Vec2, color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        self.points.push(PathPoint::new(pos.x, pos.y, r, g, b, dwell));
        self
    }
    
    /// Build the final PathSegment
    pub fn build(self) -> PathSegment {
        PathSegment {
            points: self.points,
            line_style: LineStyle::Continuous,
            hint: PathHint::General,
        }
    }
}

/// Universal path representation containing multiple segments
#[derive(Component, Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct UniversalPath {
    pub segments: Vec<PathSegment>,
}

impl UniversalPath {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn from_segment(segment: PathSegment) -> Self {
        Self {
            segments: vec![segment],
        }
    }

    /// Create from Lyon path (for backward compatibility)
    pub fn from_path(path: Path, color: Color, line_width: f32) -> Self {
        Self {
            segments: vec![PathSegment::from_lyon_path(&path, color, line_width)],
        }
    }

    pub fn add_segment(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    /// Add Lyon path (for backward compatibility)
    pub fn add_path(&mut self, path: Path, color: Color, line_width: f32) {
        self.segments
            .push(PathSegment::from_lyon_path(&path, color, line_width));
    }

    /// Create a circle path (delegates to `common::shapes::circle`)
    #[inline]
    pub fn circle(center: Vec2, radius: f32, color: Color) -> Self {
        crate::shapes::circle(center, radius, color)
    }

    /// Create a balloon path shape (delegates to `common::shapes::balloon`)
    #[inline]
    pub fn balloon(center: Vec2, radius: f32, color: Color) -> Self {
        crate::shapes::balloon(center, radius, color)
    }

    /// Create a diamond path (delegates to `common::shapes::diamond`)
    #[inline]
    pub fn diamond(center: Vec2, half_size: f32, color: Color) -> Self {
        crate::shapes::diamond(center, half_size, color)
    }

    /// Create a crosshair path (delegates to `common::shapes::crosshair`)
    #[inline]
    pub fn crosshair(center: Vec2, size: f32, color: Color) -> Self {
        crate::shapes::crosshair(center, size, color)
    }

    /// Create a rectangle path (delegates to `common::shapes::rectangle`)
    #[inline]
    pub fn rectangle(top_left: Vec2, size: Vec2, color: Color) -> Self {
        crate::shapes::rectangle(top_left, size, color)
    }

    /// Flatten path to line segments for gizmo rendering
    pub fn flatten(&self) -> Vec<Vec2> {
        let mut result = Vec::new();
        
        for segment in &self.segments {
            for point in &segment.points {
                result.push(Vec2::new(point.x, point.y));
            }
        }
        
        result
    }
}

pub trait PathProvider {
    fn to_universal_path(&self) -> UniversalPath;
}

#[derive(Component)]
pub struct PathRenderable {
    pub visible: bool,
}

impl Default for PathRenderable {
    fn default() -> Self {
        Self { visible: true }
    }
}

// Re-export generic vector transport primitives from dedicated transport module
pub use crate::transport::{AbstractPathData, BroadcastScenePaths, VectorFrame};

/// Easing functions for keyframe vector interpolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EasingFunction {
    #[default]
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
}

impl EasingFunction {
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => t * (2.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
        }
    }
}

/// Loop behavior for keyframe path animations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LoopMode {
    #[default]
    Once,
    PingPong,
    Repeat,
}

/// Component for animating a UniversalPath between two keyframes over time
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PathAnimation {
    pub keyframe_a: UniversalPath,
    pub keyframe_b: UniversalPath,
    pub duration_secs: f32,
    pub elapsed_secs: f32,
    pub easing: EasingFunction,
    pub loop_mode: LoopMode,
}

impl PathAnimation {
    pub fn new(keyframe_a: UniversalPath, keyframe_b: UniversalPath, duration_secs: f32) -> Self {
        Self {
            keyframe_a,
            keyframe_b,
            duration_secs: duration_secs.max(0.001),
            elapsed_secs: 0.0,
            easing: EasingFunction::Linear,
            loop_mode: LoopMode::Once,
        }
    }
}

/// Signal sources that drive path parameter modulation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModulatorSource {
    /// Time-based Sine Wave Low-Frequency Oscillator (LFO)
    LfoSine { frequency_hz: f32 },
    /// Time-based Triangle Wave LFO
    LfoTriangle { frequency_hz: f32 },
    /// Gamepad analog axis (e.g. Right Trigger pressure 0.0 - 1.0)
    GamepadTrigger { value: f32 },
    /// Target entity health / charge ratio (0.0 - 1.0)
    Ratio { value: f32 },
    /// Manual signal parameter input
    Manual(f32),
}

impl Default for ModulatorSource {
    fn default() -> Self {
        Self::LfoSine { frequency_hz: 1.0 }
    }
}

impl ModulatorSource {
    pub fn sample(&self, elapsed_secs: f32) -> f32 {
        match self {
            Self::LfoSine { frequency_hz } => {
                let radians = elapsed_secs * frequency_hz * std::f32::consts::TAU;
                (radians.sin() + 1.0) * 0.5
            }
            Self::LfoTriangle { frequency_hz } => {
                let cycle = elapsed_secs * frequency_hz;
                let rem = cycle.fract();
                if rem < 0.5 { rem * 2.0 } else { (1.0 - rem) * 2.0 }
            }
            Self::GamepadTrigger { value } => value.clamp(0.0, 1.0),
            Self::Ratio { value } => value.clamp(0.0, 1.0),
            Self::Manual(val) => val.clamp(0.0, 1.0),
        }
    }
}

/// Modulator component connecting a signal source through an EasingFunction to drive UniversalPath morphing
#[derive(Component, Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathModulator {
    pub source: ModulatorSource,
    pub easing: EasingFunction,
    pub keyframe_a: UniversalPath,
    pub keyframe_b: UniversalPath,
    pub current_t: f32,
}

impl PathModulator {
    pub fn new(keyframe_a: UniversalPath, keyframe_b: UniversalPath, source: ModulatorSource, easing: EasingFunction) -> Self {
        Self {
            source,
            easing,
            keyframe_a,
            keyframe_b,
            current_t: 0.0,
        }
    }
}
