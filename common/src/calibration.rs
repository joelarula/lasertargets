//! Calibration controls shared by the server and the terminal: one key map, one set of actions
//! and one adjustment function. The server applies them authoritatively; a terminal without a
//! server connection applies them locally.
//!
//! | Key          | Action                               |
//! |--------------|--------------------------------------|
//! | C or F3      | cycle calibration overlays ([`CalibrationOverlays`]) |
//! | L or F1      | laser on/off                         |
//! | S            | next centre shape                    |
//! | Space        | next calibration mode                |
//! | Arrows       | adjust (hold Shift for fine steps)   |

use crate::scene::SceneSetup;
use bevy::prelude::*;

pub const KEY_HELP: &str =
    "Keys: [C]/[F3] calibration overlays  [L]/[F1] laser on/off  [S] next shape  [Space] next mode  [Arrows] adjust ([Shift] fine)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKey {
    Up,
    Down,
    Left,
    Right,
    Space,
    /// L or F1
    Laser,
    /// C or F3
    Calibration,
    /// S
    Shape,
}

impl ControlKey {
    /// Parses a Bevy `KeyCode` debug name, as forwarded by terminals.
    pub fn from_key_name(name: &str) -> Option<Self> {
        match name {
            "ArrowUp" => Some(Self::Up),
            "ArrowDown" => Some(Self::Down),
            "ArrowLeft" => Some(Self::Left),
            "ArrowRight" => Some(Self::Right),
            "Space" => Some(Self::Space),
            "KeyL" | "F1" => Some(Self::Laser),
            "KeyC" | "F3" => Some(Self::Calibration),
            "KeyS" => Some(Self::Shape),
            _ => None,
        }
    }

    pub fn action(self, fine: bool) -> CalibrationAction {
        match self {
            Self::Up => CalibrationAction::Nudge { dir: IVec2::Y, fine },
            Self::Down => CalibrationAction::Nudge { dir: IVec2::NEG_Y, fine },
            Self::Left => CalibrationAction::Nudge { dir: IVec2::NEG_X, fine },
            Self::Right => CalibrationAction::Nudge { dir: IVec2::X, fine },
            Self::Space => CalibrationAction::NextMode,
            Self::Laser => CalibrationAction::ToggleLaser,
            Self::Calibration => CalibrationAction::CycleCalibration,
            Self::Shape => CalibrationAction::NextShape,
        }
    }

    /// Held arrows repeat; toggles fire once per press
    pub fn repeats(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }
}

/// Calibration commands shared by every input source (local console, terminal keys, later the gamepad).
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub enum CalibrationAction {
    /// Next step of [`CalibrationOverlays`]; the last step turns calibration off, the next one on
    CycleCalibration,
    ToggleLaser,
    /// Next shape in place of the centre crosshair
    NextShape,
    /// Cycle to the next [`CalibrationMode`]
    NextMode,
    /// Arrow key: `dir` is (-1/0/1, -1/0/1), `fine` uses the small step
    Nudge { dir: IVec2, fine: bool },
}

/// Which calibration overlays are shown. The calibration key steps through them in order.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationOverlays {
    /// Red DAC perimeter, green scene box, centre shape and mouse crosshairs
    #[default]
    All,
    /// Red DAC perimeter hidden
    NoPerimeter,
    /// Only the centre shape and mouse crosshairs
    CrosshairOnly,
    /// Nothing; calibration is off
    Off,
}

impl CalibrationOverlays {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::NoPerimeter,
            Self::NoPerimeter => Self::CrosshairOnly,
            Self::CrosshairOnly => Self::Off,
            Self::Off => Self::All,
        }
    }

    pub fn calibration_on(self) -> bool {
        self != Self::Off
    }

    pub fn shows_perimeter(self) -> bool {
        self == Self::All
    }

    pub fn shows_scene_box(self) -> bool {
        matches!(self, Self::All | Self::NoPerimeter)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all on",
            Self::NoPerimeter => "red DAC perimeter off",
            Self::CrosshairOnly => "red perimeter and green scene box off",
            Self::Off => "all off (calibration off)",
        }
    }
}

/// What the arrow keys adjust while calibration is on.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationMode {
    /// Left/Right width, Up/Down height
    #[default]
    SceneSize,
    /// Up/Down scene middle height from ground, Left/Right distance (Right = farther)
    ScenePosition,
    /// Up/Down camera + projector rig height, Left/Right projector FOV
    Rig,
    /// Up/Down choose a laser timing value, Left/Right change it. Applied by the server, which
    /// owns the laser stage settings; not a physical setup change.
    LaserTiming,
}

impl CalibrationMode {
    pub fn next(self) -> Self {
        match self {
            Self::SceneSize => Self::ScenePosition,
            Self::ScenePosition => Self::Rig,
            Self::Rig => Self::LaserTiming,
            Self::LaserTiming => Self::SceneSize,
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Self::SceneSize => "SCENE SIZE: Left/Right width, Up/Down height",
            Self::ScenePosition => "SCENE POSITION: Up/Down middle height from ground, Left/Right distance",
            Self::Rig => "RIG: Up/Down camera + projector height, Left/Right projector FOV",
            Self::LaserTiming => "LASER TIMING: Up/Down choose value, Left/Right change it ([Shift] fine)",
        }
    }
}

const STEP_M: f32 = 0.05;
const FINE_STEP_M: f32 = 0.01;
const STEP_DEG: f32 = 0.5;
const FINE_STEP_DEG: f32 = 0.1;

