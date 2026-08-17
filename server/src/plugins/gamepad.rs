use bevy::prelude::*;
use common::config::{ProjectorConfiguration, SceneConfiguration};
use common::game::{ExitGameEvent, GameSession, InitGameSessionEvent};
use common::state::{CalibrationState, GameState, ServerState};
use gamepad::{Btn, GamepadBasePlugin, GamepadState, PrevGamepadState, ServerGamepadCursor, GAMEPAD_STICK_DEADZONE, SERVER_GAMEPAD_CLIENT_ID};
use crate::plugins::actor::ActorLink;
use crate::plugins::calibration::SpawnCalibrationRippleEvent;
use crate::plugins::network::MousePositionEvent;
use crate::plugins::status::LogStatusReportEvent;

const DIMENSION_STEP: f32 = 1.0;
const DISTANCE_STEP: f32 = 1.0;
const HEIGHT_STEP: f32 = 0.25;
const HUNTER_GAME_ID: u16 = 101;
const SNAKE_GAME_ID: u16 = 2;

/// App-specific plugin registering gamepad event handlers for LaserTargets server.
pub struct GamepadInputPlugin;

#[derive(Resource)]
pub struct GamepadInputCooldowns {
    pub calibration_toggle: Timer,
    pub menu_switch: Timer,
    pub laser_toggle: Timer,
    pub calibration_step: Timer,
}

impl Default for GamepadInputCooldowns {
    fn default() -> Self {
        Self {
            calibration_toggle: Timer::from_seconds(0.5, TimerMode::Once),
            menu_switch: Timer::from_seconds(0.6, TimerMode::Once),
            laser_toggle: Timer::from_seconds(0.5, TimerMode::Once),
            calibration_step: Timer::from_seconds(0.15, TimerMode::Once),
        }
    }
}

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        // Add core hardware gamepad polling plugin from gamepad utility crate
        app.add_plugins(GamepadBasePlugin);

        app.init_resource::<GamepadInputCooldowns>();

        // Register Server Gamepad as direct Actor on startup
        app.add_systems(Startup, register_server_gamepad_actor);

        // Register application-specific gamepad event handlers & virtual cursor navigation
        app.add_systems(Update, (
            log_gamepad_buttons,
            gamepad_trigger_status_report,
            gamepad_toggle_calibration,
            gamepad_cursor_movement,
            gamepad_actor_click_handler,
            gamepad_snake_direction_handler,
        ))
        .add_systems(Update, gamepad_calibration_controls.run_if(in_state(CalibrationState::On)))
        .add_systems(Update, gamepad_laser_toggle)
        .add_systems(Update, gamepad_menu_game_switcher);
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

/// Handles click/ripple actions from the B button (Btn::East) during calibration.
fn gamepad_actor_click_handler(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    cursor: Res<ServerGamepadCursor>,
    calibration_state: Option<Res<State<CalibrationState>>>,
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
        debug!("Gamepad Left stick: ({:.2}, {:.2})", state.left_stick_x, state.left_stick_y);
    }
    if state.right_stick_x.abs() > 0.5 || state.right_stick_y.abs() > 0.5 {
        debug!("Gamepad Right stick: ({:.2}, {:.2})", state.right_stick_x, state.right_stick_y);
    }
}

// --- Game & Calibration Control Handlers ---

