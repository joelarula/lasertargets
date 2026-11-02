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

use crate::config::{CameraConfiguration, ProjectorConfiguration, SceneConfiguration};
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
const MIN_SCENE_SIZE: f32 = 0.1;
/// Closest the scene may come to the projector along Z
const MIN_SCENE_DISTANCE: f32 = 0.5;
const PROJECTOR_FOV_RANGE: (f32, f32) = (5.0, 90.0);

/// Applies one arrow-key nudge for `mode`. Camera and projector move together as one rig.
pub fn nudge(
    mode: CalibrationMode,
    dir: IVec2,
    fine: bool,
    scene: &mut SceneConfiguration,
    projector: &mut ProjectorConfiguration,
    camera: &mut CameraConfiguration,
) {
    let step = if fine { FINE_STEP_M } else { STEP_M };
    let dx = dir.x as f32;
    let dy = dir.y as f32;

    match mode {
        CalibrationMode::SceneSize => {
            scene.scene_dimension.x = (scene.scene_dimension.x + dx * step).max(MIN_SCENE_SIZE);
            scene.scene_dimension.y = (scene.scene_dimension.y + dy * step).max(MIN_SCENE_SIZE);
        }
        CalibrationMode::ScenePosition => {
            scene.origin.translation.y += dy * step;
            // The rig looks down -Z: farther means more negative Z
            let z = scene.origin.translation.z - dx * step;
            scene.origin.translation.z = z.min(projector.origin.translation.z - MIN_SCENE_DISTANCE);
        }
        CalibrationMode::Rig => {
            projector.origin.translation.y += dy * step;
            camera.origin.translation.y += dy * step;
            let deg = if fine { FINE_STEP_DEG } else { STEP_DEG };
            projector.angle = (projector.angle + dx * deg).clamp(PROJECTOR_FOV_RANGE.0, PROJECTOR_FOV_RANGE.1);
        }
    }
}

pub fn log_calibration_status(
    mode: CalibrationMode,
    scene: &SceneConfiguration,
    projector: &ProjectorConfiguration,
    camera: &CameraConfiguration,
) {
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

    fn configs() -> (SceneConfiguration, ProjectorConfiguration, CameraConfiguration) {
        (SceneConfiguration::default(), ProjectorConfiguration::default(), CameraConfiguration::default())
    }

    #[test]
    fn modes_cycle_size_position_rig() {
        let mode = CalibrationMode::default();
        assert_eq!(mode, CalibrationMode::SceneSize);
        assert_eq!(mode.next(), CalibrationMode::ScenePosition);
        assert_eq!(mode.next().next(), CalibrationMode::Rig);
        assert_eq!(mode.next().next().next(), CalibrationMode::SceneSize);
    }

    #[test]
    fn scene_size_mode_changes_width_and_height_only() {
        let (mut scene, mut projector, mut camera) = configs();
        let before = scene.clone();
        nudge(CalibrationMode::SceneSize, IVec2::X, false, &mut scene, &mut projector, &mut camera);
        nudge(CalibrationMode::SceneSize, IVec2::NEG_Y, true, &mut scene, &mut projector, &mut camera);
        assert!((scene.scene_dimension.x - (before.scene_dimension.x + STEP_M)).abs() < 1e-5);
        assert!((scene.scene_dimension.y - (before.scene_dimension.y - FINE_STEP_M)).abs() < 1e-5);
        assert_eq!(scene.origin, before.origin);
        assert_eq!((projector, camera), (ProjectorConfiguration::default(), CameraConfiguration::default()));
    }

    #[test]
    fn scene_size_never_collapses() {
        let (mut scene, mut projector, mut camera) = configs();
        for _ in 0..1000 {
            nudge(CalibrationMode::SceneSize, IVec2::new(-1, -1), false, &mut scene, &mut projector, &mut camera);
        }
        assert_eq!(scene.scene_dimension, Vec2::splat(MIN_SCENE_SIZE));
    }

    #[test]
    fn scene_position_mode_moves_middle_height_and_distance() {
        let (mut scene, mut projector, mut camera) = configs();
        let before = scene.origin.translation;
        nudge(CalibrationMode::ScenePosition, IVec2::Y, false, &mut scene, &mut projector, &mut camera);
        nudge(CalibrationMode::ScenePosition, IVec2::X, false, &mut scene, &mut projector, &mut camera);
        let after = scene.origin.translation;
        assert!((after.y - (before.y + STEP_M)).abs() < 1e-5);
        // Right = farther away from the rig, which looks down -Z
        assert!((after.z - (before.z - STEP_M)).abs() < 1e-5);
        assert_eq!(projector.origin, ProjectorConfiguration::default().origin);
    }

    #[test]
    fn scene_cannot_move_through_the_rig() {
        let (mut scene, mut projector, mut camera) = configs();
        for _ in 0..1000 {
            nudge(CalibrationMode::ScenePosition, IVec2::NEG_X, false, &mut scene, &mut projector, &mut camera);
        }
        let distance = projector.origin.translation.z - scene.origin.translation.z;
        assert!((distance - MIN_SCENE_DISTANCE).abs() < 1e-5, "distance {}", distance);
    }

    #[test]
    fn rig_mode_moves_camera_and_projector_together_and_leaves_scene() {
        let (mut scene, mut projector, mut camera) = configs();
        let (p0, c0) = (projector.origin.translation, camera.origin.translation);
        nudge(CalibrationMode::Rig, IVec2::NEG_Y, false, &mut scene, &mut projector, &mut camera);
        assert_eq!(projector.origin.translation - p0, Vec3::new(0.0, -STEP_M, 0.0));
        assert_eq!(camera.origin.translation - c0, Vec3::new(0.0, -STEP_M, 0.0));
        assert_eq!(scene, SceneConfiguration::default());

        let angle = projector.angle;
        nudge(CalibrationMode::Rig, IVec2::X, true, &mut scene, &mut projector, &mut camera);
        assert!((projector.angle - (angle + FINE_STEP_DEG)).abs() < 1e-5);
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
