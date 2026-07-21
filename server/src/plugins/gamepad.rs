use bevy::prelude::*;
use common::config::{ProjectorConfiguration, SceneConfiguration};
use common::game::{ExitGameEvent, GameSession, InitGameSessionEvent};
use common::state::{CalibrationState, GameState, ServerState};
use gamepad::{Btn, GamepadBasePlugin, GamepadState, PrevGamepadState, ServerGamepadCursor, GAMEPAD_STICK_DEADZONE, SERVER_GAMEPAD_CLIENT_ID};
use crate::plugins::actor::ActorLink;
use crate::plugins::calibration::SpawnCalibrationRippleEvent;
use crate::plugins::network::MousePositionEvent;
use crate::plugins::status::LogStatusReportEvent;
use hunter::model::HunterClickEvent;

const DIMENSION_STEP: f32 = 1.0;
const DISTANCE_STEP: f32 = 1.0;
const HEIGHT_STEP: f32 = 0.25;
const HUNTER_GAME_ID: u16 = 101;
const SNAKE_GAME_ID: u16 = 2;

/// App-specific plugin registering gamepad event handlers for LaserTargets server.
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        // Add core hardware gamepad polling plugin from gamepad utility crate
        app.add_plugins(GamepadBasePlugin);

        // Register Server Gamepad as direct Actor on startup
        app.add_systems(Startup, register_server_gamepad_actor);

        // Register application-specific gamepad event handlers & virtual cursor navigation
        app.add_systems(Update, (
            log_gamepad_buttons,
            gamepad_trigger_status_report,
            gamepad_toggle_calibration,
            gamepad_cursor_movement,
            gamepad_actor_click_handler,
        ))
        .add_systems(Update, gamepad_calibration_controls.run_if(in_state(CalibrationState::On)))
        .add_systems(Update, gamepad_laser_toggle)
        .add_systems(Update, gamepad_start_game.run_if(in_state(ServerState::Menu)))
        .add_systems(Update, gamepad_exit_game.run_if(in_state(ServerState::InGame)));
    }
}

/// Spawns an ActorLink entity for the server gamepad so it is recognized as a direct Actor.
fn register_server_gamepad_actor(mut commands: Commands) {
    let actor = common::actor::Actor {
        name: "Server-Gamepad".to_string(),
        uuid: bevy::asset::uuid::Uuid::from_u128(0x53455256_4741_4d45_5041_443030303030),
        roles: vec!["Server".to_string(), "Controller".to_string(), "Player".to_string()],
    };
    let actor_link = ActorLink {
        client_id: SERVER_GAMEPAD_CLIENT_ID,
        actor: actor.clone(),
    };
    commands.spawn((actor, actor_link));
    info!("✓ Registered Server Gamepad as direct Actor (Client ID {})", SERVER_GAMEPAD_CLIENT_ID);
}

/// Updates the virtual mouse/cursor position based on left thumbstick input and emits MousePositionEvent.
fn gamepad_cursor_movement(
    state: Res<GamepadState>,
    time: Res<Time>,
    scene_config: Res<SceneConfiguration>,
    mut cursor: ResMut<ServerGamepadCursor>,
    mut mouse_events: MessageWriter<MousePositionEvent>,
) {
    if !state.connected { return; }

    let dt = time.delta_secs();
    let move_speed = cursor.sensitivity;

    // Initialize cursor position to scene origin center if uninitialized
    if cursor.position == Vec3::ZERO {
        cursor.position = scene_config.origin.translation;
    }

    if state.left_stick_x.abs() > GAMEPAD_STICK_DEADZONE || state.left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
        cursor.position.x += state.left_stick_x * move_speed * dt;
        cursor.position.y += state.left_stick_y * move_speed * dt;
    }

    let half_w = (scene_config.scene_dimension.x as f32 / 2.0).max(0.5);
    let half_h = (scene_config.scene_dimension.y as f32 / 2.0).max(0.5);

    let min_x = scene_config.origin.translation.x - half_w;
    let max_x = scene_config.origin.translation.x + half_w;
    let min_y = scene_config.origin.translation.y - half_h;
    let max_y = scene_config.origin.translation.y + half_h;

    cursor.position.x = cursor.position.x.clamp(min_x, max_x);
    cursor.position.y = cursor.position.y.clamp(min_y, max_y);
    cursor.position.z = scene_config.origin.translation.z;

    // Always emit current cursor position for calibration crosshairs and game aim
    mouse_events.write(MousePositionEvent {
        client_id: SERVER_GAMEPAD_CLIENT_ID,
        position: Some(cursor.position),
    });
}