fn gamepad_calibration_controls(
    time: Res<Time>,
    mut cooldowns: ResMut<GamepadInputCooldowns>,
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut scene_config: ResMut<SceneConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
) {
    if !state.connected { return; }
    cooldowns.calibration_step.tick(time.delta());
    if !cooldowns.calibration_step.is_finished() { return; }

    let distance = projector_config.origin.translation.distance(scene_config.origin.translation);
    let half_angle_rad = projector_config.angle.to_radians() / 2.0;
    let max_dim = 2.0 * distance * half_angle_rad.tan();

    let mut changed = false;
    if state.just_pressed(&prev, Btn::DPadUp) {
        scene_config.scene_dimension.y = (scene_config.scene_dimension.y + DIMENSION_STEP).min(max_dim);
        info!("Gamepad: Scene height -> {:.2}m (max: {:.2}m)", scene_config.scene_dimension.y, max_dim);
        changed = true;
    }
    if state.just_pressed(&prev, Btn::DPadDown) {
        scene_config.scene_dimension.y = (scene_config.scene_dimension.y - DIMENSION_STEP).max(DIMENSION_STEP);
        info!("Gamepad: Scene height -> {:.2}m", scene_config.scene_dimension.y);
        changed = true;
    }
    if state.just_pressed(&prev, Btn::DPadRight) {
        scene_config.scene_dimension.x = (scene_config.scene_dimension.x + DIMENSION_STEP).min(max_dim);
        info!("Gamepad: Scene width -> {:.2}m (max: {:.2}m)", scene_config.scene_dimension.x, max_dim);
        changed = true;
    }
    if state.just_pressed(&prev, Btn::DPadLeft) {
        scene_config.scene_dimension.x = (scene_config.scene_dimension.x - DIMENSION_STEP).max(DIMENSION_STEP);
        info!("Gamepad: Scene width -> {:.2}m", scene_config.scene_dimension.x);
        changed = true;
    }
    if state.just_pressed(&prev, Btn::LeftBumper) {
        scene_config.origin.translation.z += DISTANCE_STEP;
        info!("Gamepad: Scene distance -> {:.2}m", scene_config.origin.translation.z.abs());
        changed = true;
    }
    if state.just_pressed(&prev, Btn::RightBumper) {
        scene_config.origin.translation.z -= DISTANCE_STEP;
        info!("Gamepad: Scene distance -> {:.2}m", scene_config.origin.translation.z.abs());
        changed = true;
    }
    if state.just_pressed(&prev, Btn::RightTrigger) {
        scene_config.origin.translation.y += HEIGHT_STEP;
        info!("Gamepad: Center height -> {:.2}m", scene_config.origin.translation.y);
        changed = true;
    }
    if state.just_pressed(&prev, Btn::LeftTrigger) {
        scene_config.origin.translation.y -= HEIGHT_STEP;
        info!("Gamepad: Center height -> {:.2}m", scene_config.origin.translation.y);
        changed = true;
    }

    if changed {
        cooldowns.calibration_step.reset();
    }
}

fn gamepad_laser_toggle(
    time: Res<Time>,
    mut cooldowns: ResMut<GamepadInputCooldowns>,
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut projector_config: ResMut<ProjectorConfiguration>,
) {
    cooldowns.laser_toggle.tick(time.delta());
    if state.just_pressed(&prev, Btn::Start) && cooldowns.laser_toggle.is_finished() {
        cooldowns.laser_toggle.reset();
        projector_config.switched_on = !projector_config.switched_on;
        info!("Gamepad: Laser {}", if projector_config.switched_on { "ON" } else { "OFF" });
    }
}

fn gamepad_toggle_calibration(
    time: Res<Time>,
    mut cooldowns: ResMut<GamepadInputCooldowns>,
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    calibration_state: Res<State<CalibrationState>>,
    mut next_calibration_state: ResMut<NextState<CalibrationState>>,
) {
    cooldowns.calibration_toggle.tick(time.delta());
    if matches!(*next_calibration_state, NextState::Pending(_)) {
        return; // Ignore button press while a state transition is queued/pending
    }

    if state.just_pressed(&prev, Btn::North) && cooldowns.calibration_toggle.is_finished() {
        cooldowns.calibration_toggle.reset();
        let next = match calibration_state.get() {
            CalibrationState::On => CalibrationState::Off,
            CalibrationState::Off => CalibrationState::On,
        };
        info!("Gamepad: Calibration mode toggled to {:?}", next);
        next_calibration_state.set(next);
    }
}

/// Handles cycling between Calibration/Menu -> Game A (Hunter) -> Game B (Snake) -> Calibration/Menu.
/// Triggers on X button (Btn::West).
fn gamepad_menu_game_switcher(
    time: Res<Time>,
    mut cooldowns: ResMut<GamepadInputCooldowns>,
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    game_sessions: Query<&GameSession>,
    next_game_state: Res<NextState<GameState>>,
    next_server_state: Res<NextState<ServerState>>,
    mut init_events: MessageWriter<InitGameSessionEvent>,
    mut exit_events: MessageWriter<ExitGameEvent>,
) {
    if !state.connected { return; }
    cooldowns.menu_switch.tick(time.delta());

    if matches!(*next_game_state, NextState::Pending(_)) || matches!(*next_server_state, NextState::Pending(_)) {
        return; // Ignore button press while a game or server state transition is queued/pending
    }

    // Button X (Btn::West): Cycle Calibration/Menu -> Hunter (Game A) -> Snake (Game B) -> Calibration/Menu
    let trigger_pressed = state.just_pressed(&prev, Btn::West) && cooldowns.menu_switch.is_finished();

    if trigger_pressed {
        cooldowns.menu_switch.reset();
        let active_session = game_sessions.iter().find(|s| s.state == GameState::InGame);
        if let Some(current_session) = active_session {
            let current_game_id = current_session.game_id;
            let current_uuid = current_session.session_id;

            // Exit current active game
            info!("★ Gamepad Menu Switcher: Exiting current game ID {} (session {})", current_game_id, current_uuid);
            exit_events.write(ExitGameEvent {
                game_session_uuid: current_uuid,
            });

            if current_game_id == HUNTER_GAME_ID {
                // Hunter (Game A) -> Snake (Game B)
                let new_uuid = bevy::asset::uuid::Uuid::new_v4();
                info!("🎮 GAME STARTED: Snake (Game ID 2) [session {}]", new_uuid);
                init_events.write(InitGameSessionEvent {
                    game_id: SNAKE_GAME_ID,
                    game_session_uuid: new_uuid,
                    initial_state: GameState::InGame,
                });
            } else {
                // Snake (Game B) -> Calibration / Main Menu
                info!("★ Gamepad Menu Switcher: Switched back to Calibration / Main Menu");
            }
        } else {
            // No active InGame session -> Launch Game A (Hunter)
            let new_uuid = bevy::asset::uuid::Uuid::new_v4();
            info!("🎮 GAME STARTED: Hunter (Game ID 101) [session {}]", new_uuid);
            init_events.write(InitGameSessionEvent {
                game_id: HUNTER_GAME_ID,
                game_session_uuid: new_uuid,
                initial_state: GameState::InGame,
            });
        }
    }
}

/// Triggers a LogStatusReportEvent when the Select button (Btn::Select) is pressed.
fn gamepad_trigger_status_report(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut status_events: MessageWriter<LogStatusReportEvent>,
) {
    if state.just_pressed(&prev, Btn::Select) {
        status_events.write(LogStatusReportEvent);
    }
}

/// Handles snake direction input from gamepad left thumbstick and DPad
fn gamepad_snake_direction_handler(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    game_sessions: Query<&GameSession>,
    mut snake_dir_events: MessageWriter<snake::model::ChangeSnakeDirectionEvent>,
) {
    if !state.connected { return; }

    let snake_active = game_sessions
        .iter()
        .any(|s| s.game_id == SNAKE_GAME_ID && s.state == GameState::InGame);

    if !snake_active { return; }

    let mut desired_dir = None;

    // Check Left Thumbstick Movement (with deadzone)
    let lx = state.left_stick_x;
    let ly = state.left_stick_y;
    let deadzone = 0.35;

    if ly > deadzone && ly.abs() >= lx.abs() {
        desired_dir = Some(snake::model::SnakeDirection::Up);
    } else if ly < -deadzone && ly.abs() >= lx.abs() {
        desired_dir = Some(snake::model::SnakeDirection::Down);
    } else if lx < -deadzone && lx.abs() >= ly.abs() {
        desired_dir = Some(snake::model::SnakeDirection::Left);
    } else if lx > deadzone && lx.abs() >= ly.abs() {
        desired_dir = Some(snake::model::SnakeDirection::Right);
    }

    // Check DPad buttons (just_pressed or pressed)
    if state.just_pressed(&prev, Btn::DPadUp) {
        desired_dir = Some(snake::model::SnakeDirection::Up);
    } else if state.just_pressed(&prev, Btn::DPadDown) {
        desired_dir = Some(snake::model::SnakeDirection::Down);
    } else if state.just_pressed(&prev, Btn::DPadLeft) {
        desired_dir = Some(snake::model::SnakeDirection::Left);
    } else if state.just_pressed(&prev, Btn::DPadRight) {
        desired_dir = Some(snake::model::SnakeDirection::Right);
    }

    if let Some(dir) = desired_dir {
        snake_dir_events.write(snake::model::ChangeSnakeDirectionEvent {
            direction: dir,
        });
    }
}

