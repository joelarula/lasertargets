use bevy::prelude::*;
use lyon_path::{Path, PathEvent};
use serde::{Deserialize, Serialize};
use ttf_parser::{Face, OutlineBuilder};

#[derive(Clone, Debug)]
pub struct LaserTextOptions {
    pub origin: Vec2,
    pub height: f32,
    pub color: Color,
    pub letter_spacing: f32,
    pub curve_steps: usize,
    pub simplify_distance: f32,
    pub corner_angle_deg: f32,
    pub corner_dwell: u8,
    pub endpoint_dwell: u8,
    pub center_on_origin: bool,
}

impl Default for LaserTextOptions {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            height: 0.25,
            color: Color::WHITE,
            letter_spacing: 0.08,
            curve_steps: 8,
            simplify_distance: 0.002,
            corner_angle_deg: 145.0,
            corner_dwell: 3,
            endpoint_dwell: 4,
            center_on_origin: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum GlyphCommand {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo(Vec2, Vec2),
    Close,
}

#[derive(Default)]
struct GlyphOutlineBuilder {
    commands: Vec<GlyphCommand>,
}

impl OutlineBuilder for GlyphOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.commands.push(GlyphCommand::MoveTo(Vec2::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.commands.push(GlyphCommand::LineTo(Vec2::new(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.commands
            .push(GlyphCommand::QuadTo(Vec2::new(x1, y1), Vec2::new(x, y)));
    }

    fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, x: f32, y: f32) {
        self.commands.push(GlyphCommand::LineTo(Vec2::new(x, y)));
    }

    fn close(&mut self) {
        self.commands.push(GlyphCommand::Close);
    }
}

/// A single point in a path with color and dwell information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathPoint {
    pub x: f32,
    pub y: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
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
}

/// A segment of a path with simple point representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathSegment {
    pub points: Vec<PathPoint>,
    #[serde(default)]
    pub line_style: LineStyle,
}

impl PathSegment {
    pub fn new(points: Vec<PathPoint>) -> Self {
        Self {
            points,
            line_style: LineStyle::Continuous,
        }
    }
    
    pub fn empty() -> Self {
        Self {
            points: Vec::new(),
            line_style: LineStyle::Continuous,
        }
    }

