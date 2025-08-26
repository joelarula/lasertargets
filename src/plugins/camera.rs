use bevy::prelude::*;
use bevy::render::camera::Viewport;
//use bevy::color::palettes::css::FOREST_GREEN;
use bevy::window::WindowResized;
use crate::plugins::scene::{SceneTag, SceneSystemSet};
use crate::plugins::config::{ConfigState, DisplayMode};
use crate::util::scale::ScaleCalculations;
use bevy::render::camera::ScalingMode;
pub struct CameraPlugin;

#[derive(Component)]
pub struct CameraTag;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_camera)
        .add_systems(Update, update_camera.after(SceneSystemSet));
    }
}


fn setup_camera(mut commands: Commands, window: Single<&Window>, config: Res<ConfigState>) {
    
    let calc = ScaleCalculations::new(
        window.physical_size(),
        config.termocamera_size,
        window.scale_factor()
    ); 
  
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
                physical_position: calc.get_viewport_position(),
                physical_size: calc.get_viewport_size(),
                ..default()
            }),
            ..default()
        },
        get_projection(config.display_mode, config.scene_width, calc.get_scene_height(config.scene_width)),
        Transform::from_translation(config.termocamera_origin)
            .looking_at(config.termocamera_looking_at, Vec3::Y),
    ));

}

fn update_camera(
    mut resize_events: EventReader<WindowResized>,
    mut camera_query: Query<(&mut Camera, &mut Projection, &mut Transform), With<CameraTag>>,
    scene_query: Query<&GlobalTransform, With<SceneTag>>,
    window: Single<&Window>, 
    mut config: ResMut<ConfigState>,
    keyboard: Res<ButtonInput<KeyCode>>
) {
    if let Ok((mut camera, mut projection, mut transform)) = camera_query.single_mut() {
    
        configure_camera(&mut config, &keyboard);

        if let Ok(scene_transform) = scene_query.single() {
            transform.look_at(scene_transform.translation(), Vec3::Y);
        }

        for _event in resize_events.read() {
  
            if let Some(viewport) = camera.viewport.as_mut() {

                let calc = ScaleCalculations::new(
                    window.physical_size(),
                    config.termocamera_size,
                    window.scale_factor()
                ); 
             
                viewport.physical_position = calc.get_viewport_position();
                viewport.physical_size = calc.get_viewport_size();

            }
        }

        if config.is_changed() {
    
            let calc = ScaleCalculations::new(
                window.physical_size(),
                config.termocamera_size,
                window.scale_factor()
            ); 
  

            *projection = get_projection(config.display_mode,config.scene_width, calc.get_scene_height(config.scene_width));
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
