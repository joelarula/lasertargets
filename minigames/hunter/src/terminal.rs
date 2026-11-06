use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use common::curator_core::CuratorEventEnvelope;
use common::network::{NetworkMessage, ToServer};
use common::scene::{SceneData, SceneSetup};
use common::state::{ServerState, TerminalState};
use terminal_ui::{Docking, ToolbarItem, ToolbarPressed};

use crate::audit::HunterGameStats;
use crate::common::HunterGameState;
use crate::events::{BroadcastStatsUpdateEvent, SpawnHunterTargetEvent};
use crate::types::HunterTarget;

/// Dropped where a drag from the target button ends
const DRAG_TARGET: HunterTarget = HunterTarget::Basic(0.25, Color::WHITE);
/// Released by the balloon button, at the scene origin
const BUTTON_BALLOON: HunterTarget = HunterTarget::Balloon(0.2, Color::srgb(1.0, 0.0, 0.0));

/// A drag from the target button is in progress; the last scene point under the cursor
#[derive(Resource, Default)]
struct Drag(Option<Option<Vec3>>);

/// In-game toolbar buttons and the stats line
#[derive(Component)]
struct InGameUi;

#[derive(Component)]
struct HunterStatsDisplay;

/// Terminal side of Hunter: the start button, the in-game target buttons (drag one onto the
/// scene, or release a balloon) and the stats line
pub struct HunterTerminalPlugin;

impl Plugin for HunterTerminalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Drag>()
            .add_systems(OnEnter(ServerState::Menu), spawn_menu_toolbar)
            .add_systems(OnEnter(HunterGameState::On), spawn_in_game_ui)
            .add_systems(OnExit(HunterGameState::On), despawn_all::<InGameUi>)
            .add_systems(OnEnter(TerminalState::Connecting), (despawn_all::<InGameUi>, clear_stats))
            .add_systems(Update, receive_stats)
            .add_systems(
                Update,
                (end_target_drag, draw_drag_gizmo, update_stats_display).run_if(in_state(HunterGameState::On)),
            );
    }
}

fn despawn_all<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn clear_stats(mut commands: Commands) {
    commands.remove_resource::<HunterGameStats>();
}

/// The server sends the running totals as a `hunter_stats` curator envelope
fn receive_stats(mut envelopes: MessageReader<CuratorEventEnvelope>, mut commands: Commands) {
    for event in envelopes.read().filter_map(|e| e.downcast::<BroadcastStatsUpdateEvent>().ok()) {
        commands.insert_resource(HunterGameStats::from_tally(event.session_id, event.tally));
    }
}

fn spawn_request(target: HunterTarget, position: Vec3) -> ToServer {
    ToServer(NetworkMessage::CuratorEnvelope(CuratorEventEnvelope::from_payload(
        &SpawnHunterTargetEvent { target, position },
        None,
    )))
}

fn spawn_menu_toolbar(mut commands: Commands) {
    commands
        .spawn((
            // Target/crosshairs icon
            ToolbarItem::new("start_hunter_game").icon("\u{f140}").order(1).on(true),
            DespawnOnExit(ServerState::Menu),
        ))
        .observe(|_: On<ToolbarPressed>, mut to_server: MessageWriter<ToServer>| {
            let session_id = bevy::asset::uuid::Uuid::new_v4();
            info!("Opening Hunter session {session_id}");
            to_server.write(ToServer(NetworkMessage::OpenSession { session_id, channel_name: "hunter".to_string() }));
        });
}

fn spawn_in_game_ui(mut commands: Commands, stats: Option<Res<HunterGameStats>>) {
    let bottom = |name: &str, order: u8, icon: &str| {
        (InGameUi, ToolbarItem::new(name).icon(icon).order(order).dock(Docking::Bottom).on(true))
    };
    // Pressing starts a drag; releasing over the scene drops the target there
    commands
        .spawn(bottom("spawn_basic_target", 10, "\u{f140}"))
        .observe(|_: On<ToolbarPressed>, mut drag: ResMut<Drag>| drag.0 = Some(None));
    commands
        .spawn(bottom("spawn_balloon_target", 11, "\u{f5a4}"))
        .observe(|_: On<ToolbarPressed>, mut to_server: MessageWriter<ToServer>| {
            to_server.write(spawn_request(BUTTON_BALLOON, Vec3::ZERO));
        });
    commands.spawn((
        InGameUi,
        HunterStatsDisplay,
        Text::new(stats.map(|s| s.tally).unwrap_or_default().to_string()),
        TextFont { font_size: 16.0, ..default() },
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

fn end_target_drag(mut drag: ResMut<Drag>, mouse: Res<ButtonInput<MouseButton>>, mut to_server: MessageWriter<ToServer>) {
    if drag.0.is_some() && mouse.just_released(MouseButton::Left) {
        match drag.0.take().flatten() {
            Some(at) => {
                to_server.write(spawn_request(DRAG_TARGET, at));
            }
            None => info!("Target drag ended off the scene"),
        }
    }
}

/// Follows the drag with the cursor on the scene plane and outlines the target there
fn draw_drag_gizmo(
    mut drag: ResMut<Drag>,
    scene_data: Res<SceneData>,
    scene_setup: Res<SceneSetup>,
    camera: Query<(&Camera, &GlobalTransform)>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    let Some(last) = drag.0.as_mut() else { return };
    let origin = &scene_setup.scene.origin;
    let on_scene_plane = || {
        let (camera, camera_transform) = camera.single().ok()?;
        let ray = camera.viewport_to_world(camera_transform, window.single().ok()?.cursor_position()?).ok()?;
        let normal = Dir3::new(origin.rotation * Vec3::Z).ok()?;
        let distance = ray.intersect_plane(origin.translation, InfinitePlane3d::new(normal))?;
        Some(ray.get_point(distance))
    };
    if let Some(at) = scene_data.mouse_world_pos.or_else(on_scene_plane) {
        *last = Some(at);
    }
    if let Some(at) = *last {
        gizmos.circle(Isometry3d::new(at, origin.rotation), DRAG_TARGET.radius(), Color::WHITE);
    }
}

fn update_stats_display(
    stats: Option<Res<HunterGameStats>>,
    mut text: Query<&mut Text, With<HunterStatsDisplay>>,
) {
    let Some(stats) = stats.filter(|s| s.is_changed()) else { return };
    for mut text in &mut text {
        **text = stats.tally.to_string();
    }
}
