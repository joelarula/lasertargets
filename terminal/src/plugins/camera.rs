use bevy::prelude::*;
use bevy::prelude::KeyCode;
use bevy::camera::Viewport;
use bevy::window::WindowResized;
use crate::plugins::instructions::{DebugInfoState, InstructionState};
use crate::plugins::scene::{SceneData, SceneSystemSet, SceneTag};
use bevy_camera::ScalingMode;
use common::config::CameraConfiguration;

pub struct CameraPlugin;

 /// Defines the display mode of the application (2D or 3D).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Resource)]
pub enum DisplayMode {
    #[default]
    Mode2D,
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
    scene_query: Query<(&SceneData), With<SceneTag>>,
    display_mode: Res<DisplayMode>,
    config: Res<CameraConfiguration>) {
    
     instruction_state.instructions.push(INSTRUCTION_F2.to_string());
     
     for (scene_data) in scene_query.iter() {

        commands.spawn((
            DirectionalLight::default(),
            Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
        ));

        commands.spawn((
            CameraTag,
            Camera3d::default(),       
            Camera {
                order: 1,
                viewport: Some(Viewport {
                physical_position: scene_data.get_viewport_position(),
                physical_size: scene_data.get_viewport_size(),
                ..default()
            }),
            ..default()
        },
        get_projection(*display_mode, scene_data.dimensions.x, scene_data.dimensions.y),
        Transform::from_translation(config.transform.translation)
            .looking_at(config.transform.translation, Vec3::Y),
        ));
    }

}

fn update_camera(
    mut resize_events: EventReader<WindowResized>,
    mut camera_query: Query<(&mut Camera, &mut Projection, &mut Transform), With<CameraTag>>,
    scene_query: Query<(&GlobalTransform, &SceneData), With<SceneTag>>,
    mut config: ResMut<CameraConfiguration>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_info: ResMut<DebugInfoState>,
    mut display_mode: ResMut<DisplayMode>,
) {
    if let Ok((mut camera, mut projection, mut transform)) = camera_query.single_mut() {
      for (scene_transform,scene_data) in scene_query.iter() {

        configure_camera(&mut display_mode, &keyboard);
        if *display_mode == DisplayMode::Mode2D {
            // For 2D mode, ensure the camera is looking straight down the Z-axis by aligning it with the scene center.
            // This resets the camera's orientation and X/Y position for a clean top-down view, while preserving Z-distance.
            let scene_center = scene_transform.translation();
            *transform = Transform::from_xyz(scene_center.x, scene_center.y, transform.translation.z)
                .looking_at(scene_center, Vec3::Y);
        } else {
            *transform = Transform::from_translation(config.transform.translation)
                .looking_at(scene_transform.translation(), Vec3::Y);
        }

        let looking_at = scene_transform.translation();
        debug_info.messages.push(format!(
            "Camera pos: x:{:.2} y:{:.2} z:{:.2}, looking at: x:{:.2} y:{:.2} z:{:.2}",
            transform.translation.x, transform.translation.y, transform.translation.z, looking_at.x, looking_at.y, looking_at.z
        ));
      
        for _event in resize_events.read() {
  
            if let Some(viewport) = camera.viewport.as_mut() {

                viewport.physical_position = scene_data.get_viewport_position();
                viewport.physical_size = scene_data.get_viewport_size();

            }
        }

        if config.is_changed() {
            *projection = get_projection(*display_mode, scene_data.dimensions.x, scene_data.dimensions.y);
        }
         
      }
       
    }
}


fn get_projection(display_mode: DisplayMode,w:f32, h:f32) -> Projection {
    match display_mode {
        
        DisplayMode::Mode2D => {
            // Scale to fit the largest dimension proportionally
            // This ensures the entire scene fits in the viewport
            let scale = w.max(h);
            Projection::from(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical { viewport_height: scale },
                ..OrthographicProjection::default_2d()
            })
        },
        
        DisplayMode::Mode3D => Projection::from(PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
    }
}

fn configure_camera(display_mode: &mut ResMut<DisplayMode>, keyboard: &Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::F2) {
        if **display_mode == DisplayMode::Mode2D {
            **display_mode = DisplayMode::Mode3D;
        } else {
            **display_mode = DisplayMode::Mode2D;
        }
    }
}

