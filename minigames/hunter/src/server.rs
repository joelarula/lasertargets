use bevy::{app::{App, Plugin, Update}, ecs::{component::Component, message::{MessageReader, MessageWriter}, system::Commands}, prelude::*};
use common::{
    game::GameSession,
    path::{LaserTextOptions, UniversalPath},
    scene::{SceneEntity, SceneSetup},
    state::{GameState, ServerState},
    target::HunterTarget,
};
use crate::common::{GAME_ID, generate_game_report};
use crate::model::{BalloonRiseSpeed, BalloonTargetEntity, BroadcastStatsUpdateEvent, CollisionIndicator, GameReport, HunterClickEvent, HunterGameStats, TargetEvent};
use gamepad::{Btn, GamepadState, PrevGamepadState, ServerGamepadCursor};


/// Event for spawning hunter targets (server-only)
#[derive(Message, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpawnHunterTargetEvent {
    pub target: HunterTarget,
    pub position: Vec3,
}

/// Component for hunter target entities
#[derive(Component)]
pub struct HunterTargetEntity {
    pub target_type: HunterTarget,
    pub uuid: bevy::asset::uuid::Uuid,
    pub reward: u32,
    pub session_id: bevy::asset::uuid::Uuid,
}

/// Component for title announcement text overlay
#[derive(Component)]
pub struct HunterTitleAnnouncement {
    pub timer: Timer,
}

/// Resource tracking the reticle cursor mode and target selection
#[derive(Resource, Debug, Clone)]
pub struct HunterTargetSelection {
    pub selected_index: usize, // 0 = GunShot Mode (Default), 1 = Cyan Static Big, 2 = Yellow Balloon, 3 = Magenta Balloon, 4 = Green Balloon
    pub sizes: [f32; 5],
}

impl Default for HunterTargetSelection {
    fn default() -> Self {
        Self {
            selected_index: 0,
            sizes: [0.0, 0.45, 0.30, 0.20, 0.25],
        }
    }
}

impl HunterTargetSelection {
    pub fn get_target(&self) -> Option<HunterTarget> {
        let size = self.sizes.get(self.selected_index).copied().unwrap_or(0.25);
        match self.selected_index {
            1 => Some(HunterTarget::Basic(size, Color::srgb(0.0, 0.9, 1.0))),  // Static Big: Cyan Large Practice Circle
            2 => Some(HunterTarget::Baloon(size, Color::srgb(1.0, 0.9, 0.1))),  // Balloon 1: Yellow Rising Balloon
            3 => Some(HunterTarget::Baloon(size, Color::srgb(1.0, 0.1, 0.9))),  // Balloon 2: Magenta Small Fast Balloon
            4 => Some(HunterTarget::Baloon(size, Color::srgb(0.1, 1.0, 0.3))),  // Balloon 3: Green Medium Balloon
            _ => None, // 0 = GunShot Mode
        }
    }

    pub fn target_name(&self) -> String {
        let size = self.sizes.get(self.selected_index).copied().unwrap_or(0.25);
        match self.selected_index {
            0 => "Gun Shot Mode".to_string(),
            1 => format!("Cyan Practice Circle ({:.2}m)", size),
            2 => format!("Yellow Rising Balloon ({:.2}m)", size),
            3 => format!("Magenta Fast Balloon ({:.2}m)", size),
            4 => format!("Green Medium Balloon ({:.2}m)", size),
            _ => "Gun Shot Mode".to_string(),
        }
    }

    pub fn cycle(&mut self) {
        self.selected_index = (self.selected_index + 1) % 5;
    }

    pub fn reset_to_gunshot(&mut self) {
        self.selected_index = 0;
    }

    pub fn increase_size(&mut self) {
        if self.selected_index > 0 && self.selected_index < 5 {
            self.sizes[self.selected_index] = (self.sizes[self.selected_index] + 0.05).min(1.00);
        }
    }

