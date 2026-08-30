use bevy::prelude::*;
use common::path::{LaserTextOptions, PathRenderable, PathSegment, UniversalPath};
use common::scene::SceneSetup;

use crate::components::*;
use crate::resources::SnakeState;
use crate::types::{CELL_SIZE, GEM_HALF_SIZE, SEGMENT_RADIUS};

/// Convert grid cell to scene-local position (centred on origin)
pub fn grid_to_local(cell: IVec2, grid_w: i32, grid_h: i32) -> Vec3 {
    let half_w = (grid_w as f32 * CELL_SIZE) / 2.0;
    let half_h = (grid_h as f32 * CELL_SIZE) / 2.0;
    Vec3::new(
        (cell.x as f32 + 0.5) * CELL_SIZE - half_w,
        (cell.y as f32 + 0.5) * CELL_SIZE - half_h,
        0.0,
    )
}

pub fn random_color() -> (f32, f32, f32) {
    use rand::random_range;
    match random_range(0..3) {
        0 => (1.0, 0.0, 0.0), // Red
        1 => (0.0, 1.0, 0.0), // Green
        _ => (0.0, 0.3, 1.0), // Blue
    }
}

pub fn random_gem_position(snake: &SnakeState) -> IVec2 {
    use rand::random_range;
    let margin_x = if snake.grid_w > 4 { 2 } else { 0 };
    let margin_y = if snake.grid_h > 4 { 2 } else { 0 };
    loop {
        let x = random_range(margin_x..(snake.grid_w - margin_x));
        let y = random_range(margin_y..(snake.grid_h - margin_y));
        let pos = IVec2::new(x, y);
        if !snake.segments.contains(&pos) {
            return pos;
        }
    }
}

pub fn spawn_head_entity(commands: &mut Commands, state: &SnakeState, scene_entity: Option<Entity>) {
    let pos = grid_to_local(state.segments[0], state.grid_w, state.grid_h);
    let (r, g, b) = state.segment_colors.first().copied().unwrap_or((1.0, 1.0, 1.0));
    let head_color = Color::srgb(r, g, b);
    let path = UniversalPath::circle(Vec2::ZERO, SEGMENT_RADIUS, head_color);
    let id = commands
        .spawn((
            SnakeHead,
            Transform::from_translation(pos),
            GlobalTransform::from(Transform::from_translation(pos)),
            Visibility::default(),
            path,
            PathRenderable::default(),
        ))
        .id();
    if let Some(scene) = scene_entity {
        commands.entity(scene).add_child(id);
    }
}

/// Build the snake body as a UniversalPath (polyline, split on boundary wraps)
pub fn build_snake_body_path(state: &SnakeState) -> UniversalPath {
    let mut universal_path = UniversalPath::new();
    if state.segments.len() < 2 {
        return universal_path;
    }
    let mut current_segment = PathSegment::empty();

    for i in 0..state.segments.len() {
        let current_cell = state.segments[i];
        let pos = grid_to_local(current_cell, state.grid_w, state.grid_h);
        let (r, g, b) = state.segment_colors.get(i).copied().unwrap_or((1.0, 1.0, 1.0));
        let color = Color::srgb(r, g, b);

        if i > 0 {
            let prev_cell = state.segments[i - 1];
            let dx = (current_cell.x - prev_cell.x).abs();
            let dy = (current_cell.y - prev_cell.y).abs();
            if dx > 1 || dy > 1 {
                // Screen boundary wrap: split sub-segment to prevent connecting laser lines
                if !current_segment.points.is_empty() {
                    universal_path.add_segment(current_segment);
                    current_segment = PathSegment::empty();
                }
            }
        }

        current_segment.push(pos.x, pos.y, color, 1);
    }

    if !current_segment.points.is_empty() {
        universal_path.add_segment(current_segment);
    }
    universal_path
}

pub fn spawn_snake_body_entity(
    commands: &mut Commands,
    state: &SnakeState,
    scene_entity: Option<Entity>,
) {
    let universal_path = build_snake_body_path(state);
    if universal_path.segments.is_empty() {
        return;
    }

    let id = commands
        .spawn((
            SnakeSegment { color: Color::WHITE },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            universal_path,
            PathRenderable::default(),
        ))
        .id();

    if let Some(scene) = scene_entity {
        commands.entity(scene).add_child(id);
    }
}

