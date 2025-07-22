use bevy::prelude::*;
use bevy::render::camera::Viewport;
//use bevy::color::palettes::css::FOREST_GREEN;
use bevy::window::WindowResized;
use crate::plugins::config::ConfigState;
use crate::util::scale::ScaleCalculations;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_camera)
        .add_systems(FixedUpdate, update_camera)
        .add_systems(Update, on_window_resize);
    }
}

fn setup_camera(mut commands: Commands,window: Single<&Window>, config: ResMut<ConfigState>) {
    
    let calc = ScaleCalculations::new(
        window.physical_size(),
        config.termocamera_size,
        window.scale_factor()
    ); 

    commands.spawn((
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
        Projection::from(PerspectiveProjection {
           fov: std::f32::consts::PI / 3.5,
           near: 0.1, 
           far: 1000.0,    
            ..default()
        }),
        Transform::from_translation(config.termocamera_origin)
            .looking_at(config.termocamera_looking_at, Vec3::Y),
    ));



}

fn update_camera(
     mut camera_query: Query<(&mut Camera, &mut Transform, &mut Projection)>, 
     window: Single<&Window>,) {

    if let Ok(mut camera) = camera_query.single_mut()  {
      
        if let Some(viewport) = camera.0.viewport.as_mut() {

            //let window_size = window.resolution.physical_size().as_vec2();
         //   viewport.physical_position = UVec2::new(100,100);
          //  viewport.physical_size = UVec2::new(800,600);

        }

    };

}

fn on_window_resize( 
    mut resize_events: EventReader<WindowResized>,
    window: Single<&Window>, 
    config: ResMut<ConfigState>,
    mut camera_query: Query<(&mut Camera, &mut Transform, &mut Projection)>) {
    for _event in resize_events.read() {

        if let Ok(mut camera) = camera_query.single_mut()  {
      
            if let Some(viewport) = camera.0.viewport.as_mut() {

                let calc = ScaleCalculations::new(
                    window.physical_size(),
                    config.termocamera_size,
                    window.scale_factor()
                ); 
             
                viewport.physical_position = calc.get_viewport_position();
                viewport.physical_size = calc.get_viewport_size();

            }

        };

    }
}

