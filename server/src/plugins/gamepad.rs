use bevy::prelude::*;
use common::config::{ProjectorConfiguration, SceneConfiguration};
use common::game::{ExitGameEvent, GameSession, InitGameSessionEvent};
use common::state::{CalibrationState, GameState, ServerState};

const DIMENSION_STEP: f32 = 1.0;
const DISTANCE_STEP: f32 = 1.0;
const HEIGHT_STEP: f32 = 0.25;
const HUNTER_GAME_ID: u16 = 101;
const SNAKE_GAME_ID: u16 = 2;

// --- Cross-platform gamepad state ---

/// Represents a snapshot of gamepad button/axis state each frame.
#[derive(Resource, Default, Clone, Debug)]
pub struct GamepadState {
    pub connected: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub left_bumper: bool,
    pub right_bumper: bool,
    pub left_trigger: bool,
    pub right_trigger: bool,
    pub start: bool,
    pub select: bool,
    pub left_thumb: bool,
    pub right_thumb: bool,
    pub left_stick_x: f32,
    pub left_stick_y: f32,
    pub right_stick_x: f32,
    pub right_stick_y: f32,
}

/// Tracks previous frame for just_pressed detection.
#[derive(Resource, Default, Clone, Debug)]
struct PrevGamepadState {
    dpad_up: bool,
    dpad_down: bool,
    dpad_left: bool,
    dpad_right: bool,
    south: bool,
    east: bool,
    west: bool,
    north: bool,
    left_bumper: bool,
    right_bumper: bool,
    left_trigger: bool,
    right_trigger: bool,
    start: bool,
    select: bool,
    left_thumb: bool,
    right_thumb: bool,
}

#[derive(Clone, Copy, Debug)]
enum Btn {
    DPadUp, DPadDown, DPadLeft, DPadRight,
    South, East, West, North,
    LeftBumper, RightBumper,
    LeftTrigger, RightTrigger,
    Start, Select,
    LeftThumb, RightThumb,
}

impl GamepadState {
    fn just_pressed(&self, prev: &PrevGamepadState, button: Btn) -> bool {
        let (cur, old) = match button {
            Btn::DPadUp => (self.dpad_up, prev.dpad_up),
            Btn::DPadDown => (self.dpad_down, prev.dpad_down),
            Btn::DPadLeft => (self.dpad_left, prev.dpad_left),
            Btn::DPadRight => (self.dpad_right, prev.dpad_right),
            Btn::South => (self.south, prev.south),
            Btn::East => (self.east, prev.east),
            Btn::West => (self.west, prev.west),
            Btn::North => (self.north, prev.north),
            Btn::LeftBumper => (self.left_bumper, prev.left_bumper),
            Btn::RightBumper => (self.right_bumper, prev.right_bumper),
            Btn::LeftTrigger => (self.left_trigger, prev.left_trigger),
            Btn::RightTrigger => (self.right_trigger, prev.right_trigger),
            Btn::Start => (self.start, prev.start),
            Btn::Select => (self.select, prev.select),
            Btn::LeftThumb => (self.left_thumb, prev.left_thumb),
            Btn::RightThumb => (self.right_thumb, prev.right_thumb),
        };
        cur && !old
    }
}

// --- Windows XInput backend ---

#[cfg(target_os = "windows")]
mod xinput_backend {
    use super::GamepadState;
    use bevy::prelude::*;
    use rusty_xinput::XInputHandle;

    const TRIGGER_THRESHOLD: u8 = 30;
    const STICK_DEADZONE: i16 = 7849;

    #[derive(Resource)]
    pub struct XInputBackend {
        handle: XInputHandle,
        controller_id: u32,
    }

    impl XInputBackend {
        pub fn new() -> Option<Self> {
            let handle = XInputHandle::load_default().ok()?;
            for id in 0..4 {
                if handle.get_state(id).is_ok() {
                    info!("XInput: Found controller at slot {}", id);
                    return Some(Self { handle, controller_id: id });
                }
            }
            info!("XInput: No controllers found at startup (will poll slot 0)");
            Some(Self { handle, controller_id: 0 })
        }

        pub fn poll(&self) -> GamepadState {
            match self.handle.get_state(self.controller_id) {
                Ok(state) => {
                    let gp = state.raw.Gamepad;
                    let buttons = gp.wButtons;

                    fn stick(val: i16, deadzone: i16) -> f32 {
                        if (val as i32).abs() < deadzone as i32 { 0.0 } else { val as f32 / 32768.0 }
                    }

                    GamepadState {
                        connected: true,
                        dpad_up: buttons & 0x0001 != 0,
                        dpad_down: buttons & 0x0002 != 0,
                        dpad_left: buttons & 0x0004 != 0,
                        dpad_right: buttons & 0x0008 != 0,
                        start: buttons & 0x0010 != 0,
                        select: buttons & 0x0020 != 0,
                        left_thumb: buttons & 0x0040 != 0,
                        right_thumb: buttons & 0x0080 != 0,
                        left_bumper: buttons & 0x0100 != 0,
                        right_bumper: buttons & 0x0200 != 0,
                        south: buttons & 0x1000 != 0,
                        east: buttons & 0x2000 != 0,
                        west: buttons & 0x4000 != 0,
                        north: buttons & 0x8000 != 0,
                        left_trigger: gp.bLeftTrigger > TRIGGER_THRESHOLD,
                        right_trigger: gp.bRightTrigger > TRIGGER_THRESHOLD,
                        left_stick_x: stick(gp.sThumbLX, STICK_DEADZONE),
                        left_stick_y: stick(gp.sThumbLY, STICK_DEADZONE),
                        right_stick_x: stick(gp.sThumbRX, STICK_DEADZONE),
                        right_stick_y: stick(gp.sThumbRY, STICK_DEADZONE),
                    }
                }
                Err(_) => GamepadState { connected: false, ..Default::default() },
            }
        }
    }
}

