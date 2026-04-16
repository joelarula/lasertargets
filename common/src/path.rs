use bevy::prelude::*;
use lyon_path::{Path, PathEvent};
use serde::{Deserialize, Serialize};

/// A single point in a path with color and dwell information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f32,
    pub y: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub dwell: u8, // 0 = normal, 1-7 = dwell count
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

/// A segment of a path with simple point representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathSegment {
    pub points: Vec<PathPoint>,
}

impl PathSegment {
    pub fn new(points: Vec<PathPoint>) -> Self {
        Self { points }
    }
    
    pub fn empty() -> Self {
        Self { points: Vec::new() }
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
        }
    }
    
    /// Create multiple connected line segments (polyline)
    pub fn polyline(points: &[Vec2], color: Color, dwell: u8) -> Self {
        let (r, g, b) = PathPoint::color_to_rgb(color);
        let path_points = points
            .iter()
            .map(|p| PathPoint::new(p.x, p.y, r, g, b, dwell))
            .collect();
        Self { points: path_points }
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
        
        Self { points: path_points }
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
        
        Self { points }
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
            segments: vec![PathSegment::from_lyon_path(&builder.build(), color, 1.0)],
        }
    }

    /// Create a balloon path shape (circle for now, can be enhanced later)
    pub fn balloon(center: Vec2, radius: f32, color: Color) -> Self {
        // Start with a circle; can be changed to a teardrop/balloon shape later
        Self::circle(center, radius, color)
    }

    /// Create a diamond (rotated square) path — a square rotated 45°
    pub fn diamond(center: Vec2, half_size: f32, color: Color) -> Self {
        let top    = Vec2::new(center.x,             center.y + half_size);
        let right  = Vec2::new(center.x + half_size, center.y);
        let bottom = Vec2::new(center.x,             center.y - half_size);
        let left   = Vec2::new(center.x - half_size, center.y);
        Self::from_segment(PathSegment::polygon(&[top, right, bottom, left], color, 0))
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
            segments: vec![PathSegment::from_lyon_path(&builder.build(), color, 1.0)],
        }
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
