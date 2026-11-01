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

    /// Linearly interpolate between two PathPoints with factor t clamped to [0.0, 1.0].
    #[inline(always)]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;
        let x = self.x * inv_t + other.x * t;
        let y = self.y * inv_t + other.y * t;
        let r = (self.r as f32 * inv_t + other.r as f32 * t) as u8;
        let g = (self.g as f32 * inv_t + other.g as f32 * t) as u8;
        let b = (self.b as f32 * inv_t + other.b as f32 * t) as u8;
        let dwell = if t < 0.5 { self.dwell } else { other.dwell };
        Self { x, y, r, g, b, dwell }
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

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn from_segments(segments: Vec<PathSegment>) -> Self {
        Self { segments }
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

// =========================================================================
// Vector Transport Primitives
// =========================================================================

/// Abstract visual path data for client terminal rendering and network streaming
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

/// Signal sources that drive path parameter modulation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ModulatorSource {
    /// Timeline keyframe animation driven by elapsed delta time with looping
    Timeline {
        duration_secs: f32,
        elapsed_secs: f32,
        loop_mode: LoopMode,
    },
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
    /// Advance state (for stateful sources like Timeline) and return current normalized sample [0.0, 1.0]
    pub fn advance_and_sample(&mut self, delta_secs: f32, total_elapsed_secs: f32) -> f32 {
        match self {
            Self::Timeline {
                duration_secs,
                elapsed_secs,
                loop_mode,
            } => {
                *elapsed_secs += delta_secs;
                let dur = duration_secs.max(0.0001);
                let raw_t = (*elapsed_secs / dur).max(0.0);
                match loop_mode {
                    LoopMode::Once => raw_t.min(1.0),
                    LoopMode::Repeat => raw_t.fract(),
                    LoopMode::PingPong => {
                        let cycle = (raw_t as u32) % 2;
                        let rem = raw_t.fract();
                        if cycle == 0 {
                            rem
                        } else {
                            1.0 - rem
                        }
                    }
                }
            }
            other => other.sample(total_elapsed_secs),
        }
    }

    /// Sample source at given timestamp [0.0, 1.0]
    pub fn sample(&self, elapsed_secs: f32) -> f32 {
        match self {
            Self::Timeline {
                duration_secs,
                elapsed_secs: local_elapsed,
                loop_mode,
            } => {
                let dur = duration_secs.max(0.0001);
                let raw_t = (*local_elapsed / dur).max(0.0);
                match loop_mode {
                    LoopMode::Once => raw_t.min(1.0),
                    LoopMode::Repeat => raw_t.fract(),
                    LoopMode::PingPong => {
                        let cycle = (raw_t as u32) % 2;
                        let rem = raw_t.fract();
                        if cycle == 0 {
                            rem
                        } else {
                            1.0 - rem
                        }
                    }
                }
            }
            Self::LfoSine { frequency_hz } => {
                let radians = elapsed_secs * frequency_hz * std::f32::consts::TAU;
                (radians.sin() + 1.0) * 0.5
            }
            Self::LfoTriangle { frequency_hz } => {
                let cycle = elapsed_secs * frequency_hz;
                let rem = cycle.fract();
                if rem < 0.5 {
                    rem * 2.0
                } else {
                    (1.0 - rem) * 2.0
                }
            }
            Self::GamepadTrigger { value } => value.clamp(0.0, 1.0),
            Self::Ratio { value } => value.clamp(0.0, 1.0),
            Self::Manual(val) => val.clamp(0.0, 1.0),
        }
    }
}

/// Pluggable morphing strategy trait.
/// Enables contributors to define custom geometric morphing algorithms across keyframes.
pub trait MorphStrategy: Send + Sync + 'static {
    /// Human-readable strategy name
    fn name(&self) -> &'static str;

    /// Unique type identifier for serialization and registry lookup
    fn type_id(&self) -> &'static str;

    /// Morphs keyframe_a into keyframe_b at parameter t in [0.0, 1.0] in-place into output.
    /// MUST be zero-allocation during steady-state processing.
    fn morph(
        &mut self,
        keyframe_a: &UniversalPath,
        keyframe_b: &UniversalPath,
        t: f32,
        output: &mut UniversalPath,
    );

    /// Deep clone support for dynamic boxed strategies
    fn clone_box(&self) -> Box<dyn MorphStrategy>;
}

impl Clone for Box<dyn MorphStrategy> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Baseline fast cycle-modulo morphing strategy with zero allocation and multi-segment safety.
#[derive(Debug, Clone, Default)]
pub struct CycleModuloMorphStrategy;

