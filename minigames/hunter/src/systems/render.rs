use bevy::prelude::*;
use common::path::{LaserTextOptions, PathRenderable, UniversalPath};
use common::scene::{SceneEntity, SceneSetup};
use common::state::GameState;

use crate::common::GAME_ID;
use crate::components::{HunterShotRipple, HunterTitleAnnouncement};

pub fn spawn_hunter_title_on_session_start(
    mut commands: Commands,
    mut created_events: MessageReader<common::game::GameSessionCreated>,
    scene_query: Query<Entity, With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
    existing_titles: Query<Entity, With<HunterTitleAnnouncement>>,
) {
    for event in created_events.read() {
        if event.game_session.game_id != GAME_ID || event.game_session.state != GameState::InGame {
            continue;
        }

        for entity in existing_titles.iter() {
            commands.entity(entity).despawn();
        }

        let scene_dim = scene_setup.scene.scene_dimension;
        let height_cap = scene_dim.y as f32 * 0.55;
        let num_chars = 6usize;
        let char_width_ratio = 0.65_f32;
        let letter_spacing = 0.08_f32;
        let total_width_per_unit = num_chars as f32 * (char_width_ratio + letter_spacing);
        let width_cap = (scene_dim.x as f32 * 0.85) / total_width_per_unit;
        let text_height = height_cap.min(width_cap).clamp(0.8, 3.5);

        let options = LaserTextOptions {
            origin: Vec2::ZERO,
            height: text_height,
            color: Color::srgb(1.0, 0.95, 0.1),
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
                if let Ok(text_path) = UniversalPath::from_ttf_text(&data, "HUNTER", &options) {
                    info!("✓ [Hunter] Rendered full-scene center vector title using font {}", path);
                    maybe_title_path = Some(text_path);
                    break;
                }
            }
        }

        if maybe_title_path.is_none() {
            const FALLBACK_FONT_BYTES: &[u8] = include_bytes!("../../../../assets/fonts/centurygothic.ttf");
            if let Ok(text_path) = UniversalPath::from_ttf_text(FALLBACK_FONT_BYTES, "HUNTER", &options) {
                info!("✓ [Hunter] Rendered full-scene center vector title using embedded fallback font");
                maybe_title_path = Some(text_path);
            }
        }

        let Some(title_path) = maybe_title_path else {
            warn!("No usable TTF font found for HUNTER title");
            continue;
        };

        let child_entity = commands.spawn((
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            HunterTitleAnnouncement {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            title_path,
            PathRenderable::default(),
        )).id();

        if let Ok(scene_entity) = scene_query.single() {
            commands.entity(scene_entity).add_child(child_entity);
        }
    }
}

pub fn animate_hunter_title_announcement(
    mut commands: Commands,
    time: Res<Time>,
    mut announcement_query: Query<(Entity, &mut HunterTitleAnnouncement)>,
) {
    for (entity, mut announcement) in announcement_query.iter_mut() {
        announcement.timer.tick(time.delta());
        if announcement.timer.just_finished() {
            info!("★ [Hunter] Vector title announcement finished -> despawned");
            commands.entity(entity).despawn();
        }
    }
}

/// Animate expanding shot ripple rings upon Hunter game clicks
pub fn animate_hunter_shot_ripples(
    mut commands: Commands,
    time: Res<Time>,
    mut ripple_query: Query<(Entity, &mut HunterShotRipple, &mut UniversalPath)>,
) {
    for (entity, mut ripple, mut path) in ripple_query.iter_mut() {
        ripple.current_radius += ripple.growth_rate * time.delta_secs();
        if ripple.current_radius >= ripple.max_radius {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        } else {
            *path = UniversalPath::circle(Vec2::ZERO, ripple.current_radius, ripple.color);
        }
    }
}
