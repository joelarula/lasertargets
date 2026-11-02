//! Calibration controls shared by the server and the terminal: one key map, one set of actions
//! and one adjustment function. The server applies them authoritatively; a terminal without a
//! server connection applies them locally.
//!
//! | Key          | Action                               |
//! |--------------|--------------------------------------|
//! | F3           | calibration on/off                   |
//! | F1           | laser on/off                         |
//! | Space        | next calibration mode                |
//! | Arrows       | adjust (hold Shift for fine steps)   |

use crate::scene::SceneSetup;
use bevy::prelude::*;

pub const KEY_HELP: &str =
    "Keys: [F3] calibration on/off  [F1] laser on/off  [Space] next mode  [Arrows] adjust ([Shift] fine)";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlKey {
    Up,
    Down,
    Left,
    Right,
    Space,
    F1,
    F3,
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
            "F1" => Some(Self::F1),
            "F3" => Some(Self::F3),
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
            Self::F1 => CalibrationAction::ToggleLaser,
            Self::F3 => CalibrationAction::ToggleCalibration,
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
    ToggleCalibration,
    ToggleLaser,
    /// Cycle to the next [`CalibrationMode`]
    NextMode,
    /// Arrow key: `dir` is (-1/0/1, -1/0/1), `fine` uses the small step
    Nudge { dir: IVec2, fine: bool },
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
}

impl CalibrationMode {
    pub fn next(self) -> Self {
        match self {
            Self::SceneSize => Self::ScenePosition,
            Self::ScenePosition => Self::Rig,
            Self::Rig => Self::SceneSize,
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Self::SceneSize => "SCENE SIZE: Left/Right width, Up/Down height",
            Self::ScenePosition => "SCENE POSITION: Up/Down middle height from ground, Left/Right distance",
            Self::Rig => "RIG: Up/Down camera + projector height, Left/Right projector FOV",
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
    }
}

pub fn log_calibration_status(mode: CalibrationMode, setup: &SceneSetup) {
    let (scene, projector, camera) = (&setup.scene, &setup.projector, &setup.camera);
    match mode {
        CalibrationMode::SceneSize => info!(
            "[Calibration] SIZE      width {:.2} m  height {:.2} m",
            scene.scene_dimension.x, scene.scene_dimension.y
        ),
        CalibrationMode::ScenePosition => info!(
            "[Calibration] POSITION  middle height {:.2} m  distance {:.2} m",
            scene.origin.translation.y,
            projector.origin.translation.z - scene.origin.translation.z
        ),
        CalibrationMode::Rig => info!(
            "[Calibration] RIG       projector height {:.2} m  camera height {:.2} m  projector FOV {:.1} deg",
            projector.origin.translation.y, camera.origin.translation.y, projector.angle
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_cycle_size_position_rig() {
        let mode = CalibrationMode::default();
        assert_eq!(mode, CalibrationMode::SceneSize);
        assert_eq!(mode.next(), CalibrationMode::ScenePosition);
        assert_eq!(mode.next().next(), CalibrationMode::Rig);
        assert_eq!(mode.next().next().next(), CalibrationMode::SceneSize);
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
        assert_eq!(ControlKey::from_key_name("F3"), Some(ControlKey::F3));
        assert_eq!(ControlKey::from_key_name("KeyA"), None);
    }

    #[test]
    fn arrows_nudge_and_function_keys_toggle() {
        assert_eq!(ControlKey::Left.action(true), CalibrationAction::Nudge { dir: IVec2::NEG_X, fine: true });
        assert_eq!(ControlKey::F1.action(false), CalibrationAction::ToggleLaser);
        assert_eq!(ControlKey::F3.action(false), CalibrationAction::ToggleCalibration);
        assert_eq!(ControlKey::Space.action(false), CalibrationAction::NextMode);
    }
}