    pub fn decrease_size(&mut self) {
        if self.selected_index > 0 && self.selected_index < 5 {
            self.sizes[self.selected_index] = (self.sizes[self.selected_index] - 0.05).max(0.10);
        }
    }
}

#[derive(Component)]
pub struct TargetSpawnImmunity {
    pub spawn_pos: Vec3,
    pub radius: f32,
}

/// Component tracking an expanding shot ripple animation for Hunter game clicks
#[derive(Component)]
pub struct HunterShotRipple {
    pub current_radius: f32,
    pub max_radius: f32,
    pub growth_rate: f32,
    pub color: Color,
}

pub struct HunterGameServerPlugin;

impl Plugin for HunterGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HunterTargetSelection>();
        app.add_message::<SpawnHunterTargetEvent>();
        app.add_message::<HunterClickEvent>();
        app.add_message::<BroadcastStatsUpdateEvent>();
        app.add_systems(
            Update,
            (
                spawn_hunter_targets,
                update_target_spawn_immunity,
                handle_hunter_clicks,
                handle_hunter_gamepad_inputs,
                animate_hunter_shot_ripples,
                check_balloon_out_of_bounds,
                forward_hunter_stats_to_network,
                handle_incoming_hunter_payloads,
            )
                .run_if(in_state(ServerState::InGame))
                .run_if(hunter_session_is_running),
        );
        app.add_systems(
            FixedUpdate,
            update_balloon_positions
                .run_if(in_state(ServerState::InGame))
                .run_if(hunter_session_is_running),
        );
        app.add_systems(OnExit(ServerState::InGame), (
            save_hunter_report, 
            reset_hunter_session).chain());
        app.add_systems(Update, (
            reset_hunter_on_new_session,
            spawn_hunter_title_on_session_start,
            animate_hunter_title_announcement,
        ));
    }
}

fn hunter_session_is_running(game_sessions: Query<&GameSession>) -> bool {
    game_sessions
        .iter()
        .any(|session| session.game_id == GAME_ID && session.state == GameState::InGame)
}

fn reset_hunter_session(
    mut commands: Commands,
    targets: Query<Entity, With<HunterTargetEntity>>,
    indicators: Query<Entity, With<CollisionIndicator>>,
    titles: Query<Entity, With<HunterTitleAnnouncement>>,
    stats: Option<ResMut<HunterGameStats>>,
) {
    for entity in targets.iter() {
        commands.entity(entity).despawn();
    }
    for entity in indicators.iter() {
        commands.entity(entity).despawn();
    }
    for entity in titles.iter() {
        commands.entity(entity).despawn();
    }

    if stats.is_some() {
        commands.remove_resource::<HunterGameStats>();
    }
}

fn reset_hunter_on_new_session(
    mut commands: Commands,
    mut created_events: MessageReader<common::game::GameSessionCreated>,
    mut exit_events: MessageReader<common::game::ExitGameEvent>,
    targets: Query<Entity, With<HunterTargetEntity>>,
    indicators: Query<Entity, With<CollisionIndicator>>,
    titles: Query<Entity, With<HunterTitleAnnouncement>>,
    stats: Option<ResMut<HunterGameStats>>,
) {
    let mut should_cleanup = false;
    for _ in created_events.read() {
        should_cleanup = true;
    }
    for _ in exit_events.read() {
        should_cleanup = true;
    }

    if should_cleanup {
        for entity in targets.iter() {
            commands.entity(entity).despawn();
        }
        for entity in indicators.iter() {
            commands.entity(entity).despawn();
        }
        for entity in titles.iter() {
            commands.entity(entity).despawn();
        }

        if stats.is_some() {
            commands.remove_resource::<HunterGameStats>();
        }
    }
}

