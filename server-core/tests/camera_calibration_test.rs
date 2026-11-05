//! Camera calibration end to end without hardware: a fake camera sees the projected dot through a
//! known perspective distortion, and calibration must recover that mapping, apply and save it.

use bevy::prelude::*;
use common::camera::{CameraCalibrationPhase, PhysicalCameraSettings};
use common::config::ProjectorConfiguration;
use common::path::UniversalPath;
use common::scene::SceneSetup;
use laser_vision::{Homography, VisionObject};
use server_core::plugins::camera_calibration::{
    CameraCalibration, CameraCalibrationCommand, CameraCalibrationPlugin, CameraCalibrationSet, CameraCalibrationSettings,
};
use server_core::plugins::network::FromClientMessage;
use server_core::plugins::projector::{ndc_to_dac, ProjectorSolo};
use server_core::plugins::vision::{CameraObservation, VisionCalibration, VisionMuted};
use std::path::PathBuf;
use test_harness::TestApp;

/// How the (fake) camera sees the projector: projector coordinates → normalised camera coordinates
#[derive(Resource)]
struct CameraView(Option<Homography>);

fn tilted_view() -> Homography {
    Homography::from_four_points(
        &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)],
        &[Vec2::new(0.12, 0.08), Vec2::new(0.9, 0.15), Vec2::new(0.85, 0.92), Vec2::new(0.08, 0.86)],
    )
    .unwrap()
}

/// Reports the solo dot where the camera would see it: the centre of its outline
fn fake_camera(view: Res<CameraView>, dots: Query<&UniversalPath, With<ProjectorSolo>>, mut out: MessageWriter<CameraObservation>) {
    let Some(view) = view.0 else { return };
    let dac_full = ndc_to_dac(Vec2::ONE);
    for path in &dots {
        let points = path.flatten();
        let min = points.iter().copied().reduce(Vec2::min).unwrap();
        let max = points.iter().copied().reduce(Vec2::max).unwrap();
        let ndc = (min + max) / 2.0 / dac_full * 2.0 - Vec2::ONE;
        out.write(CameraObservation {
            timestamp_ms: 0,
            objects: vec![VisionObject {
                track: 1,
                appeared: false,
                label: "laser".into(),
                camera_pos: view.transform_point(ndc),
                galvo_pos: Vec2::ZERO,
                galvo_bounds: [Vec2::ZERO; 2],
                confidence: 1.0,
            }],
        });
    }
}

fn calibration_file(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("lasertargets-{name}-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

fn app(view: Option<Homography>, file: &PathBuf) -> TestApp {
    let mut t = TestApp::new();
    let scene = SceneSetup::default();
    t.insert_resource(ProjectorConfiguration { switched_on: true, ..scene.projector.clone() })
        .insert_resource(scene)
        .insert_resource(PhysicalCameraSettings { enabled: true, ..Default::default() })
        .insert_resource(CameraCalibrationSettings {
            settle_ms: 40.0,
            measure_ms: 60.0,
            file: file.to_string_lossy().into_owned(),
            ..Default::default()
        })
        .insert_resource(VisionCalibration::default())
        .insert_resource(VisionMuted::default())
        .insert_resource(CameraView(view));
    t.app_mut().add_message::<CameraObservation>().add_message::<FromClientMessage>();
    t.add_plugins(CameraCalibrationPlugin);
    t.add_systems(Update, fake_camera.before(CameraCalibrationSet));
    t
}

fn phase(t: &TestApp) -> CameraCalibrationPhase {
    t.resource::<CameraCalibration>().0.phase
}

#[test]
fn calibration_recovers_the_camera_view_and_saves_it() {
    let file = calibration_file("solve");
    let mut t = app(Some(tilted_view()), &file);
    t.step().send(CameraCalibrationCommand::Start).step();
    assert_eq!(phase(&t), CameraCalibrationPhase::Running);
    assert!(t.resource::<VisionMuted>().0, "nothing reacts to the calibration dots");

    let frames = t.step_until(500, |w| w.resource::<CameraCalibration>().0.phase != CameraCalibrationPhase::Running);
    assert!(frames < 500, "calibration finished");

    let status = &t.resource::<CameraCalibration>().0;
    assert_eq!(status.phase, CameraCalibrationPhase::Solved, "{}", status.message);
    assert_eq!(status.measured_count(), 9);
    assert!(status.rms_error_m.unwrap() < 0.005, "{}", status.message);
    assert!(!t.resource::<VisionMuted>().0);
    assert_eq!(t.count::<ProjectorSolo>(), 0, "the dot is gone");

    // The solved mapping undoes the camera's view: camera → projector
    let solved = t.resource::<VisionCalibration>().0.expect("calibration applied");
    for ndc in [Vec2::ZERO, Vec2::new(0.3, -0.2), Vec2::new(-0.25, 0.3)] {
        let back = solved.transform_point(tilted_view().transform_point(ndc));
        assert!(back.distance(ndc) < 0.01, "{ndc} -> {back}");
    }

    // A restart loads it again
    assert!(file.exists());
    let mut restarted = app(None, &file);
    restarted.step();
    assert!(restarted.resource::<VisionCalibration>().0.is_some());
    assert!(restarted.resource::<CameraCalibration>().0.calibrated);

    // Reset forgets it
    restarted.send(CameraCalibrationCommand::Reset).step();
    assert!(restarted.resource::<VisionCalibration>().0.is_none());
    assert!(!file.exists());
}

#[test]
fn a_camera_that_sees_nothing_fails_and_keeps_the_previous_mapping() {
    let file = calibration_file("blind");
    let mut t = app(None, &file);
    let previous = Homography::identity();
    t.resource_mut::<VisionCalibration>().0 = Some(previous);

    t.step().send(CameraCalibrationCommand::Start).step();
    t.step_until(500, |w| w.resource::<CameraCalibration>().0.phase != CameraCalibrationPhase::Running);

    assert_eq!(phase(&t), CameraCalibrationPhase::Failed);
    assert_eq!(t.resource::<VisionCalibration>().0, Some(previous));
    assert!(!t.resource::<VisionMuted>().0);
    assert!(!file.exists());
}

#[test]
fn cancel_stops_and_removes_the_dot() {
    let file = calibration_file("cancel");
    let mut t = app(Some(tilted_view()), &file);
    t.step().send(CameraCalibrationCommand::Start).step_n(3);
    assert_eq!(t.count::<ProjectorSolo>(), 1);

    t.send(CameraCalibrationCommand::Cancel).step();
    assert_eq!(phase(&t), CameraCalibrationPhase::Idle);
    assert_eq!(t.count::<ProjectorSolo>(), 0);
    assert!(t.resource::<VisionCalibration>().0.is_none());
}