    /// Expand line style (Continuous, Dashed, Dotted) into styled points with lit and blanked sections
    pub fn expand_line_style(&self) -> Vec<PathPoint> {
        if self.points.len() < 2 || self.line_style == LineStyle::Continuous {
            return self.points.clone();
        }

        let (dash_period, lit_length, step_size) = match self.line_style {
            LineStyle::Continuous => return self.points.clone(),
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

    /// Build text paths from TTF outlines, including basic simplification and dwell hints.
    pub fn from_ttf_text(
        font_data: &[u8],
        text: &str,
        options: &LaserTextOptions,
    ) -> Result<Self, String> {
        let face = Face::parse(font_data, 0).map_err(|e| format!("Failed to parse TTF: {e:?}"))?;
        let units = face.units_per_em() as f32;
        if units <= 0.0 {
            return Err("Invalid units_per_em in font".to_string());
        }

        let scale = options.height / units;
        let mut pen_x = 0.0_f32;
        let mut polylines: Vec<Vec<Vec2>> = Vec::new();

        for ch in text.chars() {
            if ch == ' ' {
                let space_advance = face
                    .glyph_index(' ')
                    .and_then(|id| face.glyph_hor_advance(id))
                    .map(|a| a as f32)
                    .unwrap_or(units * 0.5);
                pen_x += space_advance * scale + options.letter_spacing;
                continue;
            }

            let Some(glyph_id) = face.glyph_index(ch) else {
                pen_x += (units * 0.5) * scale + options.letter_spacing;
                continue;
            };

            let mut builder = GlyphOutlineBuilder::default();
            let _ = face.outline_glyph(glyph_id, &mut builder);
            let glyph_lines = commands_to_polylines(&builder.commands, options.curve_steps.max(1));

            for line in glyph_lines {
                let mut transformed = Vec::with_capacity(line.len());
                for p in line {
                    transformed.push(Vec2::new(
                        options.origin.x + pen_x + p.x * scale,
                        options.origin.y + p.y * scale,
                    ));
                }
                polylines.push(simplify_polyline(transformed, options.simplify_distance));
            }

            let advance = face.glyph_hor_advance(glyph_id).map(|a| a as f32).unwrap_or(units * 0.6);
            pen_x += advance * scale + options.letter_spacing;
        }

        if options.center_on_origin {
            center_polylines(&mut polylines, options.origin);
        }

        let (r, g, b) = PathPoint::color_to_rgb(options.color);
        let mut path = UniversalPath::new();
        for line in polylines {
            if line.len() < 2 {
                continue;
            }

            let mut segment = PathSegment::empty();
            for i in 0..line.len() {
                let prev = if i > 0 { Some(line[i - 1]) } else { None };
                let next = if i + 1 < line.len() { Some(line[i + 1]) } else { None };
                let dwell = point_dwell(i, line.len(), prev, line[i], next, options);
                segment.push_point(PathPoint::new(line[i].x, line[i].y, r, g, b, dwell));
            }

            path.add_segment(segment);
        }

        Ok(path)
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

fn commands_to_polylines(commands: &[GlyphCommand], curve_steps: usize) -> Vec<Vec<Vec2>> {
    let mut result = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();

    for cmd in commands {
        match *cmd {
            GlyphCommand::MoveTo(p) => {
                if !current.is_empty() {
                    result.push(current);
                    current = Vec::new();
                }
                current.push(p);
            }
            GlyphCommand::LineTo(p) => {
                current.push(p);
            }
            GlyphCommand::QuadTo(ctrl, to) => {
                if let Some(from) = current.last().copied() {
                    for step in 1..=curve_steps {
                        let t = step as f32 / curve_steps as f32;
                        let p = quadratic_bezier(from, ctrl, to, t);
                        current.push(p);
                    }
                }
            }
            GlyphCommand::Close => {
                if !current.is_empty() {
                    let first = current[0];
                    if current.last().copied() != Some(first) {
                        current.push(first);
                    }
                    result.push(current);
                    current = Vec::new();
                }
            }
        }
    }

    if !current.is_empty() {
        result.push(current);
    }

    result
}

fn quadratic_bezier(a: Vec2, b: Vec2, c: Vec2, t: f32) -> Vec2 {
    let one_minus_t = 1.0 - t;
    one_minus_t * one_minus_t * a + 2.0 * one_minus_t * t * b + t * t * c
}

fn simplify_polyline(points: Vec<Vec2>, min_distance: f32) -> Vec<Vec2> {
    if points.len() <= 2 || min_distance <= 0.0 {
        return points;
    }

    let mut out = Vec::with_capacity(points.len());
    out.push(points[0]);
    let mut last = points[0];
    for p in points.iter().skip(1) {
        if last.distance(*p) >= min_distance {
            out.push(*p);
            last = *p;
        }
    }

    if out.len() == 1 {
        out.push(*points.last().unwrap_or(&points[0]));
    }

    out
}

fn center_polylines(polylines: &mut [Vec<Vec2>], center: Vec2) {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for line in polylines.iter() {
        for p in line {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
    }

    if min_x == f32::MAX {
        return;
    }

    let offset = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5) - center;
    for line in polylines.iter_mut() {
        for p in line.iter_mut() {
            *p -= offset;
        }
    }
}

fn point_dwell(
    index: usize,
    len: usize,
    prev: Option<Vec2>,
    current: Vec2,
    next: Option<Vec2>,
    options: &LaserTextOptions,
) -> u8 {
    if index == 0 || index + 1 == len {
        return options.endpoint_dwell;
    }

    let (Some(a), Some(c)) = (prev, next) else {
        return 0;
    };

    let v1 = (a - current).normalize_or_zero();
    let v2 = (c - current).normalize_or_zero();
    if v1 == Vec2::ZERO || v2 == Vec2::ZERO {
        return 0;
    }

    let dot = v1.dot(v2).clamp(-1.0, 1.0);
    let angle = dot.acos().to_degrees();
    if angle <= options.corner_angle_deg {
        options.corner_dwell
    } else {
        0
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
