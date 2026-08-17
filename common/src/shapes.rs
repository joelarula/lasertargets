use std::collections::HashMap;
use std::sync::OnceLock;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::path::{LineStyle, PathPoint, PathSegment, UniversalPath};

/// A single relative point in a shape template
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShapePoint {
    pub x: f32,
    pub y: f32,
    #[serde(default = "default_color_val")]
    pub r: u8,
    #[serde(default = "default_color_val")]
    pub g: u8,
    #[serde(default = "default_color_val")]
    pub b: u8,
    #[serde(default)]
    pub dwell: u8,
}

fn default_color_val() -> u8 {
    255
}

impl ShapePoint {
    pub fn new(x: f32, y: f32, r: u8, g: u8, b: u8, dwell: u8) -> Self {
        Self { x, y, r, g, b, dwell }
    }
}

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
        let loaded = lib.scan_directory("assets/shapes/templates");
        assert!(loaded >= 7, "Expected at least 7 templates loaded from directory scan");
    }
}
