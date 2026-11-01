use std::collections::HashMap;
use std::sync::OnceLock;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::path::{LineStyle, PathPoint, PathSegment, UniversalPath};

/// Backward-compatible alias: ShapePoint is unified with PathPoint
pub type ShapePoint = PathPoint;

/// A reusable, named shape template for laser targets, UI, and minigames
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShapeTemplate {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub line_style: LineStyle,
    pub points: Vec<ShapePoint>,
}

impl ShapeTemplate {
    /// Load shape template from a JSON string
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Serialize shape template to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Instantiate this shape template into a PathSegment placed at `origin` and scaled by `scale`.
    /// Optionally override point colors with `color_override`.
    pub fn to_path_segment(
        &self,
        origin: Vec2,
        scale: Vec2,
        color_override: Option<Color>,
    ) -> PathSegment {
        let (override_r, override_g, override_b) = match color_override {
            Some(c) => PathPoint::color_to_rgb(c),
            None => (0, 0, 0),
        };

        let path_points: Vec<PathPoint> = self
            .points
            .iter()
            .map(|pt| {
                let (r, g, b) = if color_override.is_some() {
                    if pt.r == 0 && pt.g == 0 && pt.b == 0 {
                        (0, 0, 0)
                    } else {
                        (override_r, override_g, override_b)
                    }
                } else {
                    (pt.r, pt.g, pt.b)
                };

                PathPoint {
                    x: origin.x + pt.x * scale.x,
                    y: origin.y + pt.y * scale.y,
                    r,
                    g,
                    b,
                    dwell: pt.dwell,
                }
            })
            .collect();

        PathSegment {
            points: path_points,
            line_style: self.line_style,
            hint: crate::path::PathHint::General,
        }
    }

    /// Instantiate this shape template directly into a UniversalPath placed at `origin` and scaled by `scale`.
    pub fn to_universal_path(
        &self,
        origin: Vec2,
        scale: Vec2,
        color_override: Option<Color>,
    ) -> UniversalPath {
        let segment = self.to_path_segment(origin, scale, color_override);
        let mut path = UniversalPath::new();
        path.add_segment(segment);
        path
    }
}

/// Registry managing built-in and runtime-loaded shape templates (registered as a Bevy Resource)
#[derive(Resource, Clone, Debug)]
pub struct ShapeLibrary {
    shapes: HashMap<String, ShapeTemplate>,
}

impl Default for ShapeLibrary {
    fn default() -> Self {
        Self::new()
    }
}

/// Bevy plugin to register the ShapeLibrary resource on App startup
pub struct ShapesPlugin;

impl Plugin for ShapesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShapeLibrary>();
    }
}

impl ShapeLibrary {
    /// Create a new ShapeLibrary populated with all built-in default shape templates
    pub fn new() -> Self {
        let mut lib = Self {
            shapes: HashMap::new(),
        };

        lib.register_builtin_shapes();
        lib
    }

    /// Register built-in embedded shape templates and scan filesystem shape directories
    fn register_builtin_shapes(&mut self) {
        let builtins = [
            include_str!("../../assets/shapes/templates/diamond.json"),
            include_str!("../../assets/shapes/templates/box.json"),
            include_str!("../../assets/shapes/templates/circle.json"),
            include_str!("../../assets/shapes/templates/star.json"),
            include_str!("../../assets/shapes/templates/crosshair.json"),
            include_str!("../../assets/shapes/templates/target.json"),
            include_str!("../../assets/shapes/templates/triangle.json"),
        ];

        for json in builtins {
            if let Ok(shape) = ShapeTemplate::from_json(json) {
                self.shapes.insert(shape.name.to_lowercase(), shape);
            }
        }

        // Dynamically scan asset directories on disk for any new or updated JSON shape templates
        self.scan_directory("assets/shapes/templates");
        self.scan_directory("assets/shapes");
        self.scan_directory("../assets/shapes/templates");
        self.scan_directory("../assets/shapes");
    }

    /// Scan a filesystem directory (recursively) for shape template .json files and register them
    pub fn scan_directory<P: AsRef<std::path::Path>>(&mut self, dir_path: P) -> usize {
        let path = dir_path.as_ref();
        if !path.exists() || !path.is_dir() {
            return 0;
        }

        let mut loaded = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    loaded += self.scan_directory(&entry_path);
                } else if entry_path.is_file() && entry_path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = std::fs::read_to_string(&entry_path) {
                        if let Ok(shape) = ShapeTemplate::from_json(&content) {
                            self.shapes.insert(shape.name.to_lowercase(), shape);
                            loaded += 1;
                        }
                    }
                }
            }
        }
        loaded
    }

    /// Get a reference to a shape template by name (case-insensitive)
    pub fn get(&self, name: &str) -> Option<&ShapeTemplate> {
        self.shapes.get(&name.to_lowercase())
    }

    /// Register a shape template into the library
    pub fn register(&mut self, shape: ShapeTemplate) {
        self.shapes.insert(shape.name.to_lowercase(), shape);
    }

    /// Parse and register a shape template from a JSON string
    pub fn load_json(&mut self, json_str: &str) -> Result<String, String> {
        let shape = ShapeTemplate::from_json(json_str).map_err(|e: serde_json::Error| e.to_string())?;
        let name = shape.name.clone();
        self.register(shape);
        Ok(name)
    }

    /// List all registered shape names
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.shapes.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get all registered shape templates
    pub fn list(&self) -> Vec<&ShapeTemplate> {
        let mut templates: Vec<&ShapeTemplate> = self.shapes.values().collect();
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates
    }

    /// Access global pre-initialized static thread-safe ShapeLibrary
    pub fn global() -> &'static ShapeLibrary {
        static INSTANCE: OnceLock<ShapeLibrary> = OnceLock::new();
        INSTANCE.get_or_init(ShapeLibrary::new)
    }
}

