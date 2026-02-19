use bevy::{app::{App, Plugin, Update}, ecs::{component::Component, entity::Entity, query::{Changed, With}, system::{Commands, Query, Res, ResMut}}, prelude::default, state::{condition::in_state, state::{NextState, OnEnter, OnExit, State}}, ui::Interaction, input::ButtonInput, window::PrimaryWindow};
use bevy_quinnet::client::QuinnetClient;
use common::{network::NetworkMessage, path::UniversalPath, scene::{SceneData, SceneSetup}, state::{GameState, ServerState, TerminalState}, toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem}};
use crate::common::{GAME_ID, HunterGameState, generate_game_report};
use crate::model::{HunterGameStats, CollisionIndicator};
use bevy::prelude::*;

/// Extension trait to add gizmo drawing to UniversalPath
trait UniversalPathGizmos {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform, tolerance: f32);
}

impl UniversalPathGizmos for UniversalPath {
    fn draw_with_gizmos(&self, gizmos: &mut Gizmos, transform: &GlobalTransform, _tolerance: f32) {
        for segment in &self.segments {
            if segment.points.len() < 2 {
                continue;
            }
            
            // Draw lines between consecutive points
            for i in 0..segment.points.len() - 1 {
                let start_point = &segment.points[i];
                let end_point = &segment.points[i + 1];
                
                let start = transform.transform_point(Vec3::new(start_point.x, start_point.y, 0.0));
                let end = transform.transform_point(Vec3::new(end_point.x, end_point.y, 0.0));
                
                let color = Color::srgb(
                    start_point.r as f32 / 255.0,
                    start_point.g as f32 / 255.0,
                    start_point.b as f32 / 255.0,
                );
                
                gizmos.line(start, end, color);
            }
        }
    }
}

const START_GAME_BTN: &str = "start_hunter_game";
const SPAWN_BASIC_TARGET_BTN: &str = "spawn_basic_target";

#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool,
    last_world_pos: Option<Vec3>,
}

#[derive(Component)]
struct MenuButton;

#[derive(Component)]
struct BasicTargetButton;

#[derive(Component)]
struct HunterStatsDisplay;

pub struct HunterTerminalPlugin;

impl Plugin for HunterTerminalPlugin {

    fn build(&self, app: &mut App) {      
        app.init_resource::<DragState>();
        app.add_systems(OnEnter(ServerState::Menu), spawn_menu_toolbar);
        app.add_systems(OnExit(ServerState::Menu), despawn_menu_toolbar);
        app.add_systems(OnEnter(HunterGameState::On), (spawn_basictarget_toolbar_item, spawn_hunter_stats_ui));
        app.add_systems(OnExit(HunterGameState::On), on_hunter_game_finish);
        app.add_systems(OnExit(HunterGameState::On), cleanup_hunter_stats_ui);
        app.add_systems(OnEnter(ServerState::Menu), despawn_basictarget_toolbar_item); 
        app.add_systems(Update, handle_button_click);
        app.add_systems(Update, handle_target_drag.run_if(in_state(HunterGameState::On)));
        app.add_systems(Update, draw_drag_gizmo.run_if(in_state(HunterGameState::On)));
        app.add_systems(Update, update_hunter_stats_display.run_if(in_state(HunterGameState::On)));

   
    }
}

/// Spawns the 'basictarget' toolbar item when entering HunterGameState::On
fn spawn_basictarget_toolbar_item(mut commands: Commands) {
    commands.spawn((
        ToolbarItem {
            name: SPAWN_BASIC_TARGET_BTN.to_string(),
            order: 10,
            icon: Some("\u{f140}".to_string()), // Target/crosshairs icon
            state: ItemState::On,
            docking: Docking::Bottom,
            button_size: 36.0,
            ..default()
        },
        BasicTargetButton,
    ));
}

/// Despawns the 'basictarget' toolbar item when exiting HunterGameState::On
fn despawn_basictarget_toolbar_item(
    mut commands: Commands,
    query: Query<Entity, With<BasicTargetButton>>,
    mut next_state: ResMut<NextState<HunterGameState>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    next_state.set(HunterGameState::Off);
}


