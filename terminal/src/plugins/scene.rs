use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use common::config::CameraConfiguration;
use common::config::ProjectorConfiguration;
use common::config::SceneConfiguration;
use crate::plugins::camera::{CameraTag, ViewportMode};
use crate::plugins::instructions::DebugInfoState;
use crate::plugins::instructions::InstructionState;
use bevy::prelude::UVec2;
use bevy::prelude::Vec2;


const INSTRUCTION_TEXT_A: &str = "Press [Up][Down] to adjust target distance";
const INSTRUCTION_TEXT_B: &str = "Press [Left][Right] to adjust target width";

#[derive(Component)]
pub struct SceneTag;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneSystemSet;
pub struct ScenePlugin;


impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
       
        app.insert_resource(SceneConfiguration::default());
        app.add_systems(Startup, setup_scene.in_set(SceneSystemSet));
        app.add_systems(Update, update_scene.in_set(SceneSystemSet));  
    }
}

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
     /// The size of the camera resolution in pixels.   
    pub camera_resolution: UVec2,
    /// The size of the projection resolution in pixels.   
    pub projection_resolution: UVec2,
    /// Most recent calculated world position of the mouse cursor intersection with the scene, if any.
    pub mouse_world_pos: Option<Vec3>
}

impl SceneData {

   pub fn new(
        window_size: UVec2, 
        camera_resolution: UVec2, 
        projection_resolution: UVec2,
        distance: f32,
        scene_width: f32,
        mouse_pos: Option<Vec3>, 
        scale_factor:f32,
        viewport_mode: ViewportMode,
        ) -> Self {
      
        SceneData{
            dimensions: Self::get_scene_dimensions(scene_width,camera_resolution),
            distance,
            window_size,
            scale_factor,
            viewport_size: Self::calculate_viewport_size(window_size, camera_resolution, viewport_mode),
            camera_resolution: camera_resolution,
            mouse_world_pos: mouse_pos,
            projection_resolution: projection_resolution,
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
        let relative_pos = viewport_pos / self.scale_factor;
        self.get_viewport_scaled_position() + relative_pos
    }

    /// Version without applying scale factor (assumes viewport_pos is already in window pixels)
    pub fn translate_viewport_coordinates_to_window_coordinates_unscaled(&self, viewport_pos: Vec2) -> Vec2 {
        self.get_viewport_scaled_position() + viewport_pos
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

   fn calculate_viewport_size(window_size: UVec2, camera_input_size: UVec2, viewport_mode: ViewportMode) -> UVec2 { 
      
      let hscale = window_size.x as f32 / camera_input_size.x as f32;
      let vscale = window_size.y as f32 / camera_input_size.y as f32;
      
      let scale = match viewport_mode {
          ViewportMode::AspectFit => hscale.min(vscale),
          ViewportMode::FillWidth => hscale,
      };
      
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

   pub fn get_world_units_per_camera_input_pixel(&self) -> f32 {
      self.dimensions.x / self.camera_resolution.x as f32
   }

   pub fn get_world_units_per_viewport_pixel(&self) -> f32 {
      self.dimensions.x / self.viewport_size.x as f32
   }

   /// Calculates the projector angle (in degrees) needed to cover the scene width at the target distance.
   /// Uses the formula: angle = 2 * atan(width / (2 * distance))
   pub fn calculate_projector_angle_for_scene_width(&self) -> f32 {
      let half_width = self.dimensions.x / 2.0;
      let half_angle_rad = (half_width / self.distance).atan();
      let full_angle_rad = 2.0 * half_angle_rad;
      full_angle_rad.to_degrees()
   }
}



fn setup_scene(
    mut commands: Commands, 
    config: Res<CameraConfiguration>,
    projection_config: Res<ProjectorConfiguration>,
    scene_configuration: Res<SceneConfiguration>,
    window: Single<&Window>,
    mut instruction_state: ResMut<InstructionState>,
    viewport_mode: Res<ViewportMode>,
) {

        instruction_state.instructions.push(INSTRUCTION_TEXT_A.to_string());
        instruction_state.instructions.push(INSTRUCTION_TEXT_B.to_string());
    

        let scene_data = SceneData::new(
            window.physical_size(),
            config.resolution,
            projection_config.resolution,
            scene_configuration.target_projection_distance,
            scene_configuration.scene_width,
            None,
            window.scale_factor(),
            *viewport_mode,
        );

        commands.spawn((
            SceneTag,
            scene_data,
            Transform::from_xyz(0.0, scene_data.dimensions.y / 2.0, -scene_configuration.target_projection_distance),
            GlobalTransform::default(),
        ));
}

fn update_scene(
    camera_query: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut scene_query: Query<(&GlobalTransform, &mut Transform,&mut SceneData), With<SceneTag>>,
    mut config: ResMut<CameraConfiguration>,
    mut scene_configuration: ResMut<SceneConfiguration>,
    projection_config: Res<ProjectorConfiguration>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_info: ResMut<DebugInfoState>,
    viewport_mode: Res<ViewportMode>,
) {

    configure_scene(&mut config, &mut scene_configuration,&keyboard);

    if let Ok(window) = window_query.single()  {


        if let Ok((camera,camera_transform)) = camera_query.single()  {

            let mut mouse_pos: Option<Vec3> = None;

            let cursor_ray = window
                .cursor_position()
                .and_then(|cursor_pos| camera.viewport_to_world(camera_transform, cursor_pos).ok());


            for (scene_transform,mut transform,  mut scene_data) in scene_query.iter_mut() {
                
                transform.translation.z = -scene_configuration.target_projection_distance;
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



                }

                *scene_data = SceneData::new(
                    window.physical_size(),
                    config.resolution,
                    projection_config.resolution,    
                    scene_configuration.target_projection_distance,
                    scene_configuration.scene_width,
                    mouse_pos,
                    window.scale_factor(),
                    *viewport_mode,
                );

                update_debug_info(&mut debug_info, &window, &config, &projection_config, &scene_data);

            }
        }
    }
}


fn configure_scene(config: &mut CameraConfiguration, scene_configuration: &mut SceneConfiguration, keyboard: &Res<ButtonInput<KeyCode>>){
            
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        scene_configuration.target_projection_distance += 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        scene_configuration.target_projection_distance -= 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        scene_configuration.scene_width = scene_configuration.scene_width - 1.;
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) {
        scene_configuration.scene_width =  scene_configuration.scene_width + 1.;
    }

}

fn update_debug_info(debug_info: &mut DebugInfoState, window: &Window, config: &CameraConfiguration, projector_config: &ProjectorConfiguration, scene_data: &SceneData){
        
        let window_txt =  format!("Window size: {}x{} Camera input size: {}x{} Viewport size: {}x{} scale factor {}", 
            window.physical_size().x ,
            window.physical_size().y, 
            config.resolution.x,
            config.resolution.y,
            scene_data.get_viewport_size().x,
            scene_data.get_viewport_size().y,
            window.scale_factor());

        debug_info.messages.push(window_txt);

        let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO); 
        let mouse_txt =  format!("Window cursor pos: x:{:.2} y{:.2} Viewport cursor pos: x:{:.2} y{:.2}", 
            cursor_pos.x ,
            cursor_pos.y,
            scene_data.get_viewport_cursor_coordinates(cursor_pos).x,
            scene_data.get_viewport_cursor_coordinates(cursor_pos).y
        );