/// Procedural vector shape generators
pub mod procedural {
    use bevy::prelude::*;
    use lyon_path::Path;
    use crate::path::{PathPoint, PathSegment, UniversalPath};

    /// Create a circle path with zero-tail galvo blanking and dwell
    pub fn circle(center: Vec2, radius: f32, color: Color) -> UniversalPath {
        let mut seg = PathSegment::empty();
        let num_pts = 32;
        let (r, g, b) = PathPoint::color_to_rgb(color);
        let start_x = center.x + radius;
        let start_y = center.y;

        // 1. Blank move + 4 blanked dwell points at start position (laser OFF, galvo settles)
        seg.points.push(PathPoint::new(start_x, start_y, 0, 0, 0, 4));

        // 2. 4 lit dwell points at start position (laser ON, galvo stationary)
        seg.points.push(PathPoint::new(start_x, start_y, r, g, b, 4));

        // 3. Scan 360° circle perimeter
        for i in 1..=num_pts {
            let angle = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            seg.points.push(PathPoint::new(x, y, r, g, b, 0));
        }

        // 4. Extra 2 overlap points past 360° to ensure 100% galvo loop closure
        for i in 1..=2 {
            let angle = (i as f32 / num_pts as f32) * std::f32::consts::TAU;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            seg.points.push(PathPoint::new(x, y, r, g, b, 0));
        }

        let end_angle = (2.0 as f32 / num_pts as f32) * std::f32::consts::TAU;
        let end_x = center.x + radius * end_angle.cos();
        let end_y = center.y + radius * end_angle.sin();

        // 5. 4 lit dwell points at end position (laser ON, galvo stationary)
        seg.points.push(PathPoint::new(end_x, end_y, r, g, b, 4));

        // 6. 4 blanked dwell points at end position (laser OFF at end position before moving)
        seg.points.push(PathPoint::new(end_x, end_y, 0, 0, 0, 4));

        UniversalPath::from_segment(seg)
    }

    /// Create a balloon path shape (closed circular body + knot + string) with zero-tail blanking dwell
    pub fn balloon(center: Vec2, radius: f32, color: Color) -> UniversalPath {
        let mut seg = PathSegment::empty();
        let num_pts = 32;
        let (r, g, b) = PathPoint::color_to_rgb(color);
        let top_x = center.x;
        let top_y = center.y + radius;

        // 1. Blank move + 4 blanked dwell points at top of balloon (laser OFF, galvo settles)
        seg.points.push(PathPoint::new(top_x, top_y, 0, 0, 0, 4));

        // 2. 4 lit dwell points at top position (laser ON, galvo stationary)
        seg.points.push(PathPoint::new(top_x, top_y, r, g, b, 4));

        // 3. Balloon circular body
        for i in 1..=num_pts {
            let angle = std::f32::consts::FRAC_PI_2 + (i as f32 / num_pts as f32) * std::f32::consts::TAU;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            seg.points.push(PathPoint::new(x, y, r, g, b, 0));
        }

        // 4. Extra 2 overlap points to ensure 100% body closure
        for i in 1..=2 {
            let angle = std::f32::consts::FRAC_PI_2 + (i as f32 / num_pts as f32) * std::f32::consts::TAU;
            let x = center.x + radius * angle.cos();
            let y = center.y + radius * angle.sin();
            seg.points.push(PathPoint::new(x, y, r, g, b, 0));
        }

        // 5. Balloon Knot at bottom
        let knot_y = center.y - radius;
        seg.points.push(PathPoint::new(center.x, knot_y, r, g, b, 4));
        seg.points.push(PathPoint::new(center.x - 0.04 * radius, knot_y - 0.05 * radius, r, g, b, 2));
        seg.points.push(PathPoint::new(center.x + 0.04 * radius, knot_y - 0.05 * radius, r, g, b, 2));
        seg.points.push(PathPoint::new(center.x, knot_y, r, g, b, 2));

        // 6. Balloon String
        let string_y = knot_y - 0.15 * radius;
        seg.points.push(PathPoint::new(center.x, string_y, r, g, b, 4));

        // 7. Blanked dwell points at end of string (laser OFF before galvo moves to next path)
        seg.points.push(PathPoint::new(center.x, string_y, 0, 0, 0, 4));

        UniversalPath::from_segment(seg)
    }

