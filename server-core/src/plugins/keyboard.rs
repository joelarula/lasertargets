//! Keyboard control for the headless server.
//!
//! Keys from every source go through the shared key map ([`common::calibration::ControlKey`]) and
//! [`ControlKey::route`]: [`CalibrationAction`]s while calibration is on (the calibration and laser
//! toggles always), [`GameKeyPressed`] for the running game otherwise. Sources:
//! - keys forwarded by terminals (`KeyboardInputEvent`), always on
//! - the server's own console ([`ConsoleKeyboardPlugin`]), when stdin is an interactive terminal;
//!   Ctrl+C quits there. The console mouse acts as an actor mouse: the console window maps onto
//!   the scene box and produces the same position and button events a terminal sends.

use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::calibration::{CalibrationAction, ControlKey, GameKeyPressed, KeyRoute, KEY_HELP};
use common::network::{KeyboardInputEvent, MouseButtonInputEvent, MousePositionEvent};
use common::scene::SceneSetup;
use common::state::CalibrationState;
use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

/// Client id of the server console's actor mouse (terminals use their connection ids)
pub const CONSOLE_CLIENT_ID: u64 = u64::MAX;

/// Terminals only send press/release, so held arrows are repeated here like OS key repeat.
const REPEAT_DELAY_SECS: f32 = 0.35;
const REPEAT_INTERVAL_SECS: f32 = 0.05;

/// Maps keys forwarded by terminals to calibration actions. Part of the core server plugins.
pub struct KeyboardControlPlugin;

impl Plugin for KeyboardControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameKeyPressed>().add_systems(PreUpdate, handle_terminal_keys);
    }
}

#[derive(Default)]
struct TerminalKeys {
    shift: HashSet<u64>,
    held: HashMap<(u64, ControlKey), Timer>,
}

fn handle_terminal_keys(
    time: Res<Time>,
    mut key_events: MessageReader<KeyboardInputEvent>,
    mut connection_lost: MessageReader<ConnectionLostEvent>,
    mut keys: Local<TerminalKeys>,
    calibration: Option<Res<State<CalibrationState>>>,
    mut actions: MessageWriter<CalibrationAction>,
    mut game_keys: MessageWriter<GameKeyPressed>,
) {
    let TerminalKeys { shift, held } = &mut *keys;
    let calibration_on = is_calibrating(calibration.as_deref());

    for event in key_events.read() {
        if event.key.starts_with("Shift") {
            if event.pressed {
                shift.insert(event.client_id);
            } else {
                shift.remove(&event.client_id);
            }
            continue;
        }
        let Some(key) = ControlKey::from_key_name(&event.key) else { continue };
        if event.pressed {
            let route = key.route(shift.contains(&event.client_id), calibration_on);
            send_key(route, event.client_id, &mut actions, &mut game_keys);
            // Only calibration nudges repeat; a game sees one press per press
            if matches!(route, KeyRoute::Calibration(_)) && key.repeats() {
                held.insert((event.client_id, key), Timer::from_seconds(REPEAT_DELAY_SECS, TimerMode::Once));
            }
        } else {
            held.remove(&(event.client_id, key));
        }
    }

    for lost in connection_lost.read() {
        shift.remove(&lost.id);
        held.retain(|(client_id, _), _| *client_id != lost.id);
    }

    // Calibration switched off while an arrow is held: stop repeating it
    if !calibration_on {
        held.clear();
    }
    for (&(client_id, key), timer) in held.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            actions.write(key.action(shift.contains(&client_id)));
            *timer = Timer::from_seconds(REPEAT_INTERVAL_SECS, TimerMode::Once);
        }
    }
}

/// Calibration counts as on when the state is missing (apps without calibration never route to games)
fn is_calibrating(state: Option<&State<CalibrationState>>) -> bool {
    state.is_none_or(|s| *s.get() == CalibrationState::On)
}

fn send_key(
    route: KeyRoute,
    client_id: u64,
    actions: &mut MessageWriter<CalibrationAction>,
    game_keys: &mut MessageWriter<GameKeyPressed>,
) {
    match route {
        KeyRoute::Calibration(action) => {
            actions.write(action);
        }
        KeyRoute::Game(key) => {
            game_keys.write(GameKeyPressed { client_id, key });
        }
    }
}

/// Reads keys from the server's own console when stdin is an interactive terminal.
///
/// The console is put in raw mode so single key presses arrive without Enter; Ctrl+C is then
/// handled here and exits the app cleanly. Add this only in the server binary, never in tests.
pub struct ConsoleKeyboardPlugin;

impl Plugin for ConsoleKeyboardPlugin {
    fn build(&self, app: &mut App) {
        if !std::io::stdin().is_terminal() {
            info!("Console keyboard: stdin is not a terminal, disabled");
            return;
        }
        match ConsoleKeyboard::start() {
            Ok(keyboard) => {
                info!("Console keyboard detected. {}  [Ctrl+C] quit", KEY_HELP);
                app.insert_resource(keyboard)
                    .add_message::<GameKeyPressed>()
                    .add_systems(PreUpdate, read_console_keys);
            }
            Err(e) => warn!("Console keyboard unavailable: {}", e),
        }
    }
}

