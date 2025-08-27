use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::window::WindowResized;
use crate::plugins::instructions::InstructionState;
use crate::plugins::scene::{SceneData, SceneSystemSet, SceneTag};
use crate::plugins::config::{ConfigState, DisplayMode};
use bevy::render::camera::ScalingMode;
pub struct CameraPlugin;

const INSTRUCTION_F2: &str = "Press [F2] to toggle between 2d and 3d display mode";

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CameraSystemSet;

#[derive(Component)]
pub struct CameraTag;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_camera.in_set(CameraSystemSet).after(SceneSystemSet))
        .add_systems(Update, update_camera.in_set(CameraSystemSet).after(SceneSystemSet));
    }
}


fn setup_camera(
    mut commands: Commands, 
    mut instruction_state: ResMut<InstructionState>,
    scene_query: Query<(&SceneData), With<SceneTag>>,
    config: Res<ConfigState>) {
    
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
        get_projection(config.display_mode, scene_data.dimensions.x, scene_data.dimensions.y),
        Transform::from_translation(config.termocamera_origin)
            .looking_at(config.termocamera_looking_at, Vec3::Y),
        ));
    }

}

fn update_camera(
    mut resize_events: EventReader<WindowResized>,
    mut camera_query: Query<(&mut Camera, &mut Projection, &mut Transform), With<CameraTag>>,
    scene_query: Query<(&GlobalTransform, &SceneData), With<SceneTag>>,
    mut config: ResMut<ConfigState>,
    keyboard: Res<ButtonInput<KeyCode>>
) {
    if let Ok((mut camera, mut projection, mut transform)) = camera_query.single_mut() {
      for (scene_transform,scene_data) in scene_query.iter() {

        configure_camera(&mut config, &keyboard);
        transform.look_at(scene_transform.translation(), Vec3::Y);
      
        for _event in resize_events.read() {
  
            if let Some(viewport) = camera.viewport.as_mut() {

                viewport.physical_position = scene_data.get_viewport_position();
                viewport.physical_size = scene_data.get_viewport_size();

            }
        }

        if config.is_changed() { 
            *projection = get_projection(config.display_mode,scene_data.dimensions.x, scene_data.dimensions.y);
        }
         
      }
       
    }
}


fn get_projection(display_mode: DisplayMode,w:f32, h:f32) -> Projection {
    match display_mode {
        
        DisplayMode::Mode2D => Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::Fixed { width: w, height: h },
            ..OrthographicProjection::default_2d()
        }),
        
        DisplayMode::Mode3D => Projection::from(PerspectiveProjection {
            fov: std::f32::consts::PI / 4.0,
            near: 0.1,
            far: 1000.0,
            ..default()
        }),
    }
}

fn configure_camera(config: &mut ConfigState, keyboard: &Res<ButtonInput<KeyCode>>){

    if keyboard.just_pressed(KeyCode::F2) {
        if config.display_mode == DisplayMode::Mode2D {
            config.display_mode = DisplayMode::Mode3D;
        } else {
            config.display_mode = DisplayMode::Mode2D;
        }    
    }
}
