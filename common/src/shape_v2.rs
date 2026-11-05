//! Shape language v2: abstract shapes in metres, described as strokes.
//!
//! A shape carries geometry, colour and intent only. Blank points, stroke lead-ins and laser
//! timing are added later by the laser transform. See
//! `.agents/skills/laser-path-prep/DESIGN_V2.md`.
//!
//! v1 templates ([`ShapeTemplate`]) are converted with [`Shape::from_v1`].

use crate::path::{LineStyle, PathPoint, PathSegment, UniversalPath};
use crate::shapes::ShapeTemplate;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 2;

/// Point rate that v1 dwell counts were tuned for, used to turn them into milliseconds
pub const V1_REFERENCE_PPS: f32 = 30000.0;

const WHITE: [u8; 3] = [255, 255, 255];
const BLANK: [u8; 3] = [0, 0, 0];

/// Built-in v2 templates, embedded at compile time
const BUILTIN_JSON: [&str; 10] = [
    include_str!("../../assets/shapes/templates/v2/box.json"),
    include_str!("../../assets/shapes/templates/v2/triangle.json"),
    include_str!("../../assets/shapes/templates/v2/diamond.json"),
    include_str!("../../assets/shapes/templates/v2/star.json"),
    include_str!("../../assets/shapes/templates/v2/circle.json"),
    include_str!("../../assets/shapes/templates/v2/crosshair.json"),
    include_str!("../../assets/shapes/templates/v2/target.json"),
    include_str!("../../assets/shapes/templates/v2/cable_test.json"),
    include_str!("../../assets/shapes/templates/v2/dashed_box.json"),
    include_str!("../../assets/shapes/templates/v2/dotted_circle.json"),
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub author: String,
    pub version: u32,
    pub strokes: Vec<Stroke>,
}

/// One continuous line drawn without switching the laser off. One point is a dot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    /// Joins the last point back to the first; the first point is not repeated
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub closed: bool,
    #[serde(default = "white")]
    pub color: [u8; 3],
    #[serde(default, skip_serializing_if = "StrokeStyle::is_solid")]
    pub style: StrokeStyle,
    pub points: Vec<StrokePoint>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum StrokeStyle {
    #[default]
    Solid,
    Dashed { dash_m: f32, gap_m: f32 },
    Dotted { spacing_m: f32 },
}

impl StrokeStyle {
    fn is_solid(&self) -> bool {
        *self == Self::Solid
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrokePoint {
    /// Metres in the shape's own frame
    pub x: f32,
    pub y: f32,
    /// Colour of the edge that starts at this point; the stroke colour when not set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 3]>,
    /// Not set: the turn angle decides. 0: forced sharp corner. > 0: rounded with this radius
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radius_m: Option<f32>,
    /// Hold override; replaces the hold the laser stage would compute
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dwell_ms: Option<f32>,
}

impl StrokePoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y, ..Default::default() }
    }
}

fn white() -> [u8; 3] {
    WHITE
}

