use bevy::prelude::*;
use lyon_tessellation::path::{Path, PathEvent};
use serde::{Deserialize, Serialize};

/// A segment of a path with its own rendering properties
#[derive(Clone, Debug,Serialize, Deserialize)]
pub struct PathSegment {
    pub path: Path,
    pub color: Color,
    pub line_width: f32,
}

impl PathSegment {
    pub fn new(path: Path, color: Color, line_width: f32) -> Self {
        Self {
            path,
            color,
            line_width,
        }
    }
}

/// Universal path representation containing multiple segments
#[derive(Component, Clone, Debug, Serialize, Deserialize)]
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

    pub fn from_path(path: Path, color: Color, line_width: f32) -> Self {
        Self {
            segments: vec![PathSegment::new(path, color, line_width)],
        }
    }

    pub fn add_segment(&mut self, segment: PathSegment) {
        self.segments.push(segment);
    }

    pub fn add_path(&mut self, path: Path, color: Color, line_width: f32) {
        self.segments
            .push(PathSegment::new(path, color, line_width));
    }

    /// Create a circle path
    pub fn circle(center: Vec2, radius: f32, color: Color) -> Self {
        use lyon_tessellation::math::point;
        let mut builder = Path::builder();

        // Create circle with line segments
        let segments = 64;
        let mut started = false;
        for i in 0..=segments {
            let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();

            if !started {
                builder.begin(point(x, y));
                started = true;
            } else {
                builder.line_to(point(x, y));
            }
        }
        builder.end(true);

        Self {
            segments: vec![PathSegment::new(builder.build(), color, 1.0)],
        }
    }

    /// Create a rectangle path
    pub fn rectangle(top_left: Vec2, size: Vec2, color: Color) -> Self {
        use lyon_tessellation::math::point;
        let mut builder = Path::builder().with_svg();
        builder.move_to(point(top_left.x, top_left.y));
        builder.line_to(point(top_left.x + size.x, top_left.y));
        builder.line_to(point(top_left.x + size.x, top_left.y + size.y));
        builder.line_to(point(top_left.x, top_left.y + size.y));
        builder.close();

        Self {
            segments: vec![PathSegment::new(builder.build(), color, 1.0)],
        }
    }

    /// Flatten path to line segments for gizmo rendering
    pub fn flatten(&self, tolerance: f32) -> Vec<Vec2> {
        let mut points = Vec::new();

        for segment in &self.segments {
            for event in segment.path.iter() {
                match event {
                    PathEvent::Begin { at } => {
                        points.push(Vec2::new(at.x, at.y));
                    }
                    PathEvent::Line { to, .. } => {
                        points.push(Vec2::new(to.x, to.y));
                    }
                    PathEvent::Quadratic { ctrl, to, .. } => {
                        // Sample quadratic curve
                        let from = points.last().copied().unwrap_or(Vec2::ZERO);
                        let control = Vec2::new(ctrl.x, ctrl.y);
                        let end = Vec2::new(to.x, to.y);
                        let samples = Self::sample_quadratic(from, control, end, tolerance);
                        points.extend(samples);
                    }
                    PathEvent::Cubic {
                        ctrl1, ctrl2, to, ..
                    } => {
                        // Sample cubic curve
                        let from = points.last().copied().unwrap_or(Vec2::ZERO);
                        let c1 = Vec2::new(ctrl1.x, ctrl1.y);
                        let c2 = Vec2::new(ctrl2.x, ctrl2.y);
                        let end = Vec2::new(to.x, to.y);
                        let samples = Self::sample_cubic(from, c1, c2, end, tolerance);
                        points.extend(samples);
                    }
                    PathEvent::End { close, .. } => {
                        if close && !points.is_empty() {
                            points.push(points[0]);
                        }
                    }
                }
            }
        }

        points
    }

    fn sample_quadratic(start: Vec2, control: Vec2, end: Vec2, tolerance: f32) -> Vec<Vec2> {
        let mut points = Vec::new();
        let steps = ((start.distance(control) + control.distance(end)) / tolerance)
            .ceil()
            .max(2.0) as usize;

        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            let point = start * mt * mt + control * 2.0 * mt * t + end * t * t;
            points.push(point);
        }
        points
    }

    fn sample_cubic(start: Vec2, c1: Vec2, c2: Vec2, end: Vec2, tolerance: f32) -> Vec<Vec2> {
        let mut points = Vec::new();
        let steps = ((start.distance(c1) + c1.distance(c2) + c2.distance(end)) / tolerance)
            .ceil()
            .max(2.0) as usize;

        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let t2 = t * t;
            let t3 = t2 * t;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let mt3 = mt2 * mt;

            let point = start * mt3 + c1 * 3.0 * mt2 * t + c2 * 3.0 * mt * t2 + end * t3;
            points.push(point);
        }
        points
    }

    /// Draw path using gizmos
    pub fn draw_with_gizmos(
        &self,
        gizmos: &mut Gizmos,
        transform: &GlobalTransform,
        tolerance: f32,
    ) {
        for segment in &self.segments {
            let mut points = Vec::new();
            for event in segment.path.iter() {
                match event {
                    PathEvent::Begin { at } => {
                        points.push(Vec2::new(at.x, at.y));
                    }
                    PathEvent::Line { to, .. } => {
                        points.push(Vec2::new(to.x, to.y));
                    }
                    PathEvent::Quadratic { ctrl, to, .. } => {
                        let from = points.last().copied().unwrap_or(Vec2::ZERO);
                        let control = Vec2::new(ctrl.x, ctrl.y);
                        let end = Vec2::new(to.x, to.y);
                        let samples = Self::sample_quadratic(from, control, end, tolerance);
                        points.extend(samples);
                    }
                    PathEvent::Cubic {
                        ctrl1, ctrl2, to, ..
                    } => {
                        let from = points.last().copied().unwrap_or(Vec2::ZERO);
                        let c1 = Vec2::new(ctrl1.x, ctrl1.y);
                        let c2 = Vec2::new(ctrl2.x, ctrl2.y);
                        let end = Vec2::new(to.x, to.y);
                        let samples = Self::sample_cubic(from, c1, c2, end, tolerance);
                        points.extend(samples);
                    }
                    PathEvent::End { close, .. } => {
                        if close && !points.is_empty() {
                            points.push(points[0]);
                        }
                    }
                }
            }
            let world_points: Vec<Vec3> = points
                .iter()
                .map(|p| transform.transform_point(Vec3::new(p.x, p.y, 0.0)))
                .collect();
            if world_points.len() >= 2 {
                gizmos.linestrip(world_points, segment.color);
            }
        }
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