        debug_info.messages.push(mouse_txt);

        let target_txt =  format!("Target distance {:.2} m , width {:.2}, height {:.2}", scene_data.distance, scene_data.dimensions.x,scene_data.dimensions.y);
        debug_info.messages.push(target_txt);

        let ratio_txt =  format!("Input camera pixels  to world:{:.2} Viewport pixels to world{:.2}", 
            scene_data.get_world_units_per_camera_input_pixel(), scene_data.get_world_units_per_viewport_pixel());
        
        debug_info.messages.push(ratio_txt);

        // Calculate projector pixel to world ratio
        let angle_rad = projector_config.angle.to_radians();
        let half_angle = angle_rad / 2.0;
        let projected_width = 2.0 * scene_data.distance * half_angle.tan();
        let projector_pixel_to_world = projected_width / projector_config.resolution.x as f32;
        
        let projector_txt = format!("Projector angle: {:.2}° Resolution: {}x{} Pixel to world: {:.4}", 
            projector_config.angle,
            projector_config.resolution.x,
            projector_config.resolution.y,
            projector_pixel_to_world);
        
        debug_info.messages.push(projector_txt);

        if(scene_data.mouse_world_pos.is_some()){
            let raypos = scene_data.mouse_world_pos.unwrap();
            let world_txt =  format!("Scene cursor pos: x:{:.2} y{:.2} z{:.2}", raypos.x ,raypos.y,raypos.z);
            debug_info.messages.push(world_txt);
        }

       // let fps = 1.0 / time.delta_secs();
       // let fps_txt =  format!("Time {:.2} FPS: {:.2} ", time.elapsed_secs(), fps);
       // debug_info.messages.push(fps_txt);     

    
}