/// Handles click/fire actions from the B button (Btn::East).
fn gamepad_actor_click_handler(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    cursor: Res<ServerGamepadCursor>,
    calibration_state: Option<Res<State<CalibrationState>>>,
    game_sessions: Query<&GameSession>,
    mut click_events: MessageWriter<HunterClickEvent>,
    mut ripple_events: MessageWriter<SpawnCalibrationRippleEvent>,
) {
    if state.just_pressed(&prev, Btn::East) {
        info!("Gamepad B Button CLICK at position {:?}", cursor.position);

        if let Some(cal) = &calibration_state {
            if *cal.get() == CalibrationState::On {
                ripple_events.write(SpawnCalibrationRippleEvent {
                    position: cursor.position,
                });
            }
        }

        for session in game_sessions.iter() {
            if session.game_id == HUNTER_GAME_ID && session.state == GameState::InGame {
                click_events.write(HunterClickEvent {
                    session_id: session.session_id,
                    click_position: cursor.position,
                });
            }
        }
    }
}

// --- Debug logging ---

fn log_gamepad_buttons(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    calibration_state: Res<State<CalibrationState>>,
    server_state: Res<State<ServerState>>,
) {
    let buttons = [
        ("South/A", Btn::South),
        ("East/B", Btn::East),
        ("North/Y", Btn::North),
        ("West/X", Btn::West),
        ("DPadUp", Btn::DPadUp),
        ("DPadDown", Btn::DPadDown),
        ("DPadLeft", Btn::DPadLeft),
        ("DPadRight", Btn::DPadRight),
        ("LB", Btn::LeftBumper),
        ("RB", Btn::RightBumper),
        ("LT", Btn::LeftTrigger),
        ("RT", Btn::RightTrigger),
        ("Start", Btn::Start),
        ("Select", Btn::Select),
        ("LeftThumb", Btn::LeftThumb),
        ("RightThumb", Btn::RightThumb),
    ];

    for (name, button) in &buttons {
        if state.just_pressed(&prev, *button) {
            info!(
                "Gamepad button PRESSED: {} | CalibrationState: {:?}, ServerState: {:?}",
                name, *calibration_state.get(), *server_state.get()
            );
        }
    }

    if state.left_stick_x.abs() > 0.5 || state.left_stick_y.abs() > 0.5 {
        info!("Gamepad Left stick: ({:.2}, {:.2})", state.left_stick_x, state.left_stick_y);
    }
    if state.right_stick_x.abs() > 0.5 || state.right_stick_y.abs() > 0.5 {
        info!("Gamepad Right stick: ({:.2}, {:.2})", state.right_stick_x, state.right_stick_y);
    }
}

// --- Game & Calibration Control Handlers ---