impl MorphStrategy for CycleModuloMorphStrategy {
    fn name(&self) -> &'static str {
        "CycleModulo"
    }

    fn type_id(&self) -> &'static str {
        "cycle_modulo"
    }

    fn morph(
        &mut self,
        keyframe_a: &UniversalPath,
        keyframe_b: &UniversalPath,
        t: f32,
        output: &mut UniversalPath,
    ) {
        if keyframe_a.segments.is_empty() || keyframe_b.segments.is_empty() {
            return;
        }

        let num_segments = keyframe_a.segments.len().max(keyframe_b.segments.len());
        if output.segments.len() < num_segments {
            output.segments.resize_with(num_segments, PathSegment::empty);
        }

        for s in 0..num_segments {
            let seg_a = &keyframe_a.segments[s % keyframe_a.segments.len()];
            let seg_b = &keyframe_b.segments[s % keyframe_b.segments.len()];

            if seg_a.points.is_empty() && seg_b.points.is_empty() {
                output.segments[s].points.clear();
                continue;
            }

            let max_points = seg_a.points.len().max(seg_b.points.len());
            let out_points = &mut output.segments[s].points;
            out_points.clear();
            out_points.reserve(max_points);

            if seg_a.points.is_empty() {
                out_points.extend_from_slice(&seg_b.points);
            } else if seg_b.points.is_empty() {
                out_points.extend_from_slice(&seg_a.points);
            } else {
                for i in 0..max_points {
                    let pa = &seg_a.points[i % seg_a.points.len()];
                    let pb = &seg_b.points[i % seg_b.points.len()];
                    out_points.push(pa.lerp(pb, t));
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn MorphStrategy> {
        Box::new(self.clone())
    }
}

/// Pluggable registry of available morphing strategies.
#[derive(Resource, Default)]
pub struct MorphStrategyRegistry {
    factories: std::collections::HashMap<String, Box<dyn Fn() -> Box<dyn MorphStrategy> + Send + Sync>>,
}

impl MorphStrategyRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: std::collections::HashMap::new(),
        };
        registry.register::<CycleModuloMorphStrategy>();
        registry
    }

    pub fn register<T: MorphStrategy + Default + Clone>(&mut self) {
        let key = T::default().type_id().to_string();
        self.factories.insert(key, Box::new(|| Box::new(T::default())));
    }

    pub fn create(&self, type_id: &str) -> Option<Box<dyn MorphStrategy>> {
        self.factories.get(type_id).map(|f| f())
    }
}

/// Evaluation context supplied to PathModifiers during in-place processing
pub struct ModifierContext<'a> {
    pub delta_secs: f32,
    pub elapsed_secs: f32,
    pub modulation_t: f32,
    pub audio_spectral: Option<&'a [f32]>,
}

/// Pluggable vector modifier trait.
/// Allows contributors to implement transformers, oscillators, wave shaders, and spatial effects.
pub trait PathModifier: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn type_id(&self) -> &'static str;

    /// Execute the modification in-place on the path's segments and points.
    /// MUST be zero-allocation during steady-state processing.
    fn apply(&mut self, path: &mut UniversalPath, ctx: &ModifierContext);

    fn clone_box(&self) -> Box<dyn PathModifier>;
}

impl Clone for Box<dyn PathModifier> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Component holding an ordered pipeline of pluggable modifiers applied sequentially in-place.
#[derive(Component)]
pub struct PathModifierPipeline {
    pub modifiers: Vec<Box<dyn PathModifier>>,
    pub enabled: bool,
}

impl Default for PathModifierPipeline {
    fn default() -> Self {
        Self {
            modifiers: Vec::new(),
            enabled: true,
        }
    }
}

impl PathModifierPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_modifier(mut self, modifier: impl PathModifier) -> Self {
        self.modifiers.push(Box::new(modifier));
        self
    }

    pub fn add(&mut self, modifier: impl PathModifier) {
        self.modifiers.push(Box::new(modifier));
    }

    pub fn apply(&mut self, path: &mut UniversalPath, ctx: &ModifierContext) {
        if !self.enabled {
            return;
        }
        for modifier in &mut self.modifiers {
            modifier.apply(path, ctx);
        }
    }
}

/// Pluggable registry of available path modifiers.
#[derive(Resource, Default)]
pub struct PathModifierRegistry {
    factories: std::collections::HashMap<String, Box<dyn Fn() -> Box<dyn PathModifier> + Send + Sync>>,
}

impl PathModifierRegistry {
    pub fn new() -> Self {
        Self {
            factories: std::collections::HashMap::new(),
        }
    }

