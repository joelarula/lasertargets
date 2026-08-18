use bevy::prelude::*;
use common::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
use common::game::GameSession;
use common::state::{CalibrationState, GameState, ServerInstanceId, ServerState};
use gamepad::{GamepadState, ServerGamepadCursor};
use crate::plugins::actor::ActorLink;
use crate::plugins::projector::{ProjectorDacController, LaserOptimizeConfig, LaserPointBuffer};
use bevy_quinnet::server::QuinnetServer;

/// Event to trigger logging a complete server status report.
#[derive(Message, Debug, Clone, Default)]
pub struct LogStatusReportEvent;

/// Standalone plugin for server diagnostic status reporting.
pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LogStatusReportEvent>();
        app.add_systems(Startup, trigger_startup_status_report);
        app.add_systems(Update, handle_status_report_event);
    }
}

fn trigger_startup_status_report(mut events: MessageWriter<LogStatusReportEvent>) {
    events.write(LogStatusReportEvent);
}

/// System that handles `LogStatusReportEvent` and logs a formatted diagnostic report.
fn handle_status_report_event(
    mut events: MessageReader<LogStatusReportEvent>,
    gamepad_state: Option<Res<GamepadState>>,
    gamepad_cursor: Option<Res<ServerGamepadCursor>>,
    server_state: Option<Res<State<ServerState>>>,
    calibration_state: Option<Res<State<CalibrationState>>>,
    game_state: Option<Res<State<GameState>>>,
    server_instance_id: Option<Res<ServerInstanceId>>,
    game_sessions: Query<(Entity, &GameSession, Option<&Children>)>,
    projector_config: Option<Res<ProjectorConfiguration>>,
    dac_controller: Option<Res<ProjectorDacController>>,
    laser_opt: Option<Res<LaserOptimizeConfig>>,
    laser_buffer: Option<Res<LaserPointBuffer>>,
    camera_config: Option<Res<CameraConfiguration>>,
    scene_config: Option<Res<SceneConfiguration>>,
    quinnet_server: Option<Res<QuinnetServer>>,
    actor_links: Query<(Entity, &ActorLink)>,
) {
    if events.read().next().is_none() {
        return;
    }

    let on_off = |b: bool| if b { "ENABLED [ON]" } else { "DISABLED [OFF]" };
    let conn_str = |b: bool| if b { "CONNECTED" } else { "DISCONNECTED" };

    info!("╔══════════════════════════════════════════════════════════════════════════════╗");
    info!("║                       LASERTARGETS SERVER STATUS REPORT                      ║");
    info!("╠══════════════════════════════════════════════════════════════════════════════╣");
    info!("║ 1. APPLICATION STATES                                                        ║");
    info!("╟──────────────────────────────────────────────────────────────────────────────╢");
    if let Some(s) = &server_state {
        let current_server_state = s.get();
        info!("  • Server Mode:        {:?}", current_server_state);
        if *current_server_state == ServerState::InGame {
            if let Some(g) = &game_state {
                info!("  • Game Session State: {:?}", g.get());
            }
        } else {
            info!("  • Game Session State: Inactive (Server in Menu)");
        }
    }
    if let Some(c) = &calibration_state {
        info!("  • Calibration Mode:   {:?}", c.get());
    }
    if let Some(id) = &server_instance_id {
        if let Some(uuid) = id.0 {
            info!("  • Server Instance ID: {}", uuid);
        } else {
            info!("  • Server Instance ID: None");
        }
    }

    let active_sessions_count = game_sessions.iter().count();
    info!("  • Active Sessions:    {}", active_sessions_count);
    if active_sessions_count == 0 {
        info!("    (No active game sessions running)");
    } else {
        for (_entity, session, children) in game_sessions.iter() {
            let child_count = children.map_or(0, |c| c.len());
            info!("    ┌ Session ID:   {}", session.session_id);
            info!("    ├ Game Name:    {} (ID {})", session.name, session.game_id);
            info!("    └ Active Actors: {} actor(s)", child_count);
        }
    }

    info!("╠══════════════════════════════════════════════════════════════════════════════╣");
    info!("║ 2. PERIPHERALS                                                               ║");
    info!("╟──────────────────────────────────────────────────────────────────────────────╢");
    if let Some(state) = &gamepad_state {
        info!("  • Game Console Controller:");
        info!("    ├ Status:       {}", conn_str(state.connected));
        info!("    ├ Left Stick:   x = {:+.2}, y = {:+.2}", state.left_stick_x, state.left_stick_y);
        info!("    ├ Right Stick:  x = {:+.2}, y = {:+.2}", state.right_stick_x, state.right_stick_y);
        if let Some(cursor) = &gamepad_cursor {
            info!("    └ Virtual Cursor: (x = {:.2}m, y = {:.2}m)", cursor.position.x, cursor.position.y);
        }
    }

    info!("  • Laser Projector / DAC Hardware:");
    if let Some(dac) = &dac_controller {
        info!("    ├ DAC Initialized: {}", if dac.initialized { "YES" } else { "NO" });
        info!("    ├ Output Thread:   {}", if dac.thread_running { "RUNNING" } else { "STOPPED" });
    }
    if let Some(pc) = &projector_config {
        let pos = pc.origin.translation;
        info!("    ├ Laser Output:    {}", on_off(pc.switched_on));
        info!("    ├ Hardware Link:   {}", conn_str(pc.connected));
        info!("    ├ Projection Angle: {:.1}°", pc.angle);
        info!("    └ Origin Position: (x={:.2}m, y={:.2}m, z={:.2}m)", pos.x, pos.y, pos.z);
    }
    if let Some(opt) = &laser_opt {
        info!("  • Laser Optimizer Parameters:");
        info!("    └ Dwell Points:    max={}, start={}, end={}, corner={}",
            opt.0.max_dwell, opt.0.start_dwell_points, opt.0.end_dwell_points, opt.0.corner_dwell_points
        );
    }
    if let Some(buf) = &laser_buffer {
        if let Ok(points) = buf.points.lock() {
            info!("  • Laser Point Buffer:");
            info!("    └ Queue Size:      {} point(s) buffered", points.len());
        }
    }

    if let Some(cam) = &camera_config {
        let pos = cam.origin.translation;
        info!("  • Tracking Camera:");
        info!("    ├ Locked To Scene: {}", if cam.locked_to_scene { "YES" } else { "NO" });
        info!("    └ Camera Position: (x={:.2}m, y={:.2}m, z={:.2}m)", pos.x, pos.y, pos.z);
    }
    if let Some(sc) = &scene_config {
        let pos = sc.origin.translation;
        info!("  • Target Scene Area:");
        info!("    ├ Dimensions:      {:.2}m wide × {:.2}m high", sc.scene_dimension.x, sc.scene_dimension.y);
        info!("    └ Center Position: (x={:.2}m, y={:.2}m, z={:.2}m)", pos.x, pos.y, pos.z);
    }

    info!("╠══════════════════════════════════════════════════════════════════════════════╣");
    info!("║ 3. CONNECTIONS & NETWORK                                                     ║");
    info!("╟──────────────────────────────────────────────────────────────────────────────╢");
    if let Some(qs) = &quinnet_server {
        let is_listening = qs.is_listening();
        info!("  • Server Network Socket:");
        info!("    └ Status:          {}", if is_listening { "LISTENING [ACTIVE]" } else { "STOPPED [INACTIVE]" });
        if let Some(endpoint) = qs.get_endpoint() {
            let clients = endpoint.clients();
            info!("  • Connected Terminals / Clients: {} connected", clients.len());
            if clients.is_empty() {
                info!("    (No remote network clients connected)");
            } else {
                for client_id in clients {
                    info!("    ├ Client ID: {}", client_id);
                }
            }
        }
    }
    let registered_actors_count = actor_links.iter().count();
    info!("  • Registered Actor Entities: {}", registered_actors_count);
    if registered_actors_count == 0 {
        info!("    (No registered actor links)");
    } else {
        for (_e, link) in actor_links.iter() {
            info!("    ├ Client ID: {:<3} | Name: {:<15} | Roles: {:?}",
                link.client_id, link.actor.name, link.actor.roles
            );
        }
    }
    info!("╚══════════════════════════════════════════════════════════════════════════════╝");
}
