use bevy::prelude::*;
use common::path::{LaserTextOptions, PathRenderable, UniversalPath};
use common::scene::{SceneEntity, SceneSetup};
use common::state::{GameState, ServerState};

use crate::model::*;

#[derive(Component)]
struct SnakeTitleAnnouncement {
    timer: Timer,
}

pub struct SnakeGameServerPlugin;

impl Plugin for SnakeGameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ChangeSnakeDirectionEvent>();
        app.add_message::<BroadcastSnakeStatsEvent>();
        app.add_message::<SnakeGameOverEvent>();
        app.add_systems(
            Update,
            (
                init_snake_game,
                handle_direction_input,
                handle_snake_game_over,
                animate_snake_title_announcement,
            ),
        );
        app.add_systems(FixedUpdate, snake_move_tick);
        app.add_systems(OnExit(ServerState::InGame), (save_snake_report, cleanup_snake_game).chain());
        app.add_systems(OnExit(GameState::InGame), cleanup_snake_game);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert grid cell to scene-local position (centred on origin)
fn grid_to_local(cell: IVec2, grid_w: i32, grid_h: i32) -> Vec3 {
    let half_w = (grid_w as f32 * CELL_SIZE) / 2.0;
    let half_h = (grid_h as f32 * CELL_SIZE) / 2.0;
    Vec3::new(
        (cell.x as f32 + 0.5) * CELL_SIZE - half_w,
        (cell.y as f32 + 0.5) * CELL_SIZE - half_h,
        0.0,
    )
}

fn random_color() -> (f32, f32, f32) {
    use rand::random_range;
    // Generate vivid colours by picking from a set of saturated hues
    let hue = random_range(0.0f32..360.0);
    let (r, g, b) = hsl_to_rgb(hue, 0.9, 0.55);
    (r, g, b)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h as i32 / 60) % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