/// Applies one arrow-key nudge for `mode` to the physical setup. Returns `false` when the setup
/// refused it (the scene would leave the red DAC perimeter); see the [`SceneSetup`] operations.
pub fn nudge(mode: CalibrationMode, dir: IVec2, fine: bool, setup: &mut SceneSetup) -> bool {
    let step = if fine { FINE_STEP_M } else { STEP_M };
    let d = dir.as_vec2();
    match mode {
        CalibrationMode::SceneSize => setup.resize_scene(d * step),
        CalibrationMode::ScenePosition => setup.move_scene(d.y * step, d.x * step),
        CalibrationMode::Rig => {
            let deg = if fine { FINE_STEP_DEG } else { STEP_DEG };
            let mut accepted = true;
            if dir.y != 0 {
                accepted &= setup.move_rig(d.y * step);
            }
            if dir.x != 0 {
                accepted &= setup.change_projector_fov(d.x * deg);
            }
            accepted
        }
        // Not a physical setup change; the server applies it to the laser stage settings
        CalibrationMode::LaserTiming => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlays_cycle_off_one_by_one_then_all_on() {
        let steps: Vec<CalibrationOverlays> =
            std::iter::successors(Some(CalibrationOverlays::All), |o| Some(o.next())).take(5).collect();
        use CalibrationOverlays::*;
        assert_eq!(steps, vec![All, NoPerimeter, CrosshairOnly, Off, All]);
        assert!(All.shows_perimeter() && All.shows_scene_box());
        assert!(!NoPerimeter.shows_perimeter() && NoPerimeter.shows_scene_box());
        assert!(!CrosshairOnly.shows_scene_box() && CrosshairOnly.calibration_on());
        assert!(!Off.calibration_on());
    }

    #[test]
    fn modes_cycle_size_position_rig_timing() {
        let mode = CalibrationMode::default();
        assert_eq!(mode, CalibrationMode::SceneSize);
        assert_eq!(mode.next(), CalibrationMode::ScenePosition);
        assert_eq!(mode.next().next(), CalibrationMode::Rig);
        assert_eq!(mode.next().next().next(), CalibrationMode::LaserTiming);
        assert_eq!(mode.next().next().next().next(), CalibrationMode::SceneSize);
    }

    /// Each mode drives the matching setup operation with the coarse or fine step
    #[test]
    fn modes_map_arrows_to_setup_operations() {
        let cases: [(CalibrationMode, IVec2, bool, fn(&mut SceneSetup) -> bool); 6] = [
            (CalibrationMode::SceneSize, IVec2::X, false, |s| s.resize_scene(Vec2::new(STEP_M, 0.0))),
            (CalibrationMode::SceneSize, IVec2::NEG_Y, true, |s| s.resize_scene(Vec2::new(0.0, -FINE_STEP_M))),
            (CalibrationMode::ScenePosition, IVec2::Y, false, |s| s.move_scene(STEP_M, 0.0)),
            (CalibrationMode::ScenePosition, IVec2::X, false, |s| s.move_scene(0.0, STEP_M)),
            (CalibrationMode::Rig, IVec2::NEG_Y, false, |s| s.move_rig(-STEP_M)),
            (CalibrationMode::Rig, IVec2::X, true, |s| s.change_projector_fov(FINE_STEP_DEG)),
        ];
        for (mode, dir, fine, expected) in cases {
            let mut nudged = SceneSetup::default();
            let mut direct = SceneSetup::default();
            assert!(nudge(mode, dir, fine, &mut nudged));
            assert!(expected(&mut direct));
            assert_eq!(nudged, direct, "{:?} {:?} fine={}", mode, dir, fine);
        }
    }

    #[test]
    fn refused_nudge_reports_false() {
        let mut setup = SceneSetup::default();
        while nudge(CalibrationMode::SceneSize, IVec2::X, false, &mut setup) {}
        let before = setup.clone();
        assert!(!nudge(CalibrationMode::SceneSize, IVec2::X, false, &mut setup));
        assert_eq!(setup, before);
    }

    #[test]
    fn terminal_key_names_map_to_control_keys() {
        assert_eq!(ControlKey::from_key_name("ArrowUp"), Some(ControlKey::Up));
        assert_eq!(ControlKey::from_key_name("Space"), Some(ControlKey::Space));
        assert_eq!(ControlKey::from_key_name("F3"), Some(ControlKey::Calibration));
        assert_eq!(ControlKey::from_key_name("KeyC"), Some(ControlKey::Calibration));
        assert_eq!(ControlKey::from_key_name("F1"), Some(ControlKey::Laser));
        assert_eq!(ControlKey::from_key_name("KeyL"), Some(ControlKey::Laser));
        assert_eq!(ControlKey::from_key_name("KeyS"), Some(ControlKey::Shape));
        assert_eq!(ControlKey::from_key_name("KeyA"), None);
    }

    #[test]
    fn arrows_nudge_and_function_keys_toggle() {
        assert_eq!(ControlKey::Left.action(true), CalibrationAction::Nudge { dir: IVec2::NEG_X, fine: true });
        assert_eq!(ControlKey::Laser.action(false), CalibrationAction::ToggleLaser);
        assert_eq!(ControlKey::Calibration.action(false), CalibrationAction::CycleCalibration);
        assert_eq!(ControlKey::Shape.action(false), CalibrationAction::NextShape);
        assert_eq!(ControlKey::Space.action(false), CalibrationAction::NextMode);
    }
}
