use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
use crate::plugins::camera::CameraTag;
use bevy::prelude::UVec2;
use bevy::prelude::Vec2;

#[derive(Component)]
pub struct SceneTag;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneSystemSet;

pub struct ScenePlugin;


#[derive(Component,Debug, Default,Clone, Copy)] 
pub struct SceneData{
    /// The dimensions of the scene in world units.
    pub dimensions: Vec2,
    /// Distance from the camera to the scene in world units.
    pub distance: f32,
    /// The size of the window in pixels.
    pub window_size: UVec2,
    /// scale factor of the window.
    pub scale_factor: f32,
    /// The size of the viewport in pixels.
    pub viewport_size: UVec2,
     /// The size of the camera input in pixels.   
    pub camera_input_size: UVec2,
    /// Most recent calculated world position of the mouse cursor intersection with the scene, if any.
    pub mouse_world_pos: Option<Vec3>
}

impl SceneData {

   pub fn new(
        window_size: UVec2, 
        camera_input_size: UVec2, 
        distance: f32,
        scene_width: f32,
        mouse_pos: Option<Vec3>, 
        scale_factor:f32
        ) -> Self {
      
        SceneData{
            dimensions: Self::get_scene_dimensions(scene_width,camera_input_size),
            distance,
            window_size,
            scale_factor,
            viewport_size: Self::calculate_viewport_size(window_size, camera_input_size),
            camera_input_size,
            mouse_world_pos: mouse_pos,
       }
    }

   pub fn get_viewport_size(&self) -> UVec2 { 
      return self.viewport_size;
   }

   pub fn get_window_scaled_size(&self) -> Vec2 {
      return self.window_size.as_vec2() / self.scale_factor;
   }

   pub fn get_viewport_scaled_position(&self) -> Vec2 {
      return self.get_viewport_position().as_vec2() / self.scale_factor;
   }

   pub fn get_viewport_scaled_size(&self) -> Vec2 {
      return self.get_viewport_size().as_vec2() / self.scale_factor;
   }

   pub fn get_viewport_cursor_coordinates(&self, cursor_pos: Vec2) -> Vec2 {
      let viewport_pos = self.get_viewport_scaled_position();
      let viewport_relative_cursor_pos = cursor_pos - viewport_pos;
      return viewport_relative_cursor_pos * self.scale_factor;
   }

   pub fn translate_viewport_coordinates_to_2d_world(&self, viewport_pos: Vec2) -> Vec2 {
      let center = self.get_viewport_size().as_vec2() / 2.0; 
      let world_pos = viewport_pos - center;
      return world_pos / self.scale_factor;

   }

   pub fn translate_viewport_coordinates_to_window_coordinates(&self, viewport_pos: Vec2) -> Vec2 {
      let relative_pos = viewport_pos /  self.scale_factor;
      return self.get_viewport_scaled_position() + relative_pos;
   }

     
   pub fn get_viewport_position(&self) -> UVec2 {              
      let window_center = self.window_size /  2;
      let viewport_center = self.get_viewport_size() / 2;

         return UVec2::new(
            window_center.x.saturating_sub(viewport_center.x),
            window_center.y.saturating_sub(viewport_center.y),
         );
   }
    

   fn get_scene_dimensions(scene_width: f32, camera_input_size: UVec2) -> Vec2 {
      let aspect_ratio = camera_input_size.y as f32 / camera_input_size.x as f32;
      Vec2::new(scene_width, scene_width * aspect_ratio)
   }

   fn calculate_viewport_size(window_size: UVec2, camera_input_size: UVec2) -> UVec2 { 
      
      let hscale = window_size.x as f32 / camera_input_size.x as f32;
      let vscale = window_size.y as f32 / camera_input_size.y as f32;
      let scale = hscale.min(vscale);   
      return  UVec2::new(
      (camera_input_size.x as f32 * scale).round() as u32, 
      (camera_input_size.y as f32 * scale).round() as u32
      );

   }

   fn get_normalized_mouse_position(&self) -> Vec2 {
      let local_pos_2d = self.mouse_world_pos.unwrap().truncate();
      let normalized_x = (local_pos_2d.x + self.dimensions.x / 2.0) / self.dimensions.x;
      let normalized_y = (local_pos_2d.y + self.dimensions.y / 2.0) / self.dimensions.y;
      Vec2::new(normalized_x, normalized_y)
   }


}



impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene);
        app.add_systems(Update,update_scene.in_set(SceneSystemSet),
        );
    }
}

fn setup_scene(mut commands: Commands, config: Res<ConfigState>,window: Single<&Window>, ) {
    let scene_data = SceneData::new(
        window.physical_size(),
        config.termocamera_size,
        config.target_projection_distance,
        config.scene_width,
        None,
        window.scale_factor()
    );

    commands.spawn((
        SceneTag,
        scene_data,
        Transform::from_xyz(0.0, scene_data.dimensions.y / 2.0, -config.target_projection_distance),
        GlobalTransform::default(),
        Name::new("SceneTag"),
    ));
}

fn update_scene(
    camera_query: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut scene_query: Query<(&GlobalTransform, &mut Transform,&mut SceneData), With<SceneTag>>,
    mut config: ResMut<ConfigState>,
    keyboard: Res<ButtonInput<KeyCode>>
) {

    configure_scene(&mut config, &keyboard);



    if let Ok(window) = window_query.single()  {
        if let Ok((camera,camera_transform)) = camera_query.single()  {

            let mut mouse_pos: Option<Vec3> = None;

            let cursor_ray = window
                .cursor_position()
                .and_then(|cursor_pos| camera.viewport_to_world(camera_transform, cursor_pos).ok());


            for (scene_transform,mut transform,  mut scene_data) in scene_query.iter_mut() {
                
                transform.translation.z = -config.target_projection_distance;
                transform.translation.y = scene_data.dimensions.y / 2.0;

                if let Some(ray) = cursor_ray {
                    let scene_position = scene_transform.translation();
                    // The scene's plane is defined by its position and its forward vector (normal).
                    let scene_plane = InfinitePlane3d::new(scene_transform.forward());

                    // Find the intersection of the ray with the scene plane.
                    if let Some(distance) = ray.intersect_plane(scene_position, scene_plane) {
                        let intersection_point = ray.get_point(distance);
                        let local_pos_3d = scene_transform.affine().inverse().transform_point(intersection_point);

                        // Check if the intersection point is within the scene bounds
                        if local_pos_3d.x.abs() <= scene_data.dimensions.x / 2.0
                            && local_pos_3d.y.abs() <= scene_data.dimensions.y / 2.0
                        {
                            mouse_pos = Some(intersection_point);
                        }
                    }

                    *scene_data = SceneData::new(
                        window.physical_size(),
                        config.termocamera_size,
                        config.target_projection_distance,
                        config.scene_width,
                        mouse_pos,
                        window.scale_factor(),
                    );


                }
            }
        }
    }
}


fn configure_scene(config: &mut ConfigState, keyboard: &Res<ButtonInput<KeyCode>>){
            
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        config.target_projection_distance = config.target_projection_distance + 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        config.target_projection_distance = config.target_projection_distance - 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        config.scene_width = config.scene_width - 1.;
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) {
        config.scene_width =  config.scene_width+1.;
    }

}