use bevy::prelude::*;
use common::path::PathRenderable;
use common::pipeline::plugin::{DespawnOnFinish, UsePipeline};
use laserlogic::{LaserTextOptions, from_ttf_text};

use crate::components::HunterEntity;
use common::scene::Billboard;
use crate::types::HIT_COLOR;

/// HUNTER across the scene; assets/pipelines/hunter/title.pipeline.yaml animates and removes it
pub fn spawn_title(commands: &mut Commands, billboard: &Billboard) {
    const TEXT: &str = "HUNTER";
    const FONT: &[u8] = include_bytes!("../../../../assets/fonts/centurygothic.ttf");
    const CHAR_WIDTH_RATIO: f32 = 0.65;
    const LETTER_SPACING: f32 = 0.08;

    // As large as fits: at most 55% of the scene height and 85% of its width
    let scene = billboard.half_size() * 2.0;
    let width_cap = scene.x * 0.85 / (TEXT.len() as f32 * (CHAR_WIDTH_RATIO + LETTER_SPACING));
    let options = LaserTextOptions {
        origin: Vec2::ZERO,
        height: (scene.y * 0.55).min(width_cap).clamp(0.8, 3.5),
        color: HIT_COLOR,
        center_on_origin: true,
        ..default()
    };
    match from_ttf_text(FONT, TEXT, &options) {
        Ok(path) => {
            billboard.spawn(
                commands,
                (
                    HunterEntity,
                    Transform::default(),
                    Visibility::default(),
                    path,
                    PathRenderable::default(),
                    UsePipeline::new("hunter/title"),
                    DespawnOnFinish,
                ),
            );
        }
        Err(e) => warn!("Could not outline the HUNTER title: {e:?}"),
    }
}
