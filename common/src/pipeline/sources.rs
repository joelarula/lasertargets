//! Built-in pipeline sources.

use super::registry::PipelineRegistry;
use super::stage::{PathSource, StageCtx, StageKind};
use crate::path::UniversalPath;
use crate::shape_v2::Shape;
use bevy::prelude::*;
use serde::Deserialize;

/// Registers the sources defined in `common`
pub fn register(registry: &mut PipelineRegistry) {
    registry.register_source::<ShapeSource>("shape");
}

/// Static geometry: a built-in primitive or a Shape v2 template by name.
///
/// ```yaml
/// source:
///   shape: { builtin: circle, size_m: 0.05, color: [255, 0, 0] }
/// source:
///   shape: { template: dashed_box }
/// ```
#[derive(Deserialize)]
#[serde(deny_unknown_fields, try_from = "ShapeSourceSpec")]
pub struct ShapeSource {
    path: UniversalPath,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShapeSourceSpec {
    #[serde(default)]
    builtin: Option<Builtin>,
    #[serde(default)]
    template: Option<String>,
    /// Radius or half size in metres (built-ins)
    #[serde(default = "default_size")]
    size_m: f32,
    #[serde(default = "white")]
    color: [u8; 3],
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Builtin {
    Circle,
    Balloon,
    Diamond,
    Crosshair,
    Rect,
}

fn default_size() -> f32 {
    0.1
}

fn white() -> [u8; 3] {
    [255, 255, 255]
}

impl TryFrom<ShapeSourceSpec> for ShapeSource {
    type Error = String;

    fn try_from(spec: ShapeSourceSpec) -> Result<Self, String> {
        let [r, g, b] = spec.color;
        let color = Color::srgb_u8(r, g, b);
        let s = spec.size_m;
        let path = match (spec.builtin, spec.template) {
            (Some(builtin), None) => match builtin {
                Builtin::Circle => UniversalPath::circle(Vec2::ZERO, s, color),
                Builtin::Balloon => UniversalPath::balloon(Vec2::ZERO, s, color),
                Builtin::Diamond => UniversalPath::diamond(Vec2::ZERO, s, color),
                Builtin::Crosshair => UniversalPath::crosshair(Vec2::ZERO, s, color),
                Builtin::Rect => UniversalPath::rectangle(Vec2::new(-s, s), Vec2::new(2.0 * s, -2.0 * s), color),
            },
            (None, Some(name)) => Shape::builtin()
                .into_iter()
                .find(|shape| shape.name == name)
                .map(|shape| shape.to_universal_path())
                .ok_or_else(|| format!("unknown shape template `{name}`"))?,
            _ => return Err("set exactly one of `builtin` or `template`".into()),
        };
        Ok(Self { path })
    }
}

impl PathSource for ShapeSource {
    fn kind(&self) -> StageKind {
        StageKind::Static
    }

    fn generate(&mut self, _ctx: &StageCtx, out: &mut UniversalPath) {
        out.clone_from(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_builtins_and_rejects_ambiguous_specs() {
        let mut registry = PipelineRegistry::default();
        register(&mut registry);
        assert!(registry.build_source("shape", &json!({ "builtin": "circle", "size_m": 0.2 })).is_ok());
        assert!(registry.build_source("shape", &json!({})).is_err());
        assert!(registry.build_source("shape", &json!({ "builtin": "circle", "template": "x" })).is_err());
        assert!(registry.build_source("shape", &json!({ "template": "no_such_shape" })).is_err());
        assert!(registry.build_source("shape", &json!({ "builtin": "hexagon" })).is_err());
    }
}