pub fn spawn_gem_entity(commands: &mut Commands, state: &SnakeState, scene_entity: Option<Entity>) {
    let (r, g, b) = state.gem_color;
    let color = Color::srgb(r, g, b);
    let pos = grid_to_local(state.gem_position, state.grid_w, state.grid_h);
    info!("★ [Snake] Spawned diamond gem at grid {:?}, color RGB ({:.1}, {:.1}, {:.1})", state.gem_position, r, g, b);
    let path = UniversalPath::diamond(Vec2::ZERO, GEM_HALF_SIZE, color);
    let id = commands
        .spawn((
            DiamondFood { color },
            Transform::from_translation(pos),
            GlobalTransform::from(Transform::from_translation(pos)),
            Visibility::default(),
            path,
            PathRenderable::default(),
        ))
        .id();
    if let Some(scene) = scene_entity {
        commands.entity(scene).add_child(id);
    }
}

pub fn spawn_border_entity(commands: &mut Commands, scene_entity: Option<Entity>, w: f32, h: f32) {
    let half_w = w * 0.5;
    let half_h = h * 0.5;
    let border_color = Color::srgb(0.25, 0.25, 0.25);
    let top_left = Vec2::new(-half_w, -half_h);
    let size = Vec2::new(w, h);
    let path = UniversalPath::rectangle(top_left, size, border_color);
    let id = commands
        .spawn((
            SnakeBorder,
            Transform::from_translation(Vec3::ZERO),
            GlobalTransform::default(),
            Visibility::default(),
            path,
            PathRenderable::default(),
        ))
        .id();
    if let Some(scene) = scene_entity {
        commands.entity(scene).add_child(id);
    }
}

pub fn spawn_snake_text_announcement(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
    scene_entity: Option<Entity>,
    text: &str,
    color: Color,
    duration_secs: f32,
) {
    let scene_dim = scene_setup.scene.scene_dimension;
    let height_cap = scene_dim.y as f32 * 0.55;
    let num_chars = text.len().max(1);
    let char_width_ratio = 0.65_f32;
    let letter_spacing = 0.08_f32;
    let total_width_per_unit = num_chars as f32 * (char_width_ratio + letter_spacing);
    let width_cap = (scene_dim.x as f32 * 0.85) / total_width_per_unit;
    let text_height = height_cap.min(width_cap).clamp(0.6, 3.5);

    let options = LaserTextOptions {
        origin: Vec2::ZERO,
        height: text_height,
        color,
        center_on_origin: true,
        ..Default::default()
    };

    let font_paths = [
        "/opt/lasertargets/assets/fonts/centurygothic.ttf",
        "assets/fonts/centurygothic.ttf",
        "assets/fonts/centurygothic_bold.ttf",
        "assets/fonts/FiraCodeNerdFont-Regular.ttf",
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/seguiemj.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];

    let mut maybe_title_path = None;
    for path in &font_paths {
        if let Ok(data) = std::fs::read(path) {
            if let Ok(text_path) = UniversalPath::from_ttf_text(&data, text, &options) {
                info!("✓ [Snake] Rendered text announcement '{}' using font {}", text, path);
                maybe_title_path = Some(text_path);
                break;
            }
        }
    }

    if maybe_title_path.is_none() {
        const FALLBACK_FONT_BYTES: &[u8] = include_bytes!("../../../../assets/fonts/centurygothic.ttf");
        if let Ok(text_path) = UniversalPath::from_ttf_text(FALLBACK_FONT_BYTES, text, &options) {
            info!("✓ [Snake] Rendered text announcement '{}' using embedded fallback font", text);
            maybe_title_path = Some(text_path);
        }
    }

    if let Some(title_path) = maybe_title_path {
        let child_entity = commands.spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            SnakeTitleAnnouncement {
                timer: Timer::from_seconds(duration_secs, TimerMode::Once),
            },
            title_path,
            PathRenderable::default(),
        )).id();

        if let Some(scene) = scene_entity {
            commands.entity(scene).add_child(child_entity);
        }
    } else {
        warn!("No usable TTF font found for text announcement '{}'", text);
    }
}

pub fn spawn_snake_title_entity(commands: &mut Commands, scene_setup: &SceneSetup, scene_entity: Option<Entity>) {
    spawn_snake_text_announcement(commands, scene_setup, scene_entity, "SNAKE", Color::srgb(0.2, 1.0, 0.4), 3.0);
}

/// System to tick and despawn text title announcement after its timer elapses
pub fn animate_snake_title_announcement(
    mut commands: Commands,
    time: Res<Time>,
    mut announcement_query: Query<(Entity, &mut SnakeTitleAnnouncement)>,
) {
    for (entity, mut announcement) in announcement_query.iter_mut() {
        announcement.timer.tick(time.delta());
        if announcement.timer.just_finished() {
            info!("★ [Snake] Vector title announcement finished -> despawned");
            commands.entity(entity).despawn();
        }
    }
}
