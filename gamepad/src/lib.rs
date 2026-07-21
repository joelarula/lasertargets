use bevy::prelude::*;

/// Snapshot of gamepad button and axis state each frame.
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

/// Tracks previous frame state for just_pressed edge detection.
#[derive(Resource, Default, Clone, Debug)]
pub struct PrevGamepadState {
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Btn {
    DPadUp, DPadDown, DPadLeft, DPadRight,
    South, East, West, North,
    LeftBumper, RightBumper,
    LeftTrigger, RightTrigger,
    Start, Select,
    LeftThumb, RightThumb,
}

impl GamepadState {
    pub fn just_pressed(&self, prev: &PrevGamepadState, button: Btn) -> bool {
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

pub fn save_prev(state: &GamepadState, prev: &mut PrevGamepadState) {
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

pub const SERVER_GAMEPAD_CLIENT_ID: u64 = 0;
pub const DEFAULT_GAMEPAD_CURSOR_SENSITIVITY: f32 = 12.0;
pub const GAMEPAD_STICK_DEADZONE: f32 = 0.05;

/// Resource storing virtual mouse/cursor position driven by the gamepad.
#[derive(Resource, Debug, Clone)]
pub struct ServerGamepadCursor {
    pub position: Vec3,
    pub sensitivity: f32,
}

impl Default for ServerGamepadCursor {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            sensitivity: DEFAULT_GAMEPAD_CURSOR_SENSITIVITY,
        }
    }
}

/// Standalone Bevy plugin providing gamepad state polling resources.
pub struct GamepadBasePlugin;

impl Plugin for GamepadBasePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadState>()
            .init_resource::<PrevGamepadState>()
            .init_resource::<ServerGamepadCursor>();

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
            app.add_systems(PreUpdate, poll_bevy_gamepad);
        }
    }
}
