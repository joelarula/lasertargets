use bevy::prelude::*;
use common::path::UniversalPath;
use common::scene::{SceneEntity, SceneSetup};

use crate::components::{DiamondFood, SnakeHead, SnakeSegment};
use crate::events::{BroadcastSnakeStatsEvent, SnakeGameOverEvent};
use crate::resources::{SnakeMoveTimer, SnakeState};
use crate::systems::render::{
    build_snake_body_path, grid_to_local, random_color, random_gem_position,
    spawn_gem_entity, spawn_head_entity, spawn_snake_body_entity, spawn_snake_text_announcement,
};
use crate::types::{GEM_HALF_SIZE, INITIAL_TICK_INTERVAL, MIN_TICK_INTERVAL, SEGMENT_RADIUS, SPEED_UP_PER_GEM};

/// Main game tick: move snake, check collisions, grow, spawn gem
pub fn snake_move_tick(
    mut commands: Commands,
    time: Res<Time>,
    scene_setup: Res<SceneSetup>,
    timer_res: Option<ResMut<SnakeMoveTimer>>,
    snake_state: Option<ResMut<SnakeState>>,
    scene_query: Query<Entity, With<SceneEntity>>,
    // queries to update entities in-place (avoiding despawn gaps that black out the laser)
    mut head_query: Query<(Entity, &mut UniversalPath, &mut Transform), (With<SnakeHead>, Without<SnakeSegment>, Without<DiamondFood>)>,
    mut seg_query: Query<(Entity, &mut UniversalPath), (With<SnakeSegment>, Without<SnakeHead>, Without<DiamondFood>)>,
    mut gem_query: Query<(Entity, &mut UniversalPath, &mut Transform), (With<DiamondFood>, Without<SnakeHead>, Without<SnakeSegment>)>,
    mut stats_events: MessageWriter<BroadcastSnakeStatsEvent>,
    mut game_over_events: MessageWriter<SnakeGameOverEvent>,
) {
    let (Some(ref mut timer), Some(ref mut state)) = (timer_res, snake_state) else {
        return;
    };

    if state.game_over {
        handle_game_over_reset(
            &mut commands,
            &time,
            state,
            timer,
            &scene_query,
            &head_query,
            &seg_query,
            &gem_query,
        );
        return;
    }

    if !state.is_started {
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

    // Calculate new head position with wrap-around
    let new_head = calculate_next_head_pos(state);

    let ate_gem = new_head == state.gem_position;
    let body_to_check = if ate_gem {
        &state.segments[..]
    } else {
        &state.segments[..state.segments.len().saturating_sub(1)]
    };

    if body_to_check.contains(&new_head) {
        trigger_game_over(
            &mut commands,
            &scene_setup,
            state,
            &scene_query,
            &mut game_over_events,
            &mut stats_events,
        );
        return;
    }

    // Move snake and update segments
    advance_snake_segments(state, new_head, ate_gem, timer);

    // Synchronize renderable entities in place
    sync_laser_entities(
        &mut commands,
        state,
        &scene_query,
        &mut head_query,
        &mut seg_query,
        &mut gem_query,
    );

    // Broadcast stats
    stats_events.write(BroadcastSnakeStatsEvent {
        session_id: state.session_id,
        score: state.gems_eaten,
        length: state.segments.len() as u32,
        game_over: false,
    });
}

fn calculate_next_head_pos(state: &SnakeState) -> IVec2 {
    let delta = state.direction.delta();
    let old_head = state.segments[0];
    let mut new_head = old_head + delta;

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
    new_head
}

fn trigger_game_over(
    commands: &mut Commands,
    scene_setup: &SceneSetup,
    state: &mut SnakeState,
    scene_query: &Query<Entity, With<SceneEntity>>,
    game_over_events: &mut MessageWriter<SnakeGameOverEvent>,
    stats_events: &mut MessageWriter<BroadcastSnakeStatsEvent>,
) {
    state.game_over = true;
    state.game_over_reset_timer = Some(Timer::from_seconds(2.5, TimerMode::Once));
    info!("★ [Snake] Game over triggered! Score: {}. Showing GAME OVER announcement...", state.gems_eaten);

    let scene_entity = scene_query.single().ok();
    spawn_snake_text_announcement(commands, scene_setup, scene_entity, "GAME OVER", Color::srgb(1.0, 0.2, 0.2), 2.5);

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
}

fn advance_snake_segments(
    state: &mut SnakeState,
    new_head: IVec2,
    ate_gem: bool,
    timer: &mut SnakeMoveTimer,
) {
    state.segments.insert(0, new_head);
    let head_col = state.segment_colors.first().copied().unwrap_or((1.0, 1.0, 1.0));
    state.segment_colors.insert(0, head_col);

    if ate_gem {
        let eaten_color = state.gem_color;
        for col in state.segment_colors.iter_mut() {
            *col = eaten_color;
        }
        state.gems_eaten += 1;
        state.pending_growth += 2;

        let new_interval =
            (INITIAL_TICK_INTERVAL - SPEED_UP_PER_GEM * state.gems_eaten as f32).max(MIN_TICK_INTERVAL);
        timer.timer = Timer::from_seconds(new_interval, TimerMode::Repeating);

        info!(
            "Snake ate gem! Score: {}, new length: {}, interval: {:.3}s",
            state.gems_eaten, state.segments.len() + state.pending_growth, new_interval
        );

        state.gem_position = random_gem_position(state);
        state.gem_color = random_color();
    }

    if state.pending_growth > 0 {
        state.pending_growth -= 1;
    } else {
        state.segments.pop();
        state.segment_colors.pop();
    }
}

fn sync_laser_entities(
    commands: &mut Commands,
    state: &SnakeState,
    scene_query: &Query<Entity, With<SceneEntity>>,
    head_query: &mut Query<(Entity, &mut UniversalPath, &mut Transform), (With<SnakeHead>, Without<SnakeSegment>, Without<DiamondFood>)>,
    seg_query: &mut Query<(Entity, &mut UniversalPath), (With<SnakeSegment>, Without<SnakeHead>, Without<DiamondFood>)>,
    gem_query: &mut Query<(Entity, &mut UniversalPath, &mut Transform), (With<DiamondFood>, Without<SnakeHead>, Without<SnakeSegment>)>,
) {
    let scene_entity = scene_query.single().ok();

    // Update head
    let head_pos = grid_to_local(state.segments[0], state.grid_w, state.grid_h);
    let (hr, hg, hb) = state.segment_colors.first().copied().unwrap_or((1.0, 1.0, 1.0));
    let head_color = Color::srgb(hr, hg, hb);
    {
        let mut heads: Vec<_> = head_query.iter_mut().collect();
        for (e, _, _) in heads.iter_mut().skip(1) {
            commands.entity(*e).despawn();
        }
        if let Some((_, mut head_path, mut head_transform)) = heads.into_iter().next() {
            *head_path = UniversalPath::circle(Vec2::ZERO, SEGMENT_RADIUS, head_color);
            *head_transform = Transform::from_translation(head_pos);
        } else {
            spawn_head_entity(commands, state, scene_entity);
        }
    }

    // Update body
    let new_body = build_snake_body_path(state);
    {
        let mut segs: Vec<_> = seg_query.iter_mut().collect();
        for (e, _) in segs.iter_mut().skip(1) {
            commands.entity(*e).despawn();
        }
        if let Some((_, mut seg_path)) = segs.into_iter().next() {
            *seg_path = new_body;
        } else {
            spawn_snake_body_entity(commands, state, scene_entity);
        }
    }

    // Update gem
    {
        let gem_pos = grid_to_local(state.gem_position, state.grid_w, state.grid_h);
        let (gr, gg, gb) = state.gem_color;
        let gem_color = Color::srgb(gr, gg, gb);
        let mut gems: Vec<_> = gem_query.iter_mut().collect();
        for (e, _, _) in gems.iter_mut().skip(1) {
            commands.entity(*e).despawn();
        }
        if let Some((_, mut gem_path, mut gem_transform)) = gems.into_iter().next() {
            *gem_path = UniversalPath::diamond(Vec2::ZERO, GEM_HALF_SIZE, gem_color);
            *gem_transform = Transform::from_translation(gem_pos);
        } else {
            spawn_gem_entity(commands, state, scene_entity);
        }
    }
}

fn handle_game_over_reset(
    commands: &mut Commands,
    time: &Time,
    state: &mut SnakeState,
    timer: &mut SnakeMoveTimer,
    scene_query: &Query<Entity, With<SceneEntity>>,
    head_query: &Query<(Entity, &mut UniversalPath, &mut Transform), (With<SnakeHead>, Without<SnakeSegment>, Without<DiamondFood>)>,
    seg_query: &Query<(Entity, &mut UniversalPath), (With<SnakeSegment>, Without<SnakeHead>, Without<DiamondFood>)>,
    gem_query: &Query<(Entity, &mut UniversalPath, &mut Transform), (With<DiamondFood>, Without<SnakeHead>, Without<SnakeSegment>)>,
) {
    if let Some(ref mut reset_timer) = state.game_over_reset_timer {
        reset_timer.tick(time.delta());
        if reset_timer.just_finished() {
            info!("★ [Snake] Auto-resetting game after GAME OVER screen...");
            let start = IVec2::new(state.grid_w / 2, state.grid_h / 2);
            state.segments = vec![start, start + IVec2::new(-1, 0), start + IVec2::new(-2, 0)];
            state.segment_colors = vec![(1.0, 1.0, 1.0), (0.6, 0.6, 0.6), (0.6, 0.6, 0.6)];
            state.direction = crate::types::SnakeDirection::Right;
            state.queued_direction = None;
            state.gems_eaten = 0;
            state.pending_growth = 0;
            state.game_over = false;
            state.game_over_reset_timer = None;
            state.is_started = true;
            timer.timer = Timer::from_seconds(INITIAL_TICK_INTERVAL, TimerMode::Repeating);

            for (e, _, _) in head_query.iter() { if let Ok(mut entity_cmds) = commands.get_entity(e) { entity_cmds.despawn(); } }
            for (e, _) in seg_query.iter() { if let Ok(mut entity_cmds) = commands.get_entity(e) { entity_cmds.despawn(); } }
            for (e, _, _) in gem_query.iter() { if let Ok(mut entity_cmds) = commands.get_entity(e) { entity_cmds.despawn(); } }
            let scene_entity = scene_query.single().ok();
            state.gem_position = random_gem_position(state);
            state.gem_color = random_color();
            spawn_head_entity(commands, state, scene_entity);
            spawn_snake_body_entity(commands, state, scene_entity);
            spawn_gem_entity(commands, state, scene_entity);
        }
    }
}