fn spawn_hunter_title_on_session_start(
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
        // Height-based cap: 55% of scene height
        let height_cap = scene_dim.y as f32 * 0.55;
        // Width-based cap: fit "HUNTER" (6 chars) within 85% of scene width.
        // Century Gothic char width ≈ 0.65× height, plus letter_spacing (0.08) per char.
        let num_chars = 6usize;
        let char_width_ratio = 0.65_f32;
        let letter_spacing = 0.08_f32;
        let total_width_per_unit = num_chars as f32 * (char_width_ratio + letter_spacing);
        let width_cap = (scene_dim.x as f32 * 0.85) / total_width_per_unit;
        let text_height = height_cap.min(width_cap).clamp(0.8, 3.5);

        let options = LaserTextOptions {
            origin: Vec2::ZERO,
            height: text_height,
            color: Color::srgb(1.0, 0.95, 0.1), // Bright yellow/gold title path
            center_on_origin: true,
            ..Default::default()
        };

        let font_paths = [
            // Project-bundled fonts (Century Gothic is primary default)
            "/opt/lasertargets/assets/fonts/centurygothic.ttf",
            "assets/fonts/centurygothic.ttf",
            "assets/fonts/centurygothic_bold.ttf",
            "assets/fonts/FiraCodeNerdFont-Regular.ttf",
            // Windows system fonts
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/seguiemj.ttf",
            // Linux system fonts
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
            const FALLBACK_FONT_BYTES: &[u8] = include_bytes!("../../../assets/fonts/centurygothic.ttf");
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
            common::path::PathRenderable::default(),
        )).id();

        if let Ok(scene_entity) = scene_query.single() {
            commands.entity(scene_entity).add_child(child_entity);
        }
    }
}

fn animate_hunter_title_announcement(
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

/// Spawn hunter target entities
fn spawn_hunter_targets(
    mut commands: Commands,
    mut spawn_events: MessageReader<SpawnHunterTargetEvent>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    mut stats: Option<ResMut<HunterGameStats>>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
    time: Res<Time>,
    game_sessions: Query<&GameSession>,
    scene_setup: Res<SceneSetup>,
) {
    for event in spawn_events.read() {
        info!("Spawning hunter target at {:?}", event.position);
        
        // Generate unique UUID for this target
        let target_uuid = bevy::asset::uuid::Uuid::new_v4();
        let reward = 10; // Base reward for all targets
        
        let mut session_id = bevy::asset::uuid::Uuid::nil();
        
        // Create UniversalPath based on target type
        let (radius, color, is_balloon) = match &event.target {
            HunterTarget::Basic(size, color) => (*size, *color, false),
            HunterTarget::Baloon(size, color) => (*size, *color, true),
        };
        
        let path = if is_balloon {
            UniversalPath::balloon(Vec2::ZERO, radius, color)
        } else {
            UniversalPath::circle(Vec2::ZERO, radius, color)
        };
        
        // Get local position relative to scene transform (all target types release from reticle cursor position)
        let (local_position, spawn_world_pos) = if let Ok((_scene_entity, scene_transform)) = scene_query.single() {
            let mut snapped_world_pos = event.position;
            snapped_world_pos.z = scene_transform.translation.z;

            // Convert world position to local position relative to scene
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            (scene_matrix.inverse().transform_point3(snapped_world_pos), snapped_world_pos)
        } else {
            // Fallback: use world position if no scene found
            (event.position, event.position)
        };

        // Update stats for this session
    if let Some(stats) = stats.as_mut() {
            stats.targets_spawned += 1;
            session_id = stats.session_id;

            let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
            stats.target_events.push(TargetEvent {
                target_uuid,
                event_type: "spawned".to_string(),
                timestamp: elapsed,
                position: spawn_world_pos,
            });

            // Raise event for network plugin to broadcast
            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: stats.session_id,
                targets_spawned: stats.targets_spawned,
                targets_popped: stats.targets_popped,
                misses: stats.misses,
                score: stats.score,
            });
        } else if let Some(session) = game_sessions.iter().find(|session| session.game_id == GAME_ID) {
            session_id = session.session_id;
            let elapsed = 0.0;
            let mut new_stats = HunterGameStats {
                session_id: session.session_id,
                targets_spawned: 1,
                targets_popped: 0,
                misses: 0,
                score: 0,
                target_events: Vec::new(),
                game_start_time: time.elapsed_secs_f64(),
            };
            new_stats.target_events.push(TargetEvent {
                target_uuid,
                event_type: "spawned".to_string(),
                timestamp: elapsed,
                position: spawn_world_pos,
            });
            commands.insert_resource(new_stats);

            stats_events.write(BroadcastStatsUpdateEvent {
                session_id: session.session_id,
                targets_spawned: 1,
                targets_popped: 0,
                misses: 0,
                score: 0,
            });
        }
        
        let radius = match &event.target {
            HunterTarget::Basic(size, _) => *size,
            HunterTarget::Baloon(size, _) => *size,
        };

        let transform = Transform::from_translation(local_position);
        
        let target_entity = commands.spawn((
            transform,
            GlobalTransform::from(transform),
            Visibility::default(),
            HunterTargetEntity {
                target_type: event.target.clone(),
                uuid: target_uuid,
                reward,
                session_id,
            },
            TargetSpawnImmunity {
                spawn_pos: event.position,
                radius,
            },
            path,
            common::path::PathRenderable::default(),
        )).id();
        
        // Add balloon-specific components for rising behavior
        if is_balloon {
            commands.entity(target_entity).insert((
                BalloonTargetEntity,
                BalloonRiseSpeed::default(),
            ));
        }
        
        // Parent to scene entity if it exists
        if let Ok((scene_entity, _)) = scene_query.single() {
            commands.entity(scene_entity).add_child(target_entity);
            info!("Spawned hunter target entity as child of scene at local position {:?}", local_position);
        } else {
            warn!("No scene entity found, spawned hunter target without parent at {:?}", event.position);
        }

        info!("Spawned hunter target entity {:?}", target_entity);
    }
}