fn gamepad_calibration_controls(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut scene_config: ResMut<SceneConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
) {
    if !state.connected { return; }

    let distance = projector_config.origin.translation.distance(scene_config.origin.translation);
    let half_angle_rad = projector_config.angle.to_radians() / 2.0;
    let max_dim = 2.0 * distance * half_angle_rad.tan();

    if state.just_pressed(&prev, Btn::DPadUp) {
        scene_config.scene_dimension.y = (scene_config.scene_dimension.y + DIMENSION_STEP).min(max_dim);
        info!("Gamepad: Scene height -> {:.2}m (max: {:.2}m)", scene_config.scene_dimension.y, max_dim);
    }
    if state.just_pressed(&prev, Btn::DPadDown) {
        scene_config.scene_dimension.y = (scene_config.scene_dimension.y - DIMENSION_STEP).max(DIMENSION_STEP);
        info!("Gamepad: Scene height -> {:.2}m", scene_config.scene_dimension.y);
    }
    if state.just_pressed(&prev, Btn::DPadRight) {
        scene_config.scene_dimension.x = (scene_config.scene_dimension.x + DIMENSION_STEP).min(max_dim);
        info!("Gamepad: Scene width -> {:.2}m (max: {:.2}m)", scene_config.scene_dimension.x, max_dim);
    }
    if state.just_pressed(&prev, Btn::DPadLeft) {
        scene_config.scene_dimension.x = (scene_config.scene_dimension.x - DIMENSION_STEP).max(DIMENSION_STEP);
        info!("Gamepad: Scene width -> {:.2}m", scene_config.scene_dimension.x);
    }
    if state.just_pressed(&prev, Btn::LeftBumper) {
        scene_config.origin.translation.z += DISTANCE_STEP;
        info!("Gamepad: Scene distance -> {:.2}m", scene_config.origin.translation.z.abs());
    }
    if state.just_pressed(&prev, Btn::RightBumper) {
        scene_config.origin.translation.z -= DISTANCE_STEP;
        info!("Gamepad: Scene distance -> {:.2}m", scene_config.origin.translation.z.abs());
    }
    if state.just_pressed(&prev, Btn::RightTrigger) {
        scene_config.origin.translation.y += HEIGHT_STEP;
        info!("Gamepad: Center height -> {:.2}m", scene_config.origin.translation.y);
    }
    if state.just_pressed(&prev, Btn::LeftTrigger) {
        scene_config.origin.translation.y -= HEIGHT_STEP;
        info!("Gamepad: Center height -> {:.2}m", scene_config.origin.translation.y);
    }
}

fn gamepad_laser_toggle(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut projector_config: ResMut<ProjectorConfiguration>,
) {
    if state.just_pressed(&prev, Btn::West) {
        projector_config.switched_on = !projector_config.switched_on;
        info!("Gamepad: Laser {}", if projector_config.switched_on { "ON" } else { "OFF" });
    }
}

fn gamepad_toggle_calibration(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
) {
    if state.just_pressed(&prev, Btn::North) {
        let next = match calibration_state.get() {
            CalibrationState::On => CalibrationState::Off,
            CalibrationState::Off => CalibrationState::On,
        };
        info!("Gamepad: Calibration mode toggled to {:?}", next);
        next_calibration_state.set(next);
    }
}

fn gamepad_start_game(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut init_events: MessageWriter<InitGameSessionEvent>,
) {
    if state.just_pressed(&prev, Btn::Start) {
        let uuid = bevy::asset::uuid::Uuid::new_v4();
        info!("Gamepad: Starting Hunter game (session {})", uuid);
        init_events.write(InitGameSessionEvent {
            game_id: HUNTER_GAME_ID,
            game_session_uuid: uuid,
            initial_state: GameState::InGame,
        });
    }
    if state.just_pressed(&prev, Btn::Select) {
        let uuid = bevy::asset::uuid::Uuid::new_v4();
        info!("Gamepad: Starting Snake game (session {})", uuid);
        init_events.write(InitGameSessionEvent {
            game_id: SNAKE_GAME_ID,
            game_session_uuid: uuid,
            initial_state: GameState::InGame,
        });
    }
}

fn gamepad_exit_game(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    game_sessions: Query<&GameSession>,
    mut exit_events: MessageWriter<ExitGameEvent>,
) {
    if state.just_pressed(&prev, Btn::Select) {
        if let Some(session) = game_sessions.iter().next() {
            info!("Gamepad: Exiting game session {}", session.session_id);
            exit_events.write(ExitGameEvent {
                game_session_uuid: session.session_id,
            });
        }
    }
}

/// Triggers a LogStatusReportEvent when the A button (Btn::South) is pressed.
fn gamepad_trigger_status_report(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut status_events: MessageWriter<LogStatusReportEvent>,
) {
    if state.just_pressed(&prev, Btn::South) {
        status_events.write(LogStatusReportEvent);
    }
}