    /// Create a diamond (rotated square) path — a square rotated 45° with sharp corner dwells
    pub fn diamond(center: Vec2, half_size: f32, color: Color) -> UniversalPath {
        let top    = Vec2::new(center.x,             center.y + half_size);
        let right  = Vec2::new(center.x + half_size, center.y);
        let bottom = Vec2::new(center.x,             center.y - half_size);
        let left   = Vec2::new(center.x - half_size, center.y);

        let mut segment = PathSegment::empty();
        // Closed loop: top → right → bottom → left → top, all with sharp corner dwell
        segment.push(top.x,    top.y,    color, 4);
        segment.push(right.x,  right.y,  color, 4);
        segment.push(bottom.x, bottom.y, color, 4);
        segment.push(left.x,   left.y,   color, 4);
        segment.push(top.x,    top.y,    color, 4); // Exact closed loop seam (start == end)

        UniversalPath::from_segment(segment)
    }

    /// Create a sharp perpendicular crosshair path with proper arrival/departure dwells & blanking
    pub fn crosshair(center: Vec2, size: f32, color: Color) -> UniversalPath {
        let half_size = size / 2.0;
        let blank = Color::NONE;

        // 1. Horizontal Line Segment (-X to +X)
        let mut horiz_seg = PathSegment::empty();
        let left_x = center.x - half_size;
        let right_x = center.x + half_size;
        let y = center.y;

        horiz_seg.push(left_x, y, blank, 4);  // Blanked arrival dwell
        horiz_seg.push(left_x, y, color, 4);  // Lit start dwell
        horiz_seg.push(right_x, y, color, 4); // Lit end dwell
        horiz_seg.push(right_x, y, blank, 4); // Blanked departure dwell

        // 2. Vertical Line Segment (-Y to +Y)
        let mut vert_seg = PathSegment::empty();
        let bottom_y = center.y - half_size;
        let top_y = center.y + half_size;
        let x = center.x;

        vert_seg.push(x, bottom_y, blank, 4); // Blanked arrival dwell
        vert_seg.push(x, bottom_y, color, 4); // Lit start dwell
        vert_seg.push(x, top_y, color, 4);    // Lit end dwell
        vert_seg.push(x, top_y, blank, 4);    // Blanked departure dwell

        let mut path = UniversalPath::new();
        path.add_segment(horiz_seg);
        path.add_segment(vert_seg);
        path
    }

    /// Create a rectangle path
    pub fn rectangle(top_left: Vec2, size: Vec2, color: Color) -> UniversalPath {
        use lyon_tessellation::math::point;
        let mut builder = Path::builder().with_svg();
        builder.move_to(point(top_left.x, top_left.y));
        builder.line_to(point(top_left.x + size.x, top_left.y));
        builder.line_to(point(top_left.x + size.x, top_left.y + size.y));
        builder.line_to(point(top_left.x, top_left.y + size.y));
        builder.close();

        UniversalPath {
            segments: vec![PathSegment::from_lyon_path(&builder.build(), color, 1.0)],
        }
    }
}

pub use procedural::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_shapes_loading() {
        let lib = ShapeLibrary::new();
        let expected = ["box", "circle", "crosshair", "diamond", "star", "target", "triangle"];
        for name in expected {
            let shape = lib.get(name);
            assert!(shape.is_some(), "Built-in shape '{}' missing", name);
            assert_eq!(shape.unwrap().name.to_lowercase(), name);
            assert!(!shape.unwrap().points.is_empty(), "Shape '{}' has no points", name);
        }
    }

    #[test]
    fn test_shape_instantiation() {
        let lib = ShapeLibrary::global();
        let diamond = lib.get("diamond").expect("diamond shape should exist");
        
        let origin = Vec2::new(5.0, 10.0);
        let scale = Vec2::new(2.0, 3.0);
        let color = Color::srgb(1.0, 0.0, 0.0);
        
        let segment = diamond.to_path_segment(origin, scale, Some(color));
        assert_eq!(segment.points.len(), diamond.points.len());
        
        // First point of diamond is (0, 1) -> (5 + 0*2, 10 + 1*3) = (5, 13)
        assert_eq!(segment.points[0].x, 5.0);
        assert_eq!(segment.points[0].y, 13.0);
        assert_eq!(segment.points[0].r, 255);
        assert_eq!(segment.points[0].g, 0);
        assert_eq!(segment.points[0].b, 0);
    }

    #[test]
    fn test_directory_scanning() {
        let mut lib = ShapeLibrary::new();
        let dir = if std::path::Path::new("assets/shapes/templates").exists() {
            "assets/shapes/templates"
        } else {
            "../assets/shapes/templates"
        };
        let loaded = lib.scan_directory(dir);
        assert!(loaded >= 7, "Expected at least 7 templates loaded from directory scan");
    }
}