/// System that checks if reticle cursor has moved outside a newly spawned target's radius.
/// Once cursor leaves, immunity is removed and target becomes shootable!
fn update_target_spawn_immunity(
    mut commands: Commands,
    cursor: Option<Res<ServerGamepadCursor>>,
    immunity_query: Query<(Entity, &Transform, Option<&ChildOf>, &TargetSpawnImmunity)>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
) {
    let Some(cursor) = cursor else { return; };
    let cursor_pos = cursor.position;
    let scene_transform = scene_query.single().ok().map(|(_, t)| t);

    for (entity, transform, parent, immunity) in immunity_query.iter() {
        let target_pos = if parent.is_some() {
            if let Some(scene_transform) = scene_transform {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            }
        } else {
            transform.translation
        };

        let dist = cursor_pos.distance(target_pos);
        if dist > immunity.radius {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.remove::<TargetSpawnImmunity>();
                info!("✓ Cursor moved outside spawn radius — target is now shootable");
            }
        }
    }
}

/// Handles gamepad input during Hunter game:
/// - Button A (South): Roll / cycle through target types (Basic Red -> Yellow Balloon -> Cyan Large -> Magenta Small Balloon)
/// - Button B (East): Release selected target into game at reticle position IF pressed on open space (with red shot dot indicator), OR shoot target IF pressed on an existing target!
fn handle_hunter_gamepad_inputs(
    state: Option<Res<GamepadState>>,
    prev: Option<Res<PrevGamepadState>>,
    cursor: Option<Res<ServerGamepadCursor>>,
    mut selection: ResMut<HunterTargetSelection>,
    game_sessions: Query<&GameSession>,
    mut click_events: MessageWriter<HunterClickEvent>,
    mut spawn_events: MessageWriter<SpawnHunterTargetEvent>,
) {
    let (Some(state), Some(prev), Some(cursor)) = (state, prev, cursor) else { return; };
    if !state.connected { return; }

    let Some(active_session) = game_sessions.iter().find(|s| s.game_id == GAME_ID && s.state == GameState::InGame) else { return; };

    // Button A (South) -> Roll / Cycle reticle mode: GunShot Mode -> Red Circle -> Yellow Balloon -> Cyan Circle -> Magenta Balloon -> GunShot Mode
    if state.just_pressed(&prev, Btn::South) {
        selection.cycle();
        info!("🎮 [Hunter Mode Switch] Selected cursor mode #{}: {}", selection.selected_index, selection.target_name());
    }

    // LeftBumper (LB) -> Decrease active target radius (-0.05m)
    if state.just_pressed(&prev, Btn::LeftBumper) {
        selection.decrease_size();
        info!("🎮 [Hunter Target Size] Decreased target size: {}", selection.target_name());
    }

    // RightBumper (RB) -> Increase active target radius (+0.05m)
    if state.just_pressed(&prev, Btn::RightBumper) {
        selection.increase_size();
        info!("🎮 [Hunter Target Size] Increased target size: {}", selection.target_name());
    }

    // Button B (East) or Right Trigger (RT) -> If in Target Spawning mode (1-4): Release target into game & auto-reset cursor to GunShot mode (0)!
    // If in GunShot mode (0): Shoot at reticle cursor position!
    if state.just_pressed(&prev, Btn::East) || state.just_pressed(&prev, Btn::RightTrigger) {
        let click_pos = cursor.position;

        if let Some(target_to_spawn) = selection.get_target() {
            // Currently in Target Spawning mode (Index 1-4) -> Release target into game & auto-reset to GunShot mode!
            info!("🚀 [Hunter Gamepad] RELEASING target [{}] at {:?}", selection.target_name(), click_pos);
            spawn_events.write(SpawnHunterTargetEvent {
                target: target_to_spawn,
                position: click_pos,
            });
            selection.reset_to_gunshot();
            info!("🎯 [Hunter Gamepad] Reticle cursor mode auto-reset to GunShot Mode");
        } else {
            // Currently in GunShot Mode (Index 0) -> Shoot at reticle position!
            info!("🎯 [Hunter Gamepad] SHOOTING at {:?}", click_pos);
            click_events.write(HunterClickEvent {
                session_id: active_session.session_id,
                click_position: click_pos,
            });
        }
    }
}

