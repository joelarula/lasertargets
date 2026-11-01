use bevy::prelude::*;
use common::path::{PathPoint, PathSegment, UniversalPath};
use ttf_parser::{Face, OutlineBuilder};

/// Configuration options for rasterizing vector laser text from TTF glyph outlines.
#[derive(Clone, Debug)]
pub struct LaserTextOptions {
    pub height: f32,
    pub letter_spacing: f32,
    pub corner_dwell: u8,
    pub endpoint_dwell: u8,
    pub curve_steps: usize,
    pub simplify_distance: f32,
    pub center_on_origin: bool,
    pub origin: Vec2,
    pub color: Color,
    pub corner_angle_deg: f32,
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let mid = Vec2::new((x1 + x2) * 0.5, (y1 + y2) * 0.5);
        self.commands
            .push(GlyphCommand::QuadTo(Vec2::new(x1, y1), mid));
        self.commands
            .push(GlyphCommand::QuadTo(Vec2::new(x2, y2), Vec2::new(x, y)));
    }

    fn close(&mut self) {
        self.commands.push(GlyphCommand::Close);
    }
}

/// Build laser path from TTF font outlines, including curve decomposition,
/// simplification, corner dwelling, and origin centering.
pub fn from_ttf_text(
    font_data: &[u8],
    text: &str,
    options: &LaserTextOptions,
) -> Result<UniversalPath, String> {
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

        let mut segment = PathSegment::empty_text();
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
