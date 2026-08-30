use bevy::prelude::*;
use common::game::{BroadcastGameDataPayload, GameSession, GameSessionCreated, ExitGameEvent};
use common::network::{FromClientMessage, NetworkMessage};
use common::scene::SceneSetup;
use common::state::GameState;

use crate::common::{GAME_ID, generate_game_report};
use crate::components::{CollisionIndicator, HunterTargetEntity, HunterTitleAnnouncement};
use crate::events::{BroadcastStatsUpdateEvent, SpawnHunterTargetEvent};
use crate::resources::HunterGameStats;
use crate::types::GameReport;

pub fn hunter_session_is_running(game_sessions: Query<&GameSession>) -> bool {
    game_sessions
        .iter()
        .any(|session| session.game_id == GAME_ID && session.state == GameState::InGame)
}

pub fn reset_hunter_session(
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

pub fn reset_hunter_on_new_session(
    mut commands: Commands,
    mut created_events: MessageReader<GameSessionCreated>,
    mut exit_events: MessageReader<ExitGameEvent>,
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

pub fn save_hunter_report(
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

    writeln!(s).unwrap();
    writeln!(s, "## Statistics").unwrap();
    writeln!(s, "- **Game Duration**: {:.2}s", report.total_game_time).unwrap();
    writeln!(s, "- **Targets Spawned**: {}", report.total_targets_spawned).unwrap();
    writeln!(s, "- **Targets Popped**: {}", report.total_targets_popped).unwrap();
    writeln!(s, "- **Misses**: {}", report.total_misses).unwrap();
    writeln!(s, "- **Score**: {}", report.total_score).unwrap();
    writeln!(s, "- **Avg Spawn Interval**: {:.2}s", report.avg_spawn_interval).unwrap();
    writeln!(s, "- **Avg Target Lifetime**: {:.2}s", report.avg_target_lifetime).unwrap();

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

pub fn forward_hunter_stats_to_network(
    mut events: MessageReader<BroadcastStatsUpdateEvent>,
    mut payload_writer: MessageWriter<BroadcastGameDataPayload>,
) {
    for event in events.read() {
        if let Ok(json) = serde_json::to_string(event) {
            payload_writer.write(BroadcastGameDataPayload {
                game_id: GAME_ID,
                session_id: event.session_id,
                event_tag: "hunter_stats".to_string(),
                payload_json: json,
            });
        }
    }
}

pub fn handle_incoming_hunter_payloads(
    mut client_messages: MessageReader<FromClientMessage>,
    mut spawn_events: MessageWriter<SpawnHunterTargetEvent>,
) {
    for msg in client_messages.read() {
        if let NetworkMessage::GameDataPayload { game_id, ref event_tag, ref payload_json, .. } = msg.message {
            if game_id == GAME_ID && event_tag == "spawn_target" {
                if let Ok(event) = serde_json::from_str::<SpawnHunterTargetEvent>(payload_json) {
                    spawn_events.write(event);
                }
            }
        }
    }
}