/// Handle click events from clients and detect collisions server-side
fn handle_hunter_clicks(
    mut commands: Commands,
    mut click_events: MessageReader<HunterClickEvent>,
    target_query: Query<(Entity, &Transform, Option<&ChildOf>, &HunterTargetEntity, Option<&TargetSpawnImmunity>)>,
    scene_query: Query<(Entity, &Transform), With<SceneEntity>>,
    scene_setup: Res<SceneSetup>,
    mut stats: Option<ResMut<HunterGameStats>>,
    time: Res<Time>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
    indicator_query: Query<Entity, With<CollisionIndicator>>,
) {
    for event in click_events.read() {
        let click_pos = event.click_position;
        let scene_result = scene_query.single().ok();
        let scene_transform = scene_result.map(|(_, t)| t);
        let mut hit_any = false;
        
        // Check all targets for collision
        for (entity, transform, parent, target_entity, immunity) in &target_query {
            // Only check targets for this session, and skip immune targets (newly released target until cursor leaves area)
            if target_entity.session_id != event.session_id || immunity.is_some() {
                continue;
            }

            let target_pos = if parent.is_some() {
                if let Some(scene_transform) = scene_transform {
                    scene_transform.transform_point(transform.translation)
                } else {
                    transform.translation
                }
            } else if let Some(scene_transform) = scene_transform {
                scene_transform.transform_point(transform.translation)
            } else {
                transform.translation
            };
            let distance = click_pos.distance(target_pos);
            
            let radius = match &target_entity.target_type {
                HunterTarget::Basic(size, _) => *size,
                HunterTarget::Baloon(size, _) => *size,
            };
            
            if distance <= radius {
                hit_any = true;
                // Target hit! Update stats
                if let Some(mut stats) = stats.as_mut() {
                    stats.targets_popped += 1;
                    stats.score += target_entity.reward;
                    
                    // Track event
                    let elapsed = time.elapsed_secs_f64() - stats.game_start_time;
                    stats.target_events.push(crate::model::TargetEvent {
                        target_uuid: target_entity.uuid,
                        event_type: "popped".to_string(),
                        timestamp: elapsed,
                        position: target_pos,
                    });
                    
                    // Broadcast stats update (path despawn is automatic)
                    stats_events.write(BroadcastStatsUpdateEvent {
                        session_id: event.session_id,
                        targets_spawned: stats.targets_spawned,
                        targets_popped: stats.targets_popped,
                        misses: stats.misses,
                        score: stats.score,
                    });
                    
                    info!("Target {} popped at {:?}, score: {}", target_entity.uuid, target_pos, stats.score);
                }
                
                // Despawn target (path broadcast handles visual removal)
                commands.entity(entity).despawn();
                break; // Only pop one target per click
            }
        }

        if !hit_any {
            if let Some(scene_transform) = scene_transform {
                let scene_matrix = Mat4::from_scale_rotation_translation(
                    scene_transform.scale,
                    scene_transform.rotation,
                    scene_transform.translation,
                );
                let local_click = scene_matrix.inverse().transform_point3(click_pos);
                let half_width = scene_setup.scene.scene_dimension.x as f32 / 2.0;
                let half_height = scene_setup.scene.scene_dimension.y as f32 / 2.0;

                if local_click.x.abs() <= half_width && local_click.y.abs() <= half_height {
                    if let Some(mut stats) = stats.as_mut() {
                        stats.misses += 1;
                        stats_events.write(BroadcastStatsUpdateEvent {
                            session_id: event.session_id,
                            targets_spawned: stats.targets_spawned,
                            targets_popped: stats.targets_popped,
                            misses: stats.misses,
                            score: stats.score,
                        });
                    }
                }
            }
        }

        // Despawn any previous click indicators so only the latest shot dot is active
        for entity in indicator_query.iter() {
            if let Ok(mut e) = commands.get_entity(entity) {
                e.despawn();
            }
        }

        // Spawn shot indicator dot path at shot location (Gold on target HIT, Red on MISS)
        if let Some(scene_transform) = scene_transform {
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_transform.scale,
                scene_transform.rotation,
                scene_transform.translation,
            );
            let local_click = scene_matrix.inverse().transform_point3(click_pos);

            let dot_color = if hit_any {
                Color::srgb(1.0, 0.95, 0.1) // Gold/Yellow dot on target HIT!
            } else {
                Color::srgb(1.0, 0.1, 0.0) // Red dot on MISS!
            };

            let indicator_path = UniversalPath::circle(
                Vec2::ZERO,
                0.05, // 5cm radius shot indicator dot
                dot_color,
            );

            let indicator_transform = Transform::from_translation(local_click);
            let indicator_entity = commands.spawn((
                CollisionIndicator,
                HunterShotRipple {
                    current_radius: 0.05,
                    max_radius: 0.35, // Expands to 35cm radius ripple ring!
                    growth_rate: 3.0, // Fast 3.0m/s expansion (~0.10s snappy animation)
                    color: dot_color,
                },
                indicator_transform,
                GlobalTransform::from(indicator_transform),
                Visibility::default(),
                indicator_path,
                common::path::PathRenderable::default(),
            )).id();

            if let Some((scene_entity, _)) = scene_result {
                commands.entity(scene_entity).add_child(indicator_entity);
            }
            info!("🎯 Spawned expanding shot ripple ring at {:?}", local_click);
        }
    }
}