enum ConsoleInput {
    Key(ControlKey, bool),
    /// `at` is the position in the console window: (0, 0) top-left, (1, 1) bottom-right
    Mouse { at: Vec2, button: Option<(&'static str, bool)> },
    Quit,
}

#[derive(Resource)]
struct ConsoleKeyboard {
    receiver: Mutex<Receiver<ConsoleInput>>,
}

impl ConsoleKeyboard {
    fn start() -> std::io::Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        #[cfg(target_os = "linux")]
        keep_output_newlines();
        if let Err(e) = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture) {
            warn!("Console mouse unavailable: {}", e);
        }

        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("console-keyboard".into())
            .spawn(move || console_reader(sender))?;
        Ok(Self { receiver: Mutex::new(receiver) })
    }
}

impl Drop for ConsoleKeyboard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Raw mode also turns off output post-processing on Linux, which would print log lines as a
/// staircase; turn "\n" -> "\r\n" back on.
#[cfg(target_os = "linux")]
fn keep_output_newlines() {
    // SAFETY: plain termios calls on stdin, which was checked to be a terminal.
    unsafe {
        let mut termios: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(libc::STDIN_FILENO, &mut termios) == 0 {
            termios.c_oflag |= libc::OPOST | libc::ONLCR;
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);
        }
    }
}

fn console_reader(sender: Sender<ConsoleInput>) {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

    loop {
        let input = match event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    Some(ConsoleInput::Quit)
                } else {
                    let control_key = match key.code {
                        KeyCode::Up => Some(ControlKey::Up),
                        KeyCode::Down => Some(ControlKey::Down),
                        KeyCode::Left => Some(ControlKey::Left),
                        KeyCode::Right => Some(ControlKey::Right),
                        KeyCode::Char(' ') => Some(ControlKey::Space),
                        KeyCode::Char('l' | 'L') | KeyCode::F(1) => Some(ControlKey::Laser),
                        KeyCode::Char('c' | 'C') | KeyCode::F(3) => Some(ControlKey::Calibration),
                        KeyCode::Char('s' | 'S') => Some(ControlKey::Shape),
                        _ => None,
                    };
                    if control_key.is_none() {
                        info!("Console keyboard: {:?} has no action", key.code);
                    }
                    control_key.map(|k| ConsoleInput::Key(k, key.modifiers.contains(KeyModifiers::SHIFT)))
                }
            }
            Ok(Event::Mouse(mouse)) => {
                let button_name = |button: MouseButton| match button {
                    MouseButton::Left => "Left",
                    MouseButton::Right => "Right",
                    MouseButton::Middle => "Middle",
                };
                let button = match mouse.kind {
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => Some(None),
                    MouseEventKind::Down(b) => Some(Some((button_name(b), true))),
                    MouseEventKind::Up(b) => Some(Some((button_name(b), false))),
                    _ => None,
                };
                button.map(|button| {
                    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                    let at = Vec2::new(
                        mouse.column as f32 / cols.saturating_sub(1).max(1) as f32,
                        mouse.row as f32 / rows.saturating_sub(1).max(1) as f32,
                    );
                    ConsoleInput::Mouse { at, button }
                })
            }
            Ok(_) => None,
            Err(e) => {
                warn!("Console keyboard stopped: {}", e);
                return;
            }
        };
        if let Some(input) = input {
            if sender.send(input).is_err() {
                return;
            }
        }
    }
}

fn read_console_keys(
    keyboard: Res<ConsoleKeyboard>,
    scene_setup: Res<SceneSetup>,
    calibration: Option<Res<State<CalibrationState>>>,
    mut actions: MessageWriter<CalibrationAction>,
    mut game_keys: MessageWriter<GameKeyPressed>,
    mut mouse_positions: MessageWriter<MousePositionEvent>,
    mut mouse_buttons: MessageWriter<MouseButtonInputEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(receiver) = keyboard.receiver.lock() else { return };
    for input in receiver.try_iter() {
        match input {
            ConsoleInput::Key(key, fine) => {
                let route = key.route(fine, is_calibrating(calibration.as_deref()));
                send_key(route, CONSOLE_CLIENT_ID, &mut actions, &mut game_keys);
            }
            ConsoleInput::Mouse { at, button } => {
                // The console window is the billboard
                let position = Some(scene_setup.billboard_point(at.clamp(Vec2::ZERO, Vec2::ONE)));
                mouse_positions.write(MousePositionEvent { client_id: CONSOLE_CLIENT_ID, position });
                if let Some((button, pressed)) = button {
                    mouse_buttons.write(MouseButtonInputEvent {
                        client_id: CONSOLE_CLIENT_ID,
                        button: button.to_string(),
                        pressed,
                        position,
                    });
                }
            }
            ConsoleInput::Quit => {
                info!("Ctrl+C: shutting down");
                exit.write(AppExit::Success);
            }
        }
    }
}