    pub fn register<T: PathModifier + Default + Clone>(&mut self) {
        let key = T::default().type_id().to_string();
        self.factories.insert(key, Box::new(|| Box::new(T::default())));
    }

    pub fn create(&self, type_id: &str) -> Option<Box<dyn PathModifier>> {
        self.factories.get(type_id).map(|f| f())
    }
}

/// Unified modulator component connecting a signal source through an EasingFunction
/// and a pluggable MorphStrategy to drive UniversalPath morphing.
#[derive(Component)]
pub struct PathModulator {
    pub source: ModulatorSource,
    pub easing: EasingFunction,
    pub keyframe_a: UniversalPath,
    pub keyframe_b: UniversalPath,
    pub strategy: Box<dyn MorphStrategy>,
    pub current_t: f32,
}

impl Default for PathModulator {
    fn default() -> Self {
        Self {
            source: ModulatorSource::default(),
            easing: EasingFunction::Linear,
            keyframe_a: UniversalPath::default(),
            keyframe_b: UniversalPath::default(),
            strategy: Box::new(CycleModuloMorphStrategy),
            current_t: 0.0,
        }
    }
}

impl PathModulator {
    pub fn new(
        keyframe_a: UniversalPath,
        keyframe_b: UniversalPath,
        source: ModulatorSource,
        easing: EasingFunction,
    ) -> Self {
        Self {
            source,
            easing,
            keyframe_a,
            keyframe_b,
            strategy: Box::new(CycleModuloMorphStrategy),
            current_t: 0.0,
        }
    }

    pub fn with_strategy(mut self, strategy: impl MorphStrategy) -> Self {
        self.strategy = Box::new(strategy);
        self
    }