/// Animate expanding shot ripple rings upon Hunter game clicks
fn animate_hunter_shot_ripples(
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



/// Move balloon targets upward each fixed tick
fn update_balloon_positions(
    mut balloon_query: Query<(&mut Transform, &BalloonRiseSpeed), With<BalloonTargetEntity>>,
    time: Res<Time>,
) {
    for (mut transform, speed) in balloon_query.iter_mut() {
        transform.translation.y += speed.0 * time.delta_secs();
    }
}

/// Despawn balloons that have risen past the top of the scene
fn check_balloon_out_of_bounds(
    mut commands: Commands,
    balloon_query: Query<(Entity, &Transform, &HunterTargetEntity), With<BalloonTargetEntity>>,
    scene_setup: Res<SceneSetup>,
    mut stats: Option<ResMut<HunterGameStats>>,
    mut stats_events: MessageWriter<BroadcastStatsUpdateEvent>,
) {
    let half_height = scene_setup.scene.scene_dimension.y as f32 / 2.0;
    
    for (entity, transform, target) in balloon_query.iter() {
        let radius = match &target.target_type {
            HunterTarget::Baloon(size, _) => *size,
            _ => 0.2,
        };
        
        if transform.translation.y > half_height + radius {
            // Balloon escaped the scene
            info!("Balloon {} escaped at y={:.2}", target.uuid, transform.translation.y);
            
            if let Some(stats) = stats.as_mut() {
                stats.misses += 1;
                stats_events.write(BroadcastStatsUpdateEvent {
                    session_id: target.session_id,
                    targets_spawned: stats.targets_spawned,
                    targets_popped: stats.targets_popped,
                    misses: stats.misses,
                    score: stats.score,
                });
            }
            
            commands.entity(entity).despawn();
        }
    }
}

/// Save hunter game report to file on game exit
fn save_hunter_report(
    stats: Option<Res<HunterGameStats>>,
    time: Res<Time>,
    scene_setup: Res<SceneSetup>,
) {
    let Some(stats) = stats else { return; };

    let report = generate_game_report(&stats, time.elapsed_secs_f64(), &scene_setup);
    let text = format_report_text(&report);

    let session_id = stats.session_id;
    let stats_dir = format!("stats/hunter/{}", session_id);

    if let Err(e) = std::fs::create_dir_all(&stats_dir) {
        warn!("Failed to create stats directory {}: {}", stats_dir, e);
        return;
    }

    let md_path = format!("{}/report.md", stats_dir);
    match std::fs::write(&md_path, &text) {
        Ok(_) => info!("Hunter game report saved to {}", md_path),
        Err(e) => warn!("Failed to save hunter markdown report {}: {}", md_path, e),
    }

    let json_path = format!("{}/report.json", stats_dir);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => match std::fs::write(&json_path, &json) {
            Ok(_) => info!("Hunter game report (JSON) saved to {}", json_path),
            Err(e) => warn!("Failed to save hunter JSON report {}: {}", json_path, e),
        },
        Err(e) => warn!("Failed to serialize hunter report to JSON: {}", e),
    }
}

