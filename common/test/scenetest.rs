use crate::{
    config::{CameraConfiguration, ConfigTransform, ProjectorConfiguration, SceneConfiguration},
    scene::SceneSetup,
};
use bevy::{
    math::{Quat, UVec2, Vec2, Vec3},
    transform::components::Transform,
};

#[test]
fn test_get_camera_view_dimensions() {
    let scene_config = SceneConfiguration {
        scene_width: 10.0,
        transform: ConfigTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    };

    // Camera at (0, 0, 10), looking at (0, 0, 0)
    // Angle 90 degrees.
    // Distance = 10.
    // Half angle = 45 degrees. tan(45) = 1.
    // Width = 2 * 10 * 1 = 20.
    let camera_config = CameraConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.0, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: 90.0,
        locked_to_scene: false,
    };

    let projector_config = ProjectorConfiguration::default();

    let scene_setup = SceneSetup::new(scene_config, camera_config, projector_config);
    let dims = scene_setup.get_camera_view_dimensions();

    assert!((dims.x - 20.0).abs() < 1e-5);
    assert!((dims.y - 20.0).abs() < 1e-5);
}

#[test]
fn test_get_projector_view_dimensions() {
    let scene_config = SceneConfiguration {
        scene_width: 10.0,
        transform: ConfigTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    };

    let camera_config = CameraConfiguration::default();

    // Projector at (0, 0, 5), looking at (0, 0, 0)
    // Angle 60 degrees.
    // Distance = 5.
    // Half angle = 30 degrees. tan(30) approx 0.57735.
    // Width = 2 * 5 * tan(30) = 10 * 0.57735 = 5.7735.
    let projector_config = ProjectorConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.0, 0.0, 5.0),
            rotation: Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: 60.0,
        enabled: true,
        locked_to_scene: false,
    };

    let scene_setup = SceneSetup::new(scene_config, camera_config, projector_config);
    let dims = scene_setup.get_projector_view_dimensions();

    let expected_width = 2.0 * 5.0 * (30.0f32.to_radians().tan());
    assert!((dims.x - expected_width).abs() < 1e-5);
    assert!((dims.y - expected_width).abs() < 1e-5);
}

#[test]
fn test_get_camera_center_on_scene_plane() {
    let scene_config = SceneConfiguration {
        scene_width: 10.0,
        transform: ConfigTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    };

    // Camera at (0, 0, 10), looking directly at scene center.
    let camera_config = CameraConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.0, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: 45.0,
        locked_to_scene: false,
    };

    let projector_config = ProjectorConfiguration::default();
    let scene_setup = SceneSetup::new(scene_config, camera_config, projector_config);

    let center = scene_setup.get_camera_center_on_scene_plane();
    assert!(center.is_some());
    let center = center.unwrap();

    // Should be at (0, 0, 0)
    assert!(center.distance(Vec3::ZERO) < 1e-5);
}

#[test]
fn test_get_common_viewport_stats_full_overlap() {
    let scene_config = SceneConfiguration {
        scene_width: 10.0,
        transform: ConfigTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    };

    // Camera covers 20x20 area centered at 0,0
    // Camera at (0, 0, 10), angle 90 -> width 20
    let camera_config = CameraConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.0, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: 90.0,
        locked_to_scene: false,
    };

    // Projector covers 10x10 area centered at 0,0
    // Projector at (0, 0, 5), angle 90 -> width 10
    let projector_config = ProjectorConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.0, 0.0, 5.0),
            rotation: Transform::from_translation(Vec3::new(0.0, 0.0, 5.0))
                .looking_at(Vec3::ZERO, Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: 90.0,
        enabled: true,
        locked_to_scene: false,
    };

    let scene_setup = SceneSetup::new(scene_config, camera_config, projector_config);
    let stats = scene_setup.get_common_viewport_stats();

    assert!(stats.is_some());
    let (width, height, center) = stats.unwrap();

    // Intersection should be the projector's area (10x10) because it's fully inside camera's
    assert!((width - 10.0).abs() < 1e-4);
    assert!((height - 10.0).abs() < 1e-4);
    assert!(center.distance(Vec3::ZERO) < 1e-5);
}

#[test]
fn test_get_common_viewport_stats_partial_overlap() {
    let scene_config = SceneConfiguration {
        scene_width: 10.0,
        transform: ConfigTransform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        },
    };

    // Camera: width 2 centered at (1, 0)
    // Camera at (1, 0, 10). To have width 2 at dist 10:
    // 2 = 2 * 10 * tan(a/2) -> tan(a/2) = 0.1 -> a = 2 * atan(0.1)
    let camera_angle = 2.0 * 0.1f32.atan().to_degrees();

    let camera_config = CameraConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(1.0, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(1.0, 0.0, 10.0))
                .looking_at(Vec3::new(1.0, 0.0, 0.0), Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: camera_angle,
        locked_to_scene: false,
    };

    // Projector: width 2 centered at (-1, 0)
    // Projector at (-1, 0, 10). Same angle.
    let projector_angle = camera_angle; // same dimensions

    let projector_config = ProjectorConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(-1.0, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(-1.0, 0.0, 10.0))
                .looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: projector_angle,
        enabled: true,
        locked_to_scene: false,
    };

    // Camera range x: [0, 2]
    // Projector range x: [-2, 0]
    // Overlap: x=0 (point), width 0. It might return 0 width.

    // Let's shift them closer to have proper overlap.
    // Camera centered at (0.5, 0), width 2 => range [-0.5, 1.5]
    // Projector centered at (-0.5, 0), width 2 => range [-1.5, 0.5]
    // Overlap: [-0.5, 0.5] => width 1. Center (0, 0).

    let camera_config_shifted = CameraConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(0.5, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(0.5, 0.0, 10.0))
                .looking_at(Vec3::new(0.5, 0.0, 0.0), Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: camera_angle,
        locked_to_scene: false,
    };

    let projector_config_shifted = ProjectorConfiguration {
        resolution: UVec2::new(100, 100),
        transform: ConfigTransform {
            translation: Vec3::new(-0.5, 0.0, 10.0),
            rotation: Transform::from_translation(Vec3::new(-0.5, 0.0, 10.0))
                .looking_at(Vec3::new(-0.5, 0.0, 0.0), Vec3::Y)
                .rotation,
            scale: Vec3::ONE,
        },
        angle: projector_angle,
        enabled: true,
        locked_to_scene: false,
    };

    let scene_setup = SceneSetup::new(
        scene_config,
        camera_config_shifted,
        projector_config_shifted,
    );
    let stats = scene_setup.get_common_viewport_stats();

    assert!(stats.is_some());
    let (width, height, center) = stats.unwrap();

    // println!("Stats: width={}, height={}, center={:?}", width, height, center);
    assert!(
        (width - 1.0).abs() < 0.01,
        "Stats: width={}, height={}, center={:?}",
        width,
        height,
        center
    );
    // height should be full 2.0 since they are aligned on Y
    assert!((height - 2.0).abs() < 0.01);
    assert!(center.distance(Vec3::ZERO) < 1e-4);
}
