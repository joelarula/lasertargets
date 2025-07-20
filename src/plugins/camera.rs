use bevy::prelude::*;
use bevy::render::camera::Viewport;
use bevy::color::palettes::css::FOREST_GREEN;
use bevy::window::WindowResized;
use crate::plugins::config::ConfigState;


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
    
    //let window_size = window.resolution.physical_size().as_vec2();
    
    commands.spawn((
        Camera2d,        
        Camera {
          // clear_color: ClearColorConfig::Custom(Color::srgba(10.0, 10.0, 10.0, 1.)), 
            clear_color: ClearColorConfig::Custom(Color::from(FOREST_GREEN).with_alpha(0.5)), 
            viewport: Some(Viewport {
                physical_position: UVec2::new(100,100),
                physical_size: config.termocam_size,
                ..default()
            }),
            ..default()
        }));
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

                let window_physical_size = window.physical_size();
            //    let scale_factor = window.scale_factor();
            //    let physical_viewport_size = ( config.termocam_size * scale_factor as f32).as_uvec2();
               
            // let window_size = window.resolution.physical_size().as_vec2();
                
                viewport.physical_size = config.termocam_size;

                let window_center_x = window_physical_size.x / 2;
                let window_center_y = window_physical_size.y / 2;

                let viewport_center_x = viewport.physical_size.x / 2;
                let viewport_center_y = viewport.physical_size.y / 2;

                let physical_position = UVec2::new(
                    window_center_x.saturating_sub(viewport_center_x),
                    window_center_y.saturating_sub(viewport_center_y),
                );

                viewport.physical_position = physical_position;

            }



        };

    }
}