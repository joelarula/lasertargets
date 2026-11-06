//! The game's side of the visuals: base shapes and positions from [`SnakeState`]. Each entity's
//! pipeline (assets/pipelines/snake/) turns them into what the laser draws and reacts to events.

use bevy::prelude::*;
use common::path::{PathRenderable, UniversalPath};
use common::pipeline::Signals;
use common::pipeline::plugin::{BasePath, DespawnOnFinish, UsePipeline};
use common::scene::{SceneEntity, SceneSetup};
use laserlogic::{LaserTextOptions, from_ttf_text};

use crate::components::{SnakeEntity, SnakePart};
use crate::rules::SnakeState;

/// The scene entity snake visuals hang under, if there is one
pub type SceneRoot<'w, 's> = Option<Single<'w, 's, Entity, With<SceneEntity>>>;

/// Spawns the head, body and gem for `state`
pub fn spawn_parts(commands: &mut Commands, state: &SnakeState, scene: Option<Entity>) {
    for part in SnakePart::ALL {
        let (path, at) = part.shape(state);
        let mut entity = commands.spawn((
            part,
            Transform::from_translation(at),
            BasePath(path.clone()),
            path,
            UsePipeline::new(part.pipeline()),
        ));
        if let Some(scene) = scene {
            entity.insert(ChildOf(scene));
        }
    }
}

/// Rewrites every part's base shape and position in place (no respawns, so the laser never
/// blanks between moves)
pub fn sync_parts(
    state: Res<SnakeState>,
    mut parts: Query<(&SnakePart, &mut BasePath, &mut Transform)>,
) {
    for (part, mut base, mut transform) in &mut parts {
        let (path, at) = part.shape(&state);
        base.0 = path;
        transform.translation = at;
    }
}

/// Publishes `snake.speed`: 0 at the starting pace, 1 at top speed
pub fn publish_snake_speed(state: Res<SnakeState>, signals: Option<ResMut<Signals>>) {
    if let Some(mut signals) = signals {
        signals.set("snake.speed", state.speed());
    }
}

/// Spawns laser text animated by `pipeline`, which also decides when it despawns
pub fn spawn_announcement(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
    scene: Option<Entity>,
    text: &str,
    color: Color,
    pipeline: &str,
) {
    let Some(path) = laser_text(text, &text_options(scene_setup, text, color)) else {
        warn!("No usable TTF font found for text announcement '{}'", text);
        return;
    };
    let mut entity = commands.spawn((
        SnakeEntity,
        Transform::default(),
        Visibility::default(),
        path,
        PathRenderable::default(),
        UsePipeline::new(pipeline),
        DespawnOnFinish,
    ));
    if let Some(scene) = scene {
        entity.insert(ChildOf(scene));
    }
}

/// Centred text as large as fits: at most 55% of the scene height and 85% of its width
fn text_options(scene_setup: &SceneSetup, text: &str, color: Color) -> LaserTextOptions {
    const CHAR_WIDTH_RATIO: f32 = 0.65;
    const LETTER_SPACING: f32 = 0.08;
    let scene = scene_setup.scene.scene_dimension;
    let height_cap = scene.y * 0.55;
    let width_per_unit_height = text.len().max(1) as f32 * (CHAR_WIDTH_RATIO + LETTER_SPACING);
    let width_cap = scene.x * 0.85 / width_per_unit_height;
    LaserTextOptions {
        origin: Vec2::ZERO,
        height: height_cap.min(width_cap).clamp(0.6, 3.5),
        color,
        center_on_origin: true,
        ..Default::default()
    }
}

/// Outlines `text` with the first installed font that works, else the embedded one
fn laser_text(text: &str, options: &LaserTextOptions) -> Option<UniversalPath> {
    const FONT_PATHS: [&str; 8] = [
        "/opt/lasertargets/assets/fonts/centurygothic.ttf",
        "assets/fonts/centurygothic.ttf",
        "assets/fonts/centurygothic_bold.ttf",
        "assets/fonts/FiraCodeNerdFont-Regular.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/seguiemj.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    const EMBEDDED_FONT: &[u8] = include_bytes!("../../../../assets/fonts/centurygothic.ttf");

    FONT_PATHS
        .iter()
        .find_map(|path| from_ttf_text(&std::fs::read(path).ok()?, text, options).ok())
        .or_else(|| from_ttf_text(EMBEDDED_FONT, text, options).ok())
}