    /// Convenience constructor creating a timeline keyframe animation
    pub fn from_animation(
        keyframe_a: UniversalPath,
        keyframe_b: UniversalPath,
        duration_secs: f32,
        loop_mode: LoopMode,
    ) -> Self {
        Self {
            source: ModulatorSource::Timeline {
                duration_secs: duration_secs.max(0.001),
                elapsed_secs: 0.0,
                loop_mode,
            },
            easing: EasingFunction::Linear,
            keyframe_a,
            keyframe_b,
            strategy: Box::new(CycleModuloMorphStrategy),
            current_t: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_point_lerp() {
        let p1 = PathPoint::new(0.0, 10.0, 0, 100, 200, 3);
        let p2 = PathPoint::new(10.0, 30.0, 100, 200, 0, 7);

        let mid = p1.lerp(&p2, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-4);
        assert!((mid.y - 20.0).abs() < 1e-4);
        assert_eq!(mid.r, 50);
        assert_eq!(mid.g, 150);
        assert_eq!(mid.b, 100);
        assert_eq!(mid.dwell, 7); // t >= 0.5 takes p2 dwell
    }

    #[test]
    fn test_cycle_modulo_morph_strategy_multi_segment() {
        let seg_a1 = PathSegment::new(vec![
            PathPoint::new(0.0, 0.0, 255, 0, 0, 0),
            PathPoint::new(10.0, 0.0, 255, 0, 0, 0),
        ]);
        let seg_a2 = PathSegment::new(vec![
            PathPoint::new(0.0, 10.0, 0, 255, 0, 0),
        ]);
        let path_a = UniversalPath::from_segments(vec![seg_a1, seg_a2]);

        let seg_b1 = PathSegment::new(vec![
            PathPoint::new(100.0, 100.0, 0, 0, 255, 0),
        ]);
        let seg_b2 = PathSegment::new(vec![
            PathPoint::new(50.0, 50.0, 0, 0, 255, 0),
            PathPoint::new(60.0, 60.0, 0, 0, 255, 0),
        ]);
        let path_b = UniversalPath::from_segments(vec![seg_b1, seg_b2]);

        let mut output = UniversalPath::empty();
        let mut strategy = CycleModuloMorphStrategy;

        strategy.morph(&path_a, &path_b, 0.5, &mut output);

        assert_eq!(output.segments.len(), 2);
        assert_eq!(output.segments[0].points.len(), 2);
        assert_eq!(output.segments[1].points.len(), 2);

        // Segment 0, Point 0 lerp between (0,0) and (100,100)
        assert!((output.segments[0].points[0].x - 50.0).abs() < 1e-4);
        // Segment 0, Point 1 lerp between (10,0) and modulo (100,100)
        assert!((output.segments[0].points[1].x - 55.0).abs() < 1e-4);
    }

    #[test]
    fn test_modulator_timeline_loop_modes() {
        let mut source = ModulatorSource::Timeline {
            duration_secs: 1.0,
            elapsed_secs: 0.0,
            loop_mode: LoopMode::Repeat,
        };

        let t1 = source.advance_and_sample(0.25, 0.25);
        assert!((t1 - 0.25).abs() < 1e-4);

        let t2 = source.advance_and_sample(1.0, 1.25);
        assert!((t2 - 0.25).abs() < 1e-4); // wrapped around

        let mut ping_pong = ModulatorSource::Timeline {
            duration_secs: 1.0,
            elapsed_secs: 0.0,
            loop_mode: LoopMode::PingPong,
        };

        ping_pong.advance_and_sample(1.25, 1.25);
        let sample = ping_pong.sample(1.25);
        assert!((sample - 0.75).abs() < 1e-4); // 1.0 - 0.25 in reverse cycle
    }

    #[test]
    fn test_pluggable_path_modifier_pipeline() {
        #[derive(Clone, Default)]
        struct TestOffsetModifier {
            dx: f32,
            dy: f32,
        }

        impl PathModifier for TestOffsetModifier {
            fn name(&self) -> &'static str { "TestOffset" }
            fn type_id(&self) -> &'static str { "test_offset" }
            fn apply(&mut self, path: &mut UniversalPath, _ctx: &ModifierContext) {
                for seg in &mut path.segments {
                    for pt in &mut seg.points {
                        pt.x += self.dx;
                        pt.y += self.dy;
                    }
                }
            }
            fn clone_box(&self) -> Box<dyn PathModifier> {
                Box::new(self.clone())
            }
        }

        let mut path = UniversalPath::from_segment(PathSegment::new(vec![
            PathPoint::new(1.0, 2.0, 255, 255, 255, 0),
        ]));

        let mut pipeline = PathModifierPipeline::new()
            .with_modifier(TestOffsetModifier { dx: 10.0, dy: 20.0 });

        let ctx = ModifierContext {
            delta_secs: 0.016,
            elapsed_secs: 1.0,
            modulation_t: 1.0,
            audio_spectral: None,
        };

        pipeline.apply(&mut path, &ctx);

        assert!((path.segments[0].points[0].x - 11.0).abs() < 1e-4);
        assert!((path.segments[0].points[0].y - 22.0).abs() < 1e-4);
    }

    #[test]
    fn test_strategy_registry() {
        let registry = MorphStrategyRegistry::new();
        let strategy = registry.create("cycle_modulo");
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name(), "CycleModulo");
    }

    fn white(x: f32, y: f32) -> PathPoint {
        PathPoint::new(x, y, 255, 255, 255, 0)
    }

    fn is_dark(p: &PathPoint) -> bool {
        p.r == 0 && p.g == 0 && p.b == 0
    }

    fn ctx(modulation_t: f32) -> ModifierContext<'static> {
        ModifierContext { delta_secs: 0.02, elapsed_secs: 1.0, modulation_t, audio_spectral: None }
    }

    #[derive(Clone, Default)]
    struct Offset(f32);

    impl PathModifier for Offset {
        fn name(&self) -> &'static str { "Offset" }
        fn type_id(&self) -> &'static str { "offset" }
        fn apply(&mut self, path: &mut UniversalPath, _ctx: &ModifierContext) {
            path.segments.iter_mut().flat_map(|s| s.points.iter_mut()).for_each(|p| p.x += self.0);
        }
        fn clone_box(&self) -> Box<dyn PathModifier> { Box::new(self.clone()) }
    }

    #[derive(Clone, Default)]
    struct Scale(f32);

    impl PathModifier for Scale {
        fn name(&self) -> &'static str { "Scale" }
        fn type_id(&self) -> &'static str { "scale" }
        fn apply(&mut self, path: &mut UniversalPath, _ctx: &ModifierContext) {
            path.segments.iter_mut().flat_map(|s| s.points.iter_mut()).for_each(|p| p.x *= self.0);
        }
        fn clone_box(&self) -> Box<dyn PathModifier> { Box::new(self.clone()) }
    }

    // --- PathPoint ------------------------------------------------------------

    #[test]
    fn path_point_lerp_clamps_and_switches_dwell() {
        let a = PathPoint::new(0.0, 0.0, 0, 0, 0, 1);
        let b = PathPoint::new(10.0, 10.0, 200, 200, 200, 5);
        assert_eq!(a.lerp(&b, -1.0), a);
        assert_eq!(a.lerp(&b, 2.0), b);
        assert_eq!(a.lerp(&b, 0.49).dwell, 1);
        assert_eq!(a.lerp(&b, 0.5).dwell, 5);
    }

