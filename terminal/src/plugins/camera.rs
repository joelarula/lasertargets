use bevy::prelude::*;
use bevy::prelude::KeyCode;
use common::scene::{SceneSystemSet, SceneSetup};
use crate::plugins::instructions::{InstructionState};
use common::config::{CameraConfiguration, SceneConfiguration};

pub struct CameraPlugin;

 /// Defines the display mode of the application (2D or 3D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Resource)]
pub enum DisplayMode {
    Mode2D,
    #[default]
    Mode3D,
}


const INSTRUCTION_F2: &str = "Press [F2] to toggle between 2d and 3d display mode";

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CameraSystemSet;

#[derive(Component)]
pub struct CameraTag;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(DisplayMode::default())
        .insert_resource(CameraConfiguration::default())
        .add_systems(Startup, (setup_camera).chain().in_set(CameraSystemSet).after(SceneSystemSet))
        .add_systems(Update, update_camera.in_set(CameraSystemSet).after(SceneSystemSet));
    }
}


fn setup_camera(
    mut commands: Commands, 
    mut instruction_state: ResMut<InstructionState>,
    scene_setup: Res<SceneSetup>,
    scene_config: Res<SceneConfiguration>,
    display_mode: Res<DisplayMode>,
    config: Res<CameraConfiguration>) {
    
    instruction_state.instructions.push(INSTRUCTION_F2.to_string());
      
    commands.spawn((
            DirectionalLight::default(),
            Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
            CameraTag,
            Camera3d::default(),       
            Camera {
                order: 1,
                ..default()
            },
        get_projection(*display_mode, scene_setup.camera.angle, &scene_config),
        Transform::from_translation(config.origin.translation)
            .looking_at(scene_setup.scene.origin.translation, Vec3::Y),
        ));
    

}

fn update_camera(
    mut camera_query: Query<(&mut Camera, &mut Projection, &mut Transform), With<CameraTag>>,
    scene_setup: Res<SceneSetup>,
    scene_config: Res<SceneConfiguration>,
    config: Res<CameraConfiguration>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut display_mode: ResMut<DisplayMode>,
) {
    if let Ok((mut _camera, mut projection, mut transform)) = camera_query.single_mut() {

        configure_camera(&mut display_mode, &keyboard);
        if *display_mode == DisplayMode::Mode2D {
            // For 2D mode, ensure the camera is looking straight down the Z-axis by aligning it with the scene center.
            // This resets the camera's orientation and X/Y position for a clean top-down view, while preserving Z-distance.
            let scene_center = scene_setup.scene.origin.translation;
            *transform = Transform::from_xyz(scene_center.x, scene_center.y, transform.translation.z)
                .looking_at(scene_center, Vec3::Y);
        } else {
            *transform = Transform::from_translation(config.origin.translation)
                .looking_at(scene_setup.scene.origin.translation, Vec3::Y);
        }

        if config.is_changed() || scene_setup.is_changed() || scene_config.is_changed() {
            *projection = get_projection(*display_mode, scene_setup.camera.angle, &scene_config);
        }
         
       
    }
}


fn get_projection(display_mode: DisplayMode, camera_angle: f32, scene_config: &SceneConfiguration) -> Projection {
    match display_mode {
        
        DisplayMode::Mode2D => {
            // Calculate the scene dimensions to fit the camera view
            let scene_width = scene_config.scene_dimension.x as f32;
            let scene_height = scene_config.scene_dimension.y as f32;
            
            // Add some padding to ensure the scene is fully visible
            let padding = 50.0;
            let total_width = scene_width + padding;
            let total_height = scene_height + padding;
            
            // Use the area field to define the orthographic bounds
            let half_width = total_width / 2.0;
            let half_height = total_height / 2.0;
            
            Projection::from(OrthographicProjection {
                area: bevy::math::Rect::from_center_half_size(
                    bevy::math::Vec2::ZERO, 
                    bevy::math::Vec2::new(half_width, half_height)
                ),
                ..OrthographicProjection::default_3d()
            })
        },
        
        DisplayMode::Mode3D => Projection::from(PerspectiveProjection {
            fov: camera_angle.to_radians(),
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
    }
}

fn configure_camera(
    display_mode: &mut ResMut<DisplayMode>, 
    keyboard: &Res<ButtonInput<KeyCode>>
) {
    if keyboard.just_pressed(KeyCode::F2) {
        if **display_mode == DisplayMode::Mode2D {
            **display_mode = DisplayMode::Mode3D;
        } else {
            **display_mode = DisplayMode::Mode2D;
        }
    }
}

