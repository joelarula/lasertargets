//! Keyboard control for the headless server.
//!
//! Keys from every source go through the shared key map ([`common::calibration::ControlKey`])
//! and become [`CalibrationAction`]s:
//! - keys forwarded by terminals (`KeyboardInputEvent`), always on
//! - the server's own console ([`ConsoleKeyboardPlugin`]), when stdin is an interactive terminal;
//!   Ctrl+C quits there

use bevy::prelude::*;
use bevy_quinnet::server::ConnectionLostEvent;
use common::calibration::{CalibrationAction, ControlKey, KEY_HELP};
use common::network::KeyboardInputEvent;
use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;

/// Terminals only send press/release, so held arrows are repeated here like OS key repeat.
const REPEAT_DELAY_SECS: f32 = 0.35;
const REPEAT_INTERVAL_SECS: f32 = 0.05;

/// Maps keys forwarded by terminals to calibration actions. Part of the core server plugins.
pub struct KeyboardControlPlugin;

impl Plugin for KeyboardControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, handle_terminal_keys);
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
    mut actions: MessageWriter<CalibrationAction>,
) {
    let TerminalKeys { shift, held } = &mut *keys;

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
            actions.write(key.action(shift.contains(&event.client_id)));
            if key.repeats() {
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

    for (&(client_id, key), timer) in held.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            actions.write(key.action(shift.contains(&client_id)));
            *timer = Timer::from_seconds(REPEAT_INTERVAL_SECS, TimerMode::Once);
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
                app.insert_resource(keyboard).add_systems(PreUpdate, read_console_keys);
            }
            Err(e) => warn!("Console keyboard unavailable: {}", e),
        }
    }
}

enum ConsoleInput {
    Key(ControlKey, bool),
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

        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("console-keyboard".into())
            .spawn(move || console_reader(sender))?;
        Ok(Self { receiver: Mutex::new(receiver) })
    }
}

impl Drop for ConsoleKeyboard {
    fn drop(&mut self) {
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
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

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
                        KeyCode::F(1) => Some(ControlKey::F1),
                        KeyCode::F(3) => Some(ControlKey::F3),
                        _ => None,
                    };
                    if control_key.is_none() {
                        info!("Console keyboard: {:?} has no action", key.code);
                    }
                    control_key.map(|k| ConsoleInput::Key(k, key.modifiers.contains(KeyModifiers::SHIFT)))
                }
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
    mut actions: MessageWriter<CalibrationAction>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(receiver) = keyboard.receiver.lock() else { return };
    for input in receiver.try_iter() {
        match input {
            ConsoleInput::Key(key, fine) => {
                actions.write(key.action(fine));
            }
            ConsoleInput::Quit => {
                info!("Ctrl+C: shutting down");
                exit.write(AppExit::Success);
            }
        }
    }
}