fn spawn_menu_toolbar(mut commands: Commands) {
    commands.spawn((
        MenuButton,
        ToolbarItem {
            name: START_GAME_BTN.to_string(),
            order: 1,
            icon: Some("\u{f140}".to_string()), // Target/crosshairs icon
            state: ItemState::On,
            docking: Docking::Left,
            ..default()
        },
    ));
}


fn despawn_menu_toolbar(
    mut commands: Commands,
    button_query: Query<Entity, With<MenuButton>>,
) {
    for entity in &button_query {
        commands.entity(entity).despawn();
    }
}

fn handle_button_click(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {
    for (interaction, toolbar_button) in &button_query {
            if toolbar_button.name == START_GAME_BTN && *interaction == Interaction::Pressed {
            log::info!("'Start Hunter Game' button pressed");
            if *terminal_state.get() == TerminalState::Connected {
                if let Some(connection) = client.get_connection_mut() {
                    // Initialize a Hunter game session with a new UUID and game ID
                    let game_uuid = bevy::asset::uuid::Uuid::new_v4();
                    let message = NetworkMessage::InitGameSession(game_uuid, GAME_ID, GameState::Paused);

                    if let Ok(payload) = message.to_bytes() {
                        if let Err(e) = connection.send_payload(payload) {
                            bevy::log::warn!("Failed to send init Hunter game message: {:?}", e);
                        } else {
                            bevy::log::info!("Sent init Hunter game message (UUID: {}, Name: Hunter)", game_uuid);
                        }
                    }
                }
            } else {
                bevy::log::warn!("Cannot start game: not connected to server");
            }
        }
    }
}

fn handle_target_drag(
    mut drag_state: ResMut<DragState>,
    button_query: Query<( &Interaction, &ToolbarButton)>,
    scene_data: Res<SceneData>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut client: ResMut<QuinnetClient>,
    terminal_state: Res<State<TerminalState>>,
) {

    // Check for drag start from Target button
    for (interaction, button) in &button_query {
        if button.name == SPAWN_BASIC_TARGET_BTN {
            if *interaction == Interaction::Pressed && mouse_button.pressed(MouseButton::Left) {
                info!("Target button interaction: {:?}, mouse pressed: {}", interaction, mouse_button.pressed(MouseButton::Left));
            
                if !drag_state.is_dragging {
                    drag_state.is_dragging = true;
                    drag_state.last_world_pos = scene_data.mouse_world_pos;
                }
            }
        }
    }

    // Check for drag end
    if drag_state.is_dragging && mouse_button.just_released(MouseButton::Left) {
        info!("Drag ended, checking scene data...");
        info!("Scene data found, mouse_world_pos: {:?}", scene_data.mouse_world_pos);
        
        // Drag ended, spawn target at world position if mouse is over scene
        if let Some(world_pos) = scene_data.mouse_world_pos {
            if *terminal_state.get() == TerminalState::Connected {
                if let Some(connection) = client.get_connection_mut() {
                    // Create a basic target with size 0.25 world units
                    let target = common::target::HunterTarget::Basic(0.25, Color::srgb(1.0, 1.0, 1.0));
                    let message = NetworkMessage::SpawnHunterTarget(target, world_pos);

                    if let Ok(payload) = message.to_bytes() {
                        if let Err(e) = connection.send_payload(payload) {
                            bevy::log::warn!("Failed to send spawn target message: {:?}", e);
                        } else {
                            bevy::log::info!("Sent spawn target message at world position {:?}", world_pos);
                            // Server will track stats and broadcast update
                        }
                    }
                }
            } else {
                bevy::log::warn!("Cannot spawn target: not connected to server");
            }
        } else {
            info!("No mouse world position available");
        }
        drag_state.is_dragging = false;
        drag_state.last_world_pos = None;
    }
}

fn draw_drag_gizmo(
    mut drag_state: ResMut<DragState>,
    scene_data: Res<SceneData>,
    scene_setup: Res<SceneSetup>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    if drag_state.is_dragging {
        if let Some(world_pos) = scene_data.mouse_world_pos {
            drag_state.last_world_pos = Some(world_pos);
        } else if let (Ok(window), Ok((camera, camera_transform))) =
            (window_query.single(), camera_query.single())
        {
            let cursor_ray: Option<bevy::math::Ray3d> = window
                .cursor_position()
                .and_then(|cursor_pos| camera.viewport_to_world(camera_transform, cursor_pos).ok());

            if let Some(ray) = cursor_ray {
                let scene_transform: GlobalTransform = Transform::from_translation(scene_setup.scene.origin.translation)
                    .with_rotation(scene_setup.scene.origin.rotation)
                    .with_scale(scene_setup.scene.origin.scale)
                    .into();

                let scene_position = scene_transform.translation();
                let scene_plane: InfinitePlane3d = InfinitePlane3d::new(scene_transform.forward());

                if let Some(distance) = ray.intersect_plane(scene_position, scene_plane) {
                    drag_state.last_world_pos = Some(ray.get_point(distance));
                }
            }
        }

        if let Some(world_pos) = drag_state.last_world_pos {
            // Draw a white circle with radius 0.125 (diameter 0.25)
            let path = UniversalPath::circle(Vec2::ZERO, 0.125, Color::WHITE);
            let global_transform = GlobalTransform::from(Transform::from_translation(world_pos));
            path.draw_with_gizmos(&mut gizmos, &global_transform, 0.1);
        }
    }
}

/// Spawn collision indicator at click position
fn spawn_collision_indicator(commands: &mut Commands, position: Vec3) {
    let indicator_path = UniversalPath::circle(
        Vec2::new(position.x, position.y),
        0.02, // 4cm diameter (small marker)
        Color::srgb(1.0, 0.0, 0.0) // Red color
    );
    
    commands.spawn((
        CollisionIndicator,
        Transform::from_translation(position),
        GlobalTransform::default(),
        Visibility::default(),
        indicator_path,
        common::path::PathRenderable::default(),
    ));
}

/// Spawn stats UI in bottom toolbar
fn spawn_hunter_stats_ui(mut commands: Commands) {
    commands.spawn((
        HunterStatsDisplay,
        Text::new("Spawned: 0 | Hits: 0 | Points: 0"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(5.0),
            ..default()
        },
        ZIndex(1000),
    ));
}

/// Update stats display in real-time
fn update_hunter_stats_display(
    stats: Res<HunterGameStats>,
    mut query: Query<&mut Text, With<HunterStatsDisplay>>,
) {
    if stats.is_changed() {
        if let Ok(mut text) = query.single_mut() {
            **text = format!(
                "Spawned: {} | Hits: {} | Points: {}",
                stats.targets_spawned,
                stats.targets_popped,
                stats.score
            );
        }
    }
}

/// Cleanup stats UI on game exit
fn cleanup_hunter_stats_ui(
    mut commands: Commands,
    query: Query<Entity, With<HunterStatsDisplay>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Generate and log post-game report on game finish
fn on_hunter_game_finish(
    stats: Res<HunterGameStats>,
    time: Res<Time>,
) {
    let report = generate_game_report(&*stats, time.elapsed_secs_f64());
    
    // Log report header
    info!("=== HUNTER GAME REPORT ===");
    info!("Game Duration: {:.2}s", report.total_game_time);
    info!("Targets Spawned: {}", report.total_targets_spawned);
    info!("Targets Popped: {}", report.total_targets_popped);
    info!("Total Score: {}", report.total_score);
    info!("Average Spawn Interval: {:.2}s", report.avg_spawn_interval);
    info!("Average Target Lifetime: {:.2}s", report.avg_target_lifetime);
    info!("");
    
    // Log event timeline
    info!("EVENT TIMELINE:");
    for event in &report.timeline {
        info!(
            "[{:.2}s] {} target {} at ({:.2}, {:.2}, {:.2})",
            event.timestamp,
            event.event_type,
            event.target_uuid,
            event.position.x,
            event.position.y,
            event.position.z
        );
    }
    info!("=== END REPORT ===");
}