impl Shape {
    /// Parses and validates a v2 shape.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let shape: Shape = serde_json::from_str(json).map_err(|e| e.to_string())?;
        shape.validate()?;
        Ok(shape)
    }

    /// All built-in v2 templates.
    pub fn builtin() -> Vec<Shape> {
        BUILTIN_JSON
            .iter()
            .map(|json| Shape::from_json(json).expect("built-in v2 template is valid"))
            .collect()
    }

    /// Checks the v2 rules: no blank colours, closed strokes do not repeat their first point,
    /// physical values are finite and positive where required.
    pub fn validate(&self) -> Result<(), String> {
        let err = |stroke: usize, msg: &str| Err(format!("{}: stroke {}: {}", self.name, stroke, msg));

        if self.version != VERSION {
            return Err(format!("{}: version {} is not {}", self.name, self.version, VERSION));
        }
        if self.strokes.is_empty() {
            return Err(format!("{}: no strokes", self.name));
        }
        for (i, stroke) in self.strokes.iter().enumerate() {
            let points = &stroke.points;
            if points.is_empty() {
                return err(i, "no points");
            }
            if stroke.closed {
                if points.len() < 3 {
                    return err(i, "a closed stroke needs at least 3 points");
                }
                let (first, last) = (&points[0], &points[points.len() - 1]);
                if first.x == last.x && first.y == last.y {
                    return err(i, "closed stroke repeats its first point; drop the last point");
                }
            }
            if stroke.color == BLANK || points.iter().any(|p| p.color == Some(BLANK)) {
                return err(i, "colour [0, 0, 0] is blank; blanking is added by the laser stage");
            }
            if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                return err(i, "coordinates must be finite");
            }
            if points.iter().any(|p| p.corner_radius_m.is_some_and(|r| !(r >= 0.0))) {
                return err(i, "corner_radius_m must be 0 or more");
            }
            if points.iter().any(|p| p.dwell_ms.is_some_and(|d| !(d >= 0.0))) {
                return err(i, "dwell_ms must be 0 or more");
            }
            let lengths_ok = match stroke.style {
                StrokeStyle::Solid => true,
                StrokeStyle::Dashed { dash_m, gap_m } => dash_m > 0.0 && gap_m > 0.0,
                StrokeStyle::Dotted { spacing_m } => spacing_m > 0.0,
            };
            if !lengths_ok {
                return err(i, "dash_m, gap_m and spacing_m must be more than 0");
            }
        }
        Ok(())
    }

    /// Path for today's rendering pipeline, until v2 placement (stage 2) is built: one path
    /// segment per stroke, closed strokes end with a copy of their first point, `dwell_ms` becomes
    /// a dwell count at [`V1_REFERENCE_PPS`].
    ///
    /// Not supported yet: rounded corners (`corner_radius_m > 0` is drawn sharp) and the v2
    /// dash/dot lengths (dashed and dotted strokes use the path's fixed line styles).
    pub fn to_universal_path(&self) -> UniversalPath {
        let segments = self
            .strokes
            .iter()
            .map(|stroke| {
                let mut points: Vec<PathPoint> = stroke
                    .points
                    .iter()
                    .map(|p| {
                        let [r, g, b] = p.color.unwrap_or(stroke.color);
                        let dwell = p.dwell_ms.map_or(0, |ms| (ms / 1000.0 * V1_REFERENCE_PPS).round().clamp(0.0, 255.0) as u8);
                        PathPoint::new(p.x, p.y, r, g, b, dwell)
                    })
                    .collect();
                if stroke.closed {
                    // Closing copy: ends the last edge; the seam hold stays on the first point
                    let mut closing = points[points.len() - 1].clone();
                    closing.x = points[0].x;
                    closing.y = points[0].y;
                    closing.dwell = 0;
                    points.push(closing);
                }
                let mut segment = PathSegment::new(points);
                segment.line_style = match stroke.style {
                    StrokeStyle::Solid => LineStyle::Continuous,
                    StrokeStyle::Dashed { .. } => LineStyle::Dashed,
                    StrokeStyle::Dotted { .. } => LineStyle::Dotted,
                };
                segment
            })
            .collect();
        UniversalPath { segments }
    }

    /// Converts a v1 template. v1 coordinates are normalised (-1..1), so `size_m` gives the
    /// physical width of that range.
    ///
    /// - an edge is lit when the point it starts from is lit (the v1 colour rule); runs of lit
    ///   edges become strokes, a lit point with no lit edge becomes a dot
    /// - repeated positions inside a stroke are merged
    /// - a stroke ending where it started becomes `closed`
    /// - v1 dwell counts of lit points become `dwell_ms` at [`V1_REFERENCE_PPS`]
    /// - v1 dashed/dotted become the same metre values they used (1.2 m dashes with 0.8 m gaps,
    ///   dots every 1.5 m)
    pub fn from_v1(template: &ShapeTemplate, size_m: f32) -> Shape {
        let scale = size_m / 2.0;
        let style = match template.line_style {
            LineStyle::Continuous | LineStyle::Independent => StrokeStyle::Solid,
            LineStyle::Dashed => StrokeStyle::Dashed { dash_m: 1.2, gap_m: 0.8 },
            LineStyle::Dotted => StrokeStyle::Dotted { spacing_m: 1.5 },
        };
        let points = &template.points;
        let is_lit = |i: usize| (points[i].r, points[i].g, points[i].b) != (0, 0, 0);
        let edge_lit = |i: usize| i + 1 < points.len() && is_lit(i);

        let mut runs: Vec<Vec<usize>> = Vec::new();
        let mut i = 0;
        while i < points.len() {
            if edge_lit(i) {
                let mut run = vec![i];
                while edge_lit(i) {
                    i += 1;
                    run.push(i);
                }
                runs.push(run);
            } else {
                let incoming_lit = i > 0 && edge_lit(i - 1);
                if is_lit(i) && !incoming_lit {
                    runs.push(vec![i]);
                }
                i += 1;
            }
        }

        let strokes = runs
            .into_iter()
            .map(|run| {
                let color = {
                    let p = &points[run[0]];
                    [p.r, p.g, p.b]
                };
                let mut stroke_points: Vec<StrokePoint> = Vec::new();
                for &index in &run {
                    let p = &points[index];
                    let dwell_ms = (is_lit(index) && p.dwell > 0).then(|| p.dwell as f32 / V1_REFERENCE_PPS * 1000.0);
                    let position = (p.x * scale, p.y * scale);
                    match stroke_points.last_mut() {
                        Some(last) if (last.x, last.y) == position => {
                            // Merge a repeated position, keeping the longest lit hold
                            last.dwell_ms = match (last.dwell_ms, dwell_ms) {
                                (Some(a), Some(b)) => Some(a.max(b)),
                                (a, b) => a.or(b),
                            };
                        }
                        _ => {
                            // The point's colour is the colour of the edge it starts
                            let edge_color = [p.r, p.g, p.b];
                            let color = (is_lit(index) && edge_color != color).then_some(edge_color);
                            stroke_points.push(StrokePoint { x: position.0, y: position.1, color, corner_radius_m: None, dwell_ms });
                        }
                    }
                }
                let closed = stroke_points.len() > 3 && {
                    let (first, last) = (&stroke_points[0], &stroke_points[stroke_points.len() - 1]);
                    (first.x - last.x).abs() < 1e-4 * scale.max(1e-6) && (first.y - last.y).abs() < 1e-4 * scale.max(1e-6)
                };
                if closed {
                    // The closing point's hold belongs to the seam corner
                    let last = stroke_points.pop().unwrap();
                    stroke_points[0].dwell_ms = match (stroke_points[0].dwell_ms, last.dwell_ms) {
                        (Some(a), Some(b)) => Some(a.max(b)),
                        (a, b) => a.or(b),
                    };
                }
                Stroke { closed, color, style, points: stroke_points }
            })
            .collect();

        Shape {
            name: template.name.clone(),
            description: template.description.clone(),
            tags: template.tags.clone(),
            author: template.author.clone(),
            version: VERSION,
            strokes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v1(name: &str) -> ShapeTemplate {
        let json = match name {
            "box" => include_str!("../../assets/shapes/templates/box.json"),
            "crosshair" => include_str!("../../assets/shapes/templates/crosshair.json"),
            "target" => include_str!("../../assets/shapes/templates/target.json"),
            "star" => include_str!("../../assets/shapes/templates/star.json"),
            "circle" => include_str!("../../assets/shapes/templates/circle.json"),
            _ => unreachable!(),
        };
        ShapeTemplate::from_json(json).unwrap()
    }

    fn shape(strokes_json: &str) -> Result<Shape, String> {
        Shape::from_json(&format!(r#"{{"name": "t", "version": 2, "strokes": {}}}"#, strokes_json))
    }

    #[test]
    fn builtin_templates_are_valid() {
        let shapes = Shape::builtin();
        assert_eq!(shapes.len(), BUILTIN_JSON.len());
        let names: Vec<&str> = shapes.iter().map(|s| s.name.as_str()).collect();
        for name in ["box", "triangle", "diamond", "star", "circle", "crosshair", "target", "cable_test", "dashed_box", "dotted_circle"] {
            assert!(names.contains(&name), "{} missing", name);
        }
    }

    #[test]
    fn builtin_templates_roundtrip() {
        for shape in Shape::builtin() {
            let json = serde_json::to_string(&shape).unwrap();
            assert_eq!(Shape::from_json(&json).unwrap(), shape, "{}", shape.name);
        }
    }

    #[test]
    fn defaults_and_styles_parse() {
        let s = shape(r#"[{"points": [{"x": 0, "y": 0}]},
                          {"style": "solid", "points": [{"x": 0, "y": 0}, {"x": 1, "y": 0}]},
                          {"style": {"dashed": {"dash_m": 0.1, "gap_m": 0.05}}, "points": [{"x": 0, "y": 0}, {"x": 1, "y": 0}]},
                          {"style": {"dotted": {"spacing_m": 0.05}}, "points": [{"x": 0, "y": 0}, {"x": 1, "y": 0}]}]"#)
            .unwrap();
        assert_eq!(s.strokes[0].color, WHITE);
        assert!(!s.strokes[0].closed);
        assert_eq!(s.strokes[1].style, StrokeStyle::Solid);
        assert_eq!(s.strokes[2].style, StrokeStyle::Dashed { dash_m: 0.1, gap_m: 0.05 });
        assert_eq!(s.strokes[3].style, StrokeStyle::Dotted { spacing_m: 0.05 });
    }

    #[test]
    fn rule_violations_are_rejected() {
        let square = r#"{"x": 0, "y": 0}, {"x": 1, "y": 0}, {"x": 1, "y": 1}"#;
        let cases = [
            (format!(r#"[{{"closed": true, "points": [{square}, {{"x": 0, "y": 0}}]}}]"#), "repeats its first point"),
            (format!(r#"[{{"color": [0, 0, 0], "points": [{square}]}}]"#), "blank"),
            (r#"[{"points": [{"x": 0, "y": 0, "color": [0, 0, 0]}]}]"#.to_string(), "blank"),
            (r#"[{"closed": true, "points": [{"x": 0, "y": 0}, {"x": 1, "y": 0}]}]"#.to_string(), "at least 3"),
            (r#"[{"points": []}]"#.to_string(), "no points"),
            (r#"[]"#.to_string(), "no strokes"),
            (r#"[{"points": [{"x": 0, "y": 0, "corner_radius_m": -1}]}]"#.to_string(), "corner_radius_m"),
            (r#"[{"points": [{"x": 0, "y": 0, "dwell_ms": -1}]}]"#.to_string(), "dwell_ms"),
            (r#"[{"style": {"dashed": {"dash_m": 0, "gap_m": 0.1}}, "points": [{"x": 0, "y": 0}]}]"#.to_string(), "more than 0"),
            // Typos are caught instead of silently ignored
            (r#"[{"points": [{"x": 0, "y": 0, "corner_radius": 0}]}]"#.to_string(), "unknown field"),
        ];
        for (json, expected) in cases {
            let err = shape(&json).unwrap_err();
            assert!(err.contains(expected), "{} -> {}", json, err);
        }
        assert!(Shape::from_json(r#"{"name": "t", "version": 1, "strokes": [{"points": [{"x": 0, "y": 0}]}]}"#)
            .unwrap_err()
            .contains("version"));
    }

    #[test]
    fn universal_path_has_one_segment_per_stroke_and_closes_loops() {
        let target = Shape::builtin().into_iter().find(|s| s.name == "target").unwrap();
        let path = target.to_universal_path();
        assert_eq!(path.segments.len(), 3);
        let ring = &path.segments[0].points;
        assert_eq!(ring.len(), 33, "32 ring points plus the closing copy");
        assert_eq!((ring[0].x, ring[0].y), (ring[32].x, ring[32].y));
        assert_eq!((ring[0].r, ring[0].g, ring[0].b), (255, 0, 0));
        assert_eq!(path.segments[2].points.len(), 1, "centre dot");

        let cable = Shape::builtin().into_iter().find(|s| s.name == "cable_test").unwrap();
        let border = &cable.to_universal_path().segments[0].points;
        // Per-point colours: one per side
        assert_eq!((border[1].r, border[1].g, border[1].b), (0, 255, 0));
    }

    #[test]
    fn v1_box_becomes_closed_stroke_with_dwell() {
        let s = Shape::from_v1(&v1("box"), 0.5);
        assert_eq!(s.strokes.len(), 1);
        let stroke = &s.strokes[0];
        assert!(stroke.closed);
        assert_eq!(stroke.points.len(), 4);
        assert_eq!((stroke.points[0].x, stroke.points[0].y), (-0.25, 0.25));
        // v1 dwell 4 at 30000 PPS
        assert!(stroke.points.iter().all(|p| p.dwell_ms.is_some_and(|d| (d - 4.0 / 30.0).abs() < 1e-6)));
        s.validate().unwrap();
    }

    #[test]
    fn v1_crosshair_blank_points_split_into_two_strokes() {
        let s = Shape::from_v1(&v1("crosshair"), 0.5);
        assert_eq!(s.strokes.len(), 2);
        for stroke in &s.strokes {
            assert!(!stroke.closed);
            assert_eq!(stroke.points.len(), 2);
            assert_eq!(stroke.color, WHITE);
        }
        assert_eq!((s.strokes[0].points[0].x, s.strokes[0].points[1].x), (-0.25, 0.25));
        s.validate().unwrap();
    }

    #[test]
    fn v1_target_gives_ring_inner_shape_and_dot() {
        let s = Shape::from_v1(&v1("target"), 2.0);
        assert_eq!(s.strokes.len(), 3);
        assert!(s.strokes[0].closed && s.strokes[0].points.len() == 8);
        assert_eq!(s.strokes[0].color, [255, 0, 0]);
        assert!(s.strokes[1].closed && s.strokes[1].points.len() == 4);
        assert_eq!(s.strokes[1].color, WHITE);
        let dot = &s.strokes[2];
        assert_eq!(dot.points.len(), 1);
        assert_eq!((dot.points[0].x, dot.points[0].y), (0.0, 0.0));
        assert!(dot.points[0].dwell_ms.is_some());
        s.validate().unwrap();
    }

    #[test]
    fn all_v1_builtins_convert_to_valid_v2() {
        for name in ["box", "crosshair", "target", "star", "circle"] {
            Shape::from_v1(&v1(name), 1.0).validate().unwrap();
        }
    }
}

#[cfg(test)]
mod v1_collection_tests {
    use super::*;

    /// Every v1 file in the shape collection converts to a valid v2 shape
    #[test]
    fn unsorted_v1_collection_converts() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/shapes/unsorted");
        let mut converted = 0;
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let Ok(template) = ShapeTemplate::from_json(&std::fs::read_to_string(&path).unwrap()) else { continue };
            let shape = Shape::from_v1(&template, 1.0);
            if shape.strokes.is_empty() {
                continue; // nothing lit in the v1 file
            }
            match shape.validate() {
                Ok(()) => converted += 1,
                Err(e) => failures.push(format!("{}: {}", path.display(), e)),
            }
        }
        assert!(converted > 0);
        assert!(failures.is_empty(), "{} of {} failed:\n{}", failures.len(), converted + failures.len(), failures.join("\n"));
    }
}