fn random_gem_position(snake: &SnakeState) -> IVec2 {
    use rand::random_range;
    loop {
        let x = random_range(0..snake.grid_w);
        let y = random_range(0..snake.grid_h);
        let pos = IVec2::new(x, y);
        if !snake.segments.contains(&pos) {
            return pos;
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Initialise the snake game when a new session with our game_id is created
fn init_snake_game(
    mut commands: Commands,
    mut created_events: MessageReader<common::game::GameSessionCreated>,
    mut update_events: MessageReader<common::game::GameSessionUpdate>,
    existing_state: Option<Res<SnakeState>>,
    scene_setup: Res<SceneSetup>,
    scene_query: Query<Entity, With<SceneEntity>>,
    // clean up any prior entities
    head_query: Query<Entity, With<SnakeHead>>,
    seg_query: Query<Entity, With<SnakeSegment>>,
    gem_query: Query<Entity, With<DiamondFood>>,
    title_query: Query<Entity, With<SnakeTitleAnnouncement>>,
    border_query: Query<Entity, With<SnakeBorder>>,
    mut stats_events: MessageWriter<BroadcastSnakeStatsEvent>,
) {
    let mut should_init: Option<bevy::asset::uuid::Uuid> = None;
    let mut other_game_started = false;

    for event in created_events.read() {
        if event.game_session.game_id == GAME_ID && event.game_session.state == GameState::InGame {
            should_init = Some(event.game_session.session_id);
        } else {
            other_game_started = true;
        }
    }
    for event in update_events.read() {
        if event.game_session.game_id == GAME_ID && event.game_session.state == GameState::InGame && existing_state.is_none() {
            should_init = Some(event.game_session.session_id);
        }
    }

    if other_game_started {
        // Clean previous Snake entities when another game (e.g. Hunter) starts
        for e in head_query
            .iter()
            .chain(seg_query.iter())
            .chain(gem_query.iter())
            .chain(title_query.iter())
            .chain(border_query.iter())
        {
            commands.entity(e).despawn();
        }
        if existing_state.is_some() {
            commands.remove_resource::<SnakeState>();
        }
    }

    let Some(session_id) = should_init else {
        return;
    };

    // Clean previous entities (but NOT the border — it's static for the whole session)
    for e in head_query
        .iter()
        .chain(seg_query.iter())
        .chain(gem_query.iter())
        .chain(title_query.iter())
    {
        commands.entity(e).despawn();
    }

    // Compute grid from scene dimensions — clamp so grid exactly fits inside scene
    let dim = scene_setup.scene.scene_dimension; // UVec2 in metres
    let grid_w = ((dim.x as f32 / CELL_SIZE).floor() as i32).max(4);
    let grid_h = ((dim.y as f32 / CELL_SIZE).floor() as i32).max(4);
    // Ensure the occupied area is strictly <= scene area
    let actual_w = grid_w as f32 * CELL_SIZE;
    let actual_h = grid_h as f32 * CELL_SIZE;

    let start = IVec2::new(grid_w / 2, grid_h / 2);

    let mut state = SnakeState {
        segments: vec![
            start,
            start + IVec2::new(-1, 0),
            start + IVec2::new(-2, 0),
        ],
        segment_colors: vec![
            (1.0, 1.0, 1.0), // head = white
            (0.6, 0.6, 0.6), // initial body gray
            (0.6, 0.6, 0.6),
        ],
        direction: SnakeDirection::Right,
        queued_direction: None,
        gem_position: IVec2::ZERO, // placeholder
        gem_color: (0.0, 0.0, 0.0),
        gems_eaten: 0,
        grid_w,
        grid_h,
        session_id,
        is_started: false,
        game_over: false,
    };

    // Place first gem
    state.gem_position = random_gem_position(&state);
    let gc = random_color();
    state.gem_color = gc;

    // Spawn path entities for each segment + gem
    let scene_entity = scene_query.single().ok();

    // Spawn head
    spawn_head_entity(&mut commands, &state, scene_entity);

    // Spawn initial body segments
    for i in 1..state.segments.len() {
        spawn_segment_entity(&mut commands, &state, i, scene_entity);
    }

    // Spawn gem
    spawn_gem_entity(&mut commands, &state, scene_entity);

    // Spawn scene border rectangle so the game area boundary is visible
    spawn_border_entity(&mut commands, scene_entity, actual_w, actual_h);

    // Spawn full-scene center vector title announcement
    spawn_snake_title_entity(&mut commands, &scene_setup, scene_entity);

    // Insert resources
    commands.insert_resource(state.clone());
    commands.insert_resource(SnakeMoveTimer::new(INITIAL_TICK_INTERVAL));

    stats_events.write(BroadcastSnakeStatsEvent {
        session_id,
        score: 0,
        length: 3,
        game_over: false,
    });

    info!("Snake game initialized: {}x{} grid, session {}", grid_w, grid_h, session_id);
}

fn spawn_snake_title_entity(commands: &mut Commands, scene_setup: &SceneSetup, scene_entity: Option<Entity>) {
    let scene_dim = scene_setup.scene.scene_dimension;
    // Height-based cap: 55% of scene height
    let height_cap = scene_dim.y as f32 * 0.55;
    // Width-based cap: fit "SNAKE" (5 chars) within 85% of scene width.
    // Century Gothic char width ≈ 0.65× height, plus letter_spacing (0.08) per char.
    let num_chars = 5usize;
    let char_width_ratio = 0.65_f32;
    let letter_spacing = 0.08_f32;
    let total_width_per_unit = num_chars as f32 * (char_width_ratio + letter_spacing);
    let width_cap = (scene_dim.x as f32 * 0.85) / total_width_per_unit;
    let text_height = height_cap.min(width_cap).clamp(0.8, 3.5);

    let options = LaserTextOptions {
        origin: Vec2::ZERO,
        height: text_height,
        color: Color::srgb(0.2, 1.0, 0.4), // Bright green laser title path
        center_on_origin: true,
        ..Default::default()
    };

    let font_paths = [
        // Project-bundled fonts (checked first)
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
            if let Ok(text_path) = UniversalPath::from_ttf_text(&data, "SNAKE", &options) {
                info!("✓ [Snake] Rendered full-scene center vector title using font {}", path);
                maybe_title_path = Some(text_path);
                break;
            }
        }
    }

    let Some(title_path) = maybe_title_path else {
        warn!("No usable TTF font found for SNAKE title");
        return;
    };

    let id = commands
        .spawn((
            SnakeTitleAnnouncement {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            title_path,
            PathRenderable::default(),
        ))
        .id();

    if let Some(scene) = scene_entity {
        commands.entity(scene).add_child(id);
    }
}

fn animate_snake_title_announcement(
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

fn spawn_head_entity(commands: &mut Commands, state: &SnakeState, scene_entity: Option<Entity>) {
    let pos = grid_to_local(state.segments[0], state.grid_w, state.grid_h);
    let (r, g, b) = state.segment_colors.get(0).copied().unwrap_or((1.0, 1.0, 1.0));
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

fn spawn_segment_entity(
    commands: &mut Commands,
    state: &SnakeState,
    index: usize,
    scene_entity: Option<Entity>,
) {
    let cell = state.segments[index];
    let (r, g, b) = state.segment_colors[index];
    let color = Color::srgb(r, g, b);
    let pos = grid_to_local(cell, state.grid_w, state.grid_h);
    let path = UniversalPath::circle(Vec2::ZERO, SEGMENT_RADIUS * 0.85, color);
    let id = commands
        .spawn((
            SnakeSegment { color },
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

fn spawn_gem_entity(commands: &mut Commands, state: &SnakeState, scene_entity: Option<Entity>) {
    let (r, g, b) = state.gem_color;
    let color = Color::srgb(r, g, b);
    let pos = grid_to_local(state.gem_position, state.grid_w, state.grid_h);
    // Diamond = rectangle rotated 45°
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

fn spawn_border_entity(commands: &mut Commands, scene_entity: Option<Entity>, w: f32, h: f32) {
    // Draw the snake play-field boundary as a dim white rectangle
    let half_w = w * 0.5;
    let half_h = h * 0.5;
    let border_color = Color::srgb(0.25, 0.25, 0.25);
    let top_left = Vec2::new(-half_w, -half_h);
    let size = Vec2::new(w, h);
    let path = UniversalPath::rectangle(top_left, size, border_color);
    let id = commands
        .spawn((
            SnakeBorder, // dedicated marker — excluded from per-tick segment despawn
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

/// Handle direction change events from keyboard input
fn handle_direction_input(
    mut direction_events: MessageReader<ChangeSnakeDirectionEvent>,
    mut snake_state: Option<ResMut<SnakeState>>,
) {
    let Some(ref mut state) = snake_state else {
        return;
    };
    if state.game_over {
        return;
    }
    for event in direction_events.read() {
        let new_dir = event.direction;
        // Don't allow reversing into yourself
        if !new_dir.is_opposite(state.direction) {
            state.queued_direction = Some(new_dir);
            state.is_started = true;
        }
    }
}

/// Main game tick: move snake, check collisions, grow, spawn gem
fn snake_move_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut timer_res: Option<ResMut<SnakeMoveTimer>>,
    mut snake_state: Option<ResMut<SnakeState>>,
    scene_query: Query<Entity, With<SceneEntity>>,
    // queries to despawn/respawn entities
    head_query: Query<Entity, With<SnakeHead>>,
    seg_query: Query<Entity, With<SnakeSegment>>,
    gem_query: Query<Entity, With<DiamondFood>>,
    mut stats_events: MessageWriter<BroadcastSnakeStatsEvent>,
    mut game_over_events: MessageWriter<SnakeGameOverEvent>,
) {
    let Some(ref mut timer) = timer_res else {
        return;
    };
    let Some(ref mut state) = snake_state else {
        return;
    };
    if !state.is_started || state.game_over {
        return;
    }

    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    // Apply queued direction
    if let Some(dir) = state.queued_direction.take() {
        state.direction = dir;
    }

    // Calculate new head position
    let delta = state.direction.delta();
    let old_head = state.segments[0];
    let mut new_head = old_head + delta;

    // Wrap around edges
    if new_head.x < 0 {
        new_head.x = state.grid_w - 1;
    } else if new_head.x >= state.grid_w {
        new_head.x = 0;
    }
    if new_head.y < 0 {
        new_head.y = state.grid_h - 1;
    } else if new_head.y >= state.grid_h {
        new_head.y = 0;
    }

    // Self-collision check (check against all body segments, not the tail that will move)
    // We check against segments[0..len-1] because the tail will move away
    let ate_gem = new_head == state.gem_position;
    let body_to_check = if ate_gem {
        // If growing, tail stays, so check all segments
        &state.segments[..]
    } else {
        // Tail will move away, so exclude last segment
        &state.segments[..state.segments.len() - 1]
    };

    if body_to_check.contains(&new_head) {
        // Game over!
        state.game_over = true;
        info!("Snake game over! Score: {}", state.gems_eaten);

        game_over_events.write(SnakeGameOverEvent {
            session_id: state.session_id,
            final_score: state.gems_eaten,
        });
        stats_events.write(BroadcastSnakeStatsEvent {
            session_id: state.session_id,
            score: state.gems_eaten,
            length: state.segments.len() as u32,
            game_over: true,
        });
        return;
    }

    // Move: insert new head, optionally remove tail
    state.segments.insert(0, new_head);
    state.segment_colors.insert(0, (1.0, 1.0, 1.0)); // head always white

    if ate_gem {
        // Snake turns the color of the eaten diamond rectangle
        let eaten_color = state.gem_color;
        for col in state.segment_colors.iter_mut() {
            *col = eaten_color;
        }
        state.gems_eaten += 1;

        // Speed up
        let new_interval =
            (INITIAL_TICK_INTERVAL - SPEED_UP_PER_GEM * state.gems_eaten as f32).max(MIN_TICK_INTERVAL);
        timer.timer = Timer::from_seconds(new_interval, TimerMode::Repeating);

        info!(
            "Snake ate gem! Score: {}, new interval: {:.3}s",
            state.gems_eaten, new_interval
        );

        // New gem
        state.gem_position = random_gem_position(state);
        let gc = random_color();
        state.gem_color = gc;
    } else {
        // Remove tail
        state.segments.pop();
        state.segment_colors.pop();
    }

    // Re-render: despawn all visual entities and recreate
    // (simple approach – works well since the PathNetworkPlugin auto-broadcasts)
    for e in head_query.iter().chain(seg_query.iter()) {
        commands.entity(e).despawn();
    }
    if ate_gem {
        for e in gem_query.iter() {
            commands.entity(e).despawn();
        }
    }

    let scene_entity = scene_query.single().ok();

    // Spawn head
    spawn_head_entity(&mut commands, state, scene_entity);

    // Spawn body
    for i in 1..state.segments.len() {
        spawn_segment_entity(&mut commands, state, i, scene_entity);
    }

    // Spawn new gem if eaten
    if ate_gem {
        spawn_gem_entity(&mut commands, state, scene_entity);
    }

    // Broadcast stats
    stats_events.write(BroadcastSnakeStatsEvent {
        session_id: state.session_id,
        score: state.gems_eaten,
        length: state.segments.len() as u32,
        game_over: false,
    });
}

/// Handle game over: trigger exit after a short delay or immediately
fn handle_snake_game_over(
    mut game_over_events: MessageReader<SnakeGameOverEvent>,
) {
    for event in game_over_events.read() {
        info!("Snake game over event received. Final score: {}", event.final_score);
    }
}

/// Cleanup all snake entities and resources when leaving InGame
fn cleanup_snake_game(
    mut commands: Commands,
    head_query: Query<Entity, With<SnakeHead>>,
    seg_query: Query<Entity, With<SnakeSegment>>,
    gem_query: Query<Entity, With<DiamondFood>>,
    title_query: Query<Entity, With<SnakeTitleAnnouncement>>,
    state: Option<Res<SnakeState>>,
    timer: Option<Res<SnakeMoveTimer>>,
) {
    for e in head_query.iter().chain(seg_query.iter()).chain(gem_query.iter()).chain(title_query.iter()) {
        commands.entity(e).despawn();
    }
    if state.is_some() {
        commands.remove_resource::<SnakeState>();
    }
    if timer.is_some() {
        commands.remove_resource::<SnakeMoveTimer>();
    }
    info!("Snake game cleaned up");
}

/// Save snake game stats to file on game exit
fn save_snake_report(
    state: Option<Res<SnakeState>>,
) {
    let Some(state) = state else { return; };

    let session_id = state.session_id;
    let stats_dir = format!("stats/snake/{}", session_id);

    if let Err(e) = std::fs::create_dir_all(&stats_dir) {
        warn!("Failed to create snake stats directory {}: {}", stats_dir, e);
        return;
    }

    // Prepare markdown report
    let mut text = String::new();
    text.push_str("# Snake Game Report\n\n");
    text.push_str(&format!("- **Session ID**: {}\n", session_id));
    text.push_str(&format!("- **Final Score (Gems Eaten)**: {}\n", state.gems_eaten));
    text.push_str(&format!("- **Final Length**: {}\n", state.segments.len()));
    text.push_str(&format!("- **Grid Dimensions**: {}x{}\n", state.grid_w, state.grid_h));
    text.push_str(&format!("- **Game Over State**: {}\n", state.game_over));
    text.push_str("\n---");

    let md_path = format!("{}/report.md", stats_dir);
    match std::fs::write(&md_path, &text) {
        Ok(_) => info!("Snake game report saved to {}", md_path),
        Err(e) => warn!("Failed to save snake markdown report {}: {}", md_path, e),
    }

    let json_path = format!("{}/report.json", stats_dir);
    match serde_json::to_string_pretty(&*state) {
        Ok(json) => match std::fs::write(&json_path, &json) {
            Ok(_) => info!("Snake game report (JSON) saved to {}", json_path),
            Err(e) => warn!("Failed to save snake JSON report {}: {}", json_path, e),
        },
        Err(e) => warn!("Failed to serialize snake state to JSON: {}", e),
    }
}