fn format_report_text(report: &GameReport) -> String {
    use std::fmt::Write;
    let mut s = String::new();

    writeln!(s, "# Hunter Game Report").unwrap();
    writeln!(s).unwrap();

    // --- Configuration ---
    writeln!(s, "## Configuration").unwrap();

    let scene = &report.scene_setup.scene;
    writeln!(s, "### Scene").unwrap();
    writeln!(s, "- **Dimensions**: {} x {}", scene.scene_dimension.x, scene.scene_dimension.y).unwrap();
    writeln!(s, "- **Origin**: ({:.3}, {:.3}, {:.3})",
        scene.origin.translation.x, scene.origin.translation.y, scene.origin.translation.z).unwrap();
    writeln!(s, "- **Rotation**: ({:.3}, {:.3}, {:.3}, {:.3})",
        scene.origin.rotation.x, scene.origin.rotation.y, scene.origin.rotation.z, scene.origin.rotation.w).unwrap();
    writeln!(s, "- **Y Difference**: {:.3}", scene.y_difference).unwrap();

    let camera = &report.scene_setup.camera;
    writeln!(s, "### Camera").unwrap();
    writeln!(s, "- **Resolution**: {} x {}", camera.resolution.x, camera.resolution.y).unwrap();
    writeln!(s, "- **Position**: ({:.3}, {:.3}, {:.3})",
        camera.origin.translation.x, camera.origin.translation.y, camera.origin.translation.z).unwrap();
    writeln!(s, "- **FOV**: {:.1} deg", camera.angle).unwrap();
    writeln!(s, "- **Locked to Scene**: {}", camera.locked_to_scene).unwrap();

    let proj = &report.scene_setup.projector;
    writeln!(s, "### Projector").unwrap();
    writeln!(s, "- **Resolution**: {} x {}", proj.resolution.x, proj.resolution.y).unwrap();
    writeln!(s, "- **Position**: ({:.3}, {:.3}, {:.3})",
        proj.origin.translation.x, proj.origin.translation.y, proj.origin.translation.z).unwrap();
    writeln!(s, "- **Angle**: {:.1} deg", proj.angle).unwrap();
    writeln!(s, "- **Enabled**: {}", proj.switched_on).unwrap();
    writeln!(s, "- **Connected**: {}", proj.connected).unwrap();
    writeln!(s, "- **Locked to Scene**: {}", proj.locked_to_scene).unwrap();

    // --- Statistics ---
    writeln!(s).unwrap();
    writeln!(s, "## Statistics").unwrap();
    writeln!(s, "- **Game Duration**: {:.2}s", report.total_game_time).unwrap();
    writeln!(s, "- **Targets Spawned**: {}", report.total_targets_spawned).unwrap();
    writeln!(s, "- **Targets Popped**: {}", report.total_targets_popped).unwrap();
    writeln!(s, "- **Misses**: {}", report.total_misses).unwrap();
    writeln!(s, "- **Score**: {}", report.total_score).unwrap();
    writeln!(s, "- **Avg Spawn Interval**: {:.2}s", report.avg_spawn_interval).unwrap();
    writeln!(s, "- **Avg Target Lifetime**: {:.2}s", report.avg_target_lifetime).unwrap();

    // --- Event Timeline ---
    writeln!(s).unwrap();
    writeln!(s, "## Event Timeline (scene coordinates)").unwrap();
    writeln!(s, "| Timestamp | Event | Target UUID | Position |").unwrap();
    writeln!(s, "|-----------|-------|-------------|----------|").unwrap();
    for event in &report.timeline {
        writeln!(s, "| {:.2}s | {} | {} | ({:.3}, {:.3}, {:.3}) |",
            event.timestamp,
            event.event_type,
            event.target_uuid,
            event.position.x,
            event.position.y,
            event.position.z,
        ).unwrap();
    }

    writeln!(s).unwrap();
    writeln!(s, "---").unwrap();
    s
}

fn forward_hunter_stats_to_network(
    mut events: MessageReader<BroadcastStatsUpdateEvent>,
    mut payload_writer: MessageWriter<common::game::BroadcastGameDataPayload>,
) {
    for event in events.read() {
        if let Ok(json) = serde_json::to_string(event) {
            payload_writer.write(common::game::BroadcastGameDataPayload {
                game_id: GAME_ID,
                session_id: event.session_id,
                event_tag: "hunter_stats".to_string(),
                payload_json: json,
            });
        }
    }
}

fn handle_incoming_hunter_payloads(
    mut client_messages: MessageReader<common::network::FromClientMessage>,
    mut spawn_events: MessageWriter<SpawnHunterTargetEvent>,
) {
    for msg in client_messages.read() {
        if let common::network::NetworkMessage::GameDataPayload { game_id, ref event_tag, ref payload_json, .. } = msg.message {
            if game_id == GAME_ID && event_tag == "spawn_target" {
                if let Ok(event) = serde_json::from_str::<SpawnHunterTargetEvent>(payload_json) {
                    spawn_events.write(event);
                }
            }
        }
    }
}