    #[test]
    fn path_point_color_conversion() {
        assert_eq!(PathPoint::color_to_rgb(Color::srgb(1.0, 0.0, 0.0)), (255, 0, 0));
        assert_eq!(PathPoint::color_to_rgb(Color::BLACK), (0, 0, 0));
        let p = PathPoint::from_vec2_color(Vec2::new(1.0, 2.0), Color::srgb(0.0, 1.0, 0.0), 4);
        assert_eq!(p, PathPoint::new(1.0, 2.0, 0, 255, 0, 4));
    }

    #[test]
    #[ignore = "BUG: color_to_rgb truncates instead of rounding; Color::WHITE (linear) converts to \
                0.99999994 sRGB and becomes 254, and mid-tones are biased down by one step"]
    fn color_to_rgb_rounds() {
        assert_eq!(PathPoint::color_to_rgb(Color::WHITE), (255, 255, 255));
        assert_eq!(PathPoint::color_to_rgb(Color::srgb(0.5, 0.5, 0.5)), (128, 128, 128));
    }

    #[test]
    fn path_point_serde_defaults() {
        let p: PathPoint = serde_json::from_str(r#"{"x":1.0,"y":2.0}"#).unwrap();
        assert_eq!(p, PathPoint::new(1.0, 2.0, 255, 255, 255, 0));
    }

    #[test]
    fn path_segment_serde_defaults() {
        let s: PathSegment = serde_json::from_str(r#"{"points":[{"x":0.0,"y":0.0}]}"#).unwrap();
        assert_eq!(s.line_style, LineStyle::Continuous);
        assert_eq!(s.hint, PathHint::General);
        assert_eq!(PathSegment::empty_text().hint, PathHint::Text);
    }

    #[test]
    fn universal_path_bincode_roundtrip() {
        let mut seg = PathSegment::polygon(&[Vec2::ZERO, Vec2::X, Vec2::ONE], Color::WHITE, 2);
        seg.line_style = LineStyle::Dashed;
        seg.hint = PathHint::Text;
        let path = UniversalPath::from_segment(seg);
        let bytes = bincode::serialize(&path).unwrap();
        let back: UniversalPath = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back, path);
    }

    // --- PathSegment constructors -----------------------------------------------

    #[test]
    fn segment_constructors() {
        let red = Color::srgb(1.0, 0.0, 0.0);
        let line = PathSegment::line(Vec2::ZERO, Vec2::new(3.0, 4.0), red, 2);
        assert_eq!(line.points, vec![PathPoint::new(0.0, 0.0, 255, 0, 0, 2), PathPoint::new(3.0, 4.0, 255, 0, 0, 2)]);

        let poly = PathSegment::polyline(&[Vec2::ZERO, Vec2::X, Vec2::ONE], Color::WHITE, 0);
        assert_eq!(poly.points.len(), 3);

        let gon = PathSegment::polygon(&[Vec2::ZERO, Vec2::X, Vec2::ONE], Color::WHITE, 0);
        assert_eq!(gon.points.len(), 4);
        assert_eq!(gon.points[0], gon.points[3]);

        assert!(PathSegment::polygon(&[], Color::WHITE, 0).points.is_empty());
    }

    #[test]
    fn segment_builder_and_push() {
        let built = PathSegment::builder()
            .point(0.0, 0.0, 1, 2, 3, 4)
            .add(1.0, 0.0, Color::WHITE, 0)
            .add_vec2(Vec2::ONE, Color::BLACK, 1)
            .build();
        assert_eq!(built.points.len(), 3);
        assert_eq!(built.points[0], PathPoint::new(0.0, 0.0, 1, 2, 3, 4));
        assert_eq!(built.points[2], PathPoint::new(1.0, 1.0, 0, 0, 0, 1));

        let mut seg = PathSegment::empty();
        seg.push(1.0, 2.0, Color::WHITE, 0);
        seg.push_vec2(Vec2::ONE, Color::WHITE, 0);
        seg.push_point(white(5.0, 5.0));
        assert_eq!(seg.points.len(), 3);
    }

    #[test]
    fn segment_from_lyon_path() {
        use lyon_path::math::point;
        let mut builder = Path::builder();
        builder.begin(point(0.0, 0.0));
        builder.line_to(point(1.0, 0.0));
        builder.quadratic_bezier_to(point(2.0, 1.0), point(3.0, 0.0));
        builder.cubic_bezier_to(point(4.0, 1.0), point(5.0, 1.0), point(6.0, 0.0));
        builder.end(false);
        let seg = PathSegment::from_lyon_path(&builder.build(), Color::WHITE, 1.0);

        // begin + line + (ctrl, to) + (ctrl1, ctrl2, to)
        let xs: Vec<f32> = seg.points.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn universal_path_flatten_spans_segments() {
        let path = UniversalPath::from_segments(vec![
            PathSegment::line(Vec2::ZERO, Vec2::X, Color::WHITE, 0),
            PathSegment::line(Vec2::Y, Vec2::ONE, Color::WHITE, 0),
        ]);
        assert_eq!(path.flatten(), vec![Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE]);
        assert!(UniversalPath::empty().flatten().is_empty());
    }

    // --- expand_line_style --------------------------------------------------------

    #[test]
    fn continuous_and_independent_are_unchanged() {
        let mut seg = PathSegment::polyline(&[Vec2::ZERO, Vec2::new(10.0, 0.0)], Color::WHITE, 0);
        assert_eq!(seg.expand_line_style(), seg.points);
        seg.line_style = LineStyle::Independent;
        assert_eq!(seg.expand_line_style(), seg.points);
    }

    #[test]
    fn short_segments_are_unchanged() {
        let mut seg = PathSegment::new(vec![white(0.0, 0.0)]);
        seg.line_style = LineStyle::Dashed;
        assert_eq!(seg.expand_line_style(), seg.points);
    }

    #[test]
    fn dashed_line_alternates_lit_and_dark() {
        let mut seg = PathSegment::new(vec![PathPoint::new(0.0, 0.0, 255, 255, 255, 2), PathPoint::new(10.0, 0.0, 255, 255, 255, 2)]);
        seg.line_style = LineStyle::Dashed;
        let pts = seg.expand_line_style();

        // 10m / 0.25m steps + closing point
        assert_eq!(pts.len(), 41);
        for p in &pts {
            let lit = (p.x % 2.0) < 1.2 - 1e-4;
            let dark = (p.x % 2.0) > 1.2 + 1e-4;
            if lit { assert!(!is_dark(p), "x = {} should be lit", p.x); }
            if dark { assert!(is_dark(p), "x = {} should be dark", p.x); }
        }
        assert!(pts.iter().all(|p| p.dwell == 2));
        assert!(pts.iter().all(|p| p.y == 0.0));
    }

    #[test]
    fn dotted_line_is_mostly_dark() {
        let mut seg = PathSegment::new(vec![white(0.0, 0.0), white(15.0, 0.0)]);
        seg.line_style = LineStyle::Dotted;
        let pts = seg.expand_line_style();
        let lit = pts.iter().filter(|p| !is_dark(p)).count();
        assert!(lit > 0 && lit * 4 < pts.len(), "lit {} of {}", lit, pts.len());
    }

    #[test]
    fn dashed_closed_loop_keeps_seam() {
        let mut seg = PathSegment::polygon(
            &[Vec2::ZERO, Vec2::new(3.0, 0.0), Vec2::new(3.0, 3.0), Vec2::new(0.0, 3.0)],
            Color::WHITE,
            0,
        );
        seg.line_style = LineStyle::Dashed;
        let pts = seg.expand_line_style();
        let (first, last) = (&pts[0], pts.last().unwrap());
        assert_eq!((first.x, first.y, first.r, first.g, first.b), (last.x, last.y, last.r, last.g, last.b));
    }

    #[test]
    fn dashed_handles_zero_length_edges() {
        let mut seg = PathSegment::new(vec![white(0.0, 0.0), white(0.0, 0.0), white(5.0, 0.0)]);
        seg.line_style = LineStyle::Dashed;
        let pts = seg.expand_line_style();
        // 1 point for the degenerate edge + 20 steps + closing point
        assert_eq!(pts.len(), 22);
        assert!(pts.iter().all(|p| p.x.is_finite()));
    }

    // --- EasingFunction / ModulatorSource -----------------------------------------

    #[test]
    fn easing_endpoints_and_midpoints() {
        for easing in [EasingFunction::Linear, EasingFunction::EaseInQuad, EasingFunction::EaseOutQuad, EasingFunction::EaseInOutQuad] {
            assert_eq!(easing.apply(0.0), 0.0, "{:?}", easing);
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-6, "{:?}", easing);
            assert_eq!(easing.apply(-5.0), easing.apply(0.0));
            assert_eq!(easing.apply(5.0), easing.apply(1.0));
        }
        assert!((EasingFunction::EaseInQuad.apply(0.5) - 0.25).abs() < 1e-6);
        assert!((EasingFunction::EaseOutQuad.apply(0.5) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn timeline_once_holds_at_end() {
        let mut source = ModulatorSource::Timeline { duration_secs: 2.0, elapsed_secs: 0.0, loop_mode: LoopMode::Once };
        assert!((source.advance_and_sample(1.0, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(source.advance_and_sample(5.0, 6.0), 1.0);
        assert_eq!(source.sample(100.0), 1.0);
    }

    #[test]
    fn timeline_ping_pong_turns_at_ends() {
        let mut source = ModulatorSource::Timeline { duration_secs: 1.0, elapsed_secs: 0.0, loop_mode: LoopMode::PingPong };
        assert!((source.advance_and_sample(1.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((source.advance_and_sample(0.5, 0.0) - 0.5).abs() < 1e-6);
        assert!(source.advance_and_sample(0.5, 0.0).abs() < 1e-6);
        assert!((source.advance_and_sample(0.25, 0.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn timeline_zero_duration_is_finite() {
        let mut source = ModulatorSource::Timeline { duration_secs: 0.0, elapsed_secs: 0.0, loop_mode: LoopMode::Repeat };
        let t = source.advance_and_sample(0.016, 0.016);
        assert!(t.is_finite() && (0.0..=1.0).contains(&t));
    }

    #[test]
    fn lfo_sources() {
        let sine = ModulatorSource::LfoSine { frequency_hz: 1.0 };
        assert!((sine.sample(0.0) - 0.5).abs() < 1e-6);
        assert!((sine.sample(0.25) - 1.0).abs() < 1e-6);
        assert!(sine.sample(0.75).abs() < 1e-6);

        let tri = ModulatorSource::LfoTriangle { frequency_hz: 2.0 };
        assert!(tri.sample(0.0).abs() < 1e-6);
        assert!((tri.sample(0.125) - 0.5).abs() < 1e-6);
        assert!((tri.sample(0.25) - 1.0).abs() < 1e-6);
        assert!((tri.sample(0.375) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn stateless_sources_ignore_delta_and_clamp() {
        let mut manual = ModulatorSource::Manual(1.5);
        assert_eq!(manual.advance_and_sample(10.0, 10.0), 1.0);
        assert_eq!(ModulatorSource::Ratio { value: -0.5 }.sample(0.0), 0.0);
        assert_eq!(ModulatorSource::GamepadTrigger { value: 0.3 }.sample(99.0), 0.3);

        // advance_and_sample on an LFO samples at the total elapsed time, not the delta
        let mut lfo = ModulatorSource::LfoSine { frequency_hz: 1.0 };
        assert!((lfo.advance_and_sample(0.0, 0.25) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn modulator_constructors() {
        let m = PathModulator::from_animation(UniversalPath::empty(), UniversalPath::empty(), 0.0, LoopMode::Once);
        match m.source {
            ModulatorSource::Timeline { duration_secs, .. } => assert!(duration_secs >= 0.001),
            other => panic!("unexpected source {:?}", other),
        }
        assert_eq!(m.strategy.name(), "CycleModulo");
        assert_eq!(PathModulator::default().strategy.name(), "CycleModulo");
    }

    // --- CycleModuloMorphStrategy -------------------------------------------------

    fn one_seg(points: Vec<PathPoint>) -> UniversalPath {
        UniversalPath::from_segment(PathSegment::new(points))
    }

    #[test]
    fn morph_endpoints_match_keyframes() {
        let a = one_seg(vec![white(0.0, 0.0), white(10.0, 0.0)]);
        let b = one_seg(vec![white(0.0, 10.0), white(10.0, 10.0)]);
        let mut out = UniversalPath::empty();
        let mut strategy = CycleModuloMorphStrategy;
        strategy.morph(&a, &b, 0.0, &mut out);
        assert_eq!(out.segments[0].points, a.segments[0].points);
        strategy.morph(&a, &b, 1.0, &mut out);
        assert_eq!(out.segments[0].points, b.segments[0].points);
    }

    #[test]
    fn morph_with_empty_keyframe_leaves_output_untouched() {
        let a = one_seg(vec![white(0.0, 0.0)]);
        let mut out = one_seg(vec![white(42.0, 42.0)]);
        let before = out.clone();
        CycleModuloMorphStrategy.morph(&a, &UniversalPath::empty(), 0.5, &mut out);
        assert_eq!(out, before);
        CycleModuloMorphStrategy.morph(&UniversalPath::empty(), &a, 0.5, &mut out);
        assert_eq!(out, before);
    }

    #[test]
    fn morph_copies_non_empty_side_when_other_segment_empty() {
        let a = one_seg(vec![]);
        let b = one_seg(vec![white(1.0, 1.0), white(2.0, 2.0)]);
        let mut out = UniversalPath::empty();
        CycleModuloMorphStrategy.morph(&a, &b, 0.0, &mut out);
        assert_eq!(out.segments[0].points, b.segments[0].points);
    }

    #[test]
    fn morph_reuses_output_without_growing() {
        let a = one_seg(vec![white(0.0, 0.0), white(10.0, 0.0)]);
        let b = one_seg(vec![white(0.0, 10.0), white(10.0, 10.0)]);
        let mut out = UniversalPath::empty();
        for i in 0..10 {
            CycleModuloMorphStrategy.morph(&a, &b, i as f32 / 10.0, &mut out);
        }
        assert_eq!(out.segments.len(), 1);
        assert_eq!(out.segments[0].points.len(), 2);
    }

    #[test]
    #[ignore = "BUG: CycleModulo only grows output.segments; segments beyond the keyframe count \
                (e.g. from the entity's initial path) are never removed and keep rendering"]
    fn morph_trims_stale_output_segments() {
        let a = one_seg(vec![white(0.0, 0.0)]);
        let b = one_seg(vec![white(1.0, 1.0)]);
        let mut out = UniversalPath::from_segments(vec![PathSegment::empty(), PathSegment::empty(), PathSegment::empty()]);
        CycleModuloMorphStrategy.morph(&a, &b, 0.5, &mut out);
        assert_eq!(out.segments.len(), 1);
    }

    #[test]
    #[ignore = "BUG: CycleModulo creates output segments with PathSegment::empty(), dropping the keyframes' \
                line_style and hint, so morphed text loses the text pipeline and dashed lines turn solid"]
    fn morph_preserves_segment_style_and_hint() {
        let mut seg = PathSegment::new(vec![white(0.0, 0.0), white(1.0, 0.0)]);
        seg.hint = PathHint::Text;
        seg.line_style = LineStyle::Dashed;
        let a = UniversalPath::from_segment(seg.clone());
        let b = UniversalPath::from_segment(seg);
        let mut out = UniversalPath::empty();
        CycleModuloMorphStrategy.morph(&a, &b, 0.5, &mut out);
        assert_eq!(out.segments[0].hint, PathHint::Text);
        assert_eq!(out.segments[0].line_style, LineStyle::Dashed);
    }

    // --- Pipeline / registries ----------------------------------------------------

    #[test]
    fn pipeline_applies_modifiers_in_order() {
        let base = one_seg(vec![white(1.0, 0.0)]);

        let mut path = base.clone();
        PathModifierPipeline::new().with_modifier(Offset(1.0)).with_modifier(Scale(10.0)).apply(&mut path, &ctx(1.0));
        assert_eq!(path.segments[0].points[0].x, 20.0);

        let mut path = base;
        let mut pipeline = PathModifierPipeline::new();
        pipeline.add(Scale(10.0));
        pipeline.add(Offset(1.0));
        pipeline.apply(&mut path, &ctx(1.0));
        assert_eq!(path.segments[0].points[0].x, 11.0);
    }

    #[test]
    fn disabled_pipeline_is_noop() {
        let mut path = one_seg(vec![white(1.0, 0.0)]);
        let mut pipeline = PathModifierPipeline::new().with_modifier(Offset(5.0));
        pipeline.enabled = false;
        pipeline.apply(&mut path, &ctx(1.0));
        assert_eq!(path.segments[0].points[0].x, 1.0);
    }

    #[test]
    fn modifier_registry_creates_registered_types() {
        let mut registry = PathModifierRegistry::new();
        assert!(registry.create("offset").is_none());
        registry.register::<Offset>();
        let modifier = registry.create("offset").unwrap();
        assert_eq!(modifier.name(), "Offset");
        assert!(registry.create("missing").is_none());
    }

    #[test]
    #[ignore = "BUG: MorphStrategyRegistry derives Default, so init_resource (used by PathNetworkPlugin) \
                yields an empty registry; only ::new() registers cycle_modulo"]
    fn default_strategy_registry_has_builtin() {
        assert!(MorphStrategyRegistry::default().create("cycle_modulo").is_some());
    }

    #[test]
    fn vector_frame_helpers() {
        let frame = VectorFrame::new(7, vec![AbstractPathData::new(UniversalPath::empty(), Vec3::ONE)]);
        assert_eq!(frame.len(), 1);
        assert!(!frame.is_empty());
        assert!(VectorFrame::default().is_empty());
    }
}