// --- Plugin ---

pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadState>()
            .init_resource::<PrevGamepadState>();

        #[cfg(target_os = "windows")]
        {
            if let Some(backend) = xinput_backend::XInputBackend::new() {
                app.insert_resource(backend);
                app.add_systems(PreUpdate, poll_xinput);
            } else {
                warn!("Failed to load XInput — gamepad will not work on Windows");
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Linux, Bevy's GilrsPlugin (evdev) provides Gamepad components.
            app.add_systems(PreUpdate, poll_bevy_gamepad);
        }

        app.add_systems(Update, log_gamepad_buttons)
            .add_systems(Update, gamepad_calibration_controls.run_if(in_state(CalibrationState::On)))
            .add_systems(Update, gamepad_laser_toggle)
            .add_systems(Update, gamepad_start_game.run_if(in_state(ServerState::Menu)))
            .add_systems(Update, gamepad_exit_game.run_if(in_state(ServerState::InGame)));
    }
}

// --- Polling systems ---

fn save_prev(state: &GamepadState, prev: &mut PrevGamepadState) {
    prev.dpad_up = state.dpad_up;
    prev.dpad_down = state.dpad_down;
    prev.dpad_left = state.dpad_left;
    prev.dpad_right = state.dpad_right;
    prev.south = state.south;
    prev.east = state.east;
    prev.west = state.west;
    prev.north = state.north;
    prev.left_bumper = state.left_bumper;
    prev.right_bumper = state.right_bumper;
    prev.left_trigger = state.left_trigger;
    prev.right_trigger = state.right_trigger;
    prev.start = state.start;
    prev.select = state.select;
    prev.left_thumb = state.left_thumb;
    prev.right_thumb = state.right_thumb;
}

#[cfg(target_os = "windows")]
fn poll_xinput(
    backend: Res<xinput_backend::XInputBackend>,
    mut state: ResMut<GamepadState>,
    mut prev: ResMut<PrevGamepadState>,
) {
    save_prev(&state, &mut prev);
    *state = backend.poll();
}

#[cfg(not(target_os = "windows"))]
fn poll_bevy_gamepad(
    gamepads: Query<&bevy::input::gamepad::Gamepad>,
    mut state: ResMut<GamepadState>,
    mut prev: ResMut<PrevGamepadState>,
) {
    use bevy::input::gamepad::{GamepadButton, GamepadAxis};

    save_prev(&state, &mut prev);

    if let Some(gamepad) = gamepads.iter().next() {
        *state = GamepadState {
            connected: true,
            dpad_up: gamepad.pressed(GamepadButton::DPadUp),
            dpad_down: gamepad.pressed(GamepadButton::DPadDown),
            dpad_left: gamepad.pressed(GamepadButton::DPadLeft),
            dpad_right: gamepad.pressed(GamepadButton::DPadRight),
            south: gamepad.pressed(GamepadButton::South),
            east: gamepad.pressed(GamepadButton::East),
            west: gamepad.pressed(GamepadButton::West),
            north: gamepad.pressed(GamepadButton::North),
            left_bumper: gamepad.pressed(GamepadButton::LeftTrigger),
            right_bumper: gamepad.pressed(GamepadButton::RightTrigger),
            left_trigger: gamepad.pressed(GamepadButton::LeftTrigger2),
            right_trigger: gamepad.pressed(GamepadButton::RightTrigger2),
            start: gamepad.pressed(GamepadButton::Start),
            select: gamepad.pressed(GamepadButton::Select),
            left_thumb: gamepad.pressed(GamepadButton::LeftThumb),
            right_thumb: gamepad.pressed(GamepadButton::RightThumb),
            left_stick_x: gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0),
            left_stick_y: gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0),
            right_stick_x: gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0),
            right_stick_y: gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0),
        };
    } else {
        *state = GamepadState::default();
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

// --- Game control systems ---

fn gamepad_calibration_controls(
    state: Res<GamepadState>,
    prev: Res<PrevGamepadState>,
    mut scene_config: ResMut<SceneConfiguration>,
    projector_config: Res<ProjectorConfiguration>,
) {
    if !state.connected { return; }

    // Compute max projectable dimension from projector FOV and distance
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
