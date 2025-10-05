use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
use crate::plugins::camera::CameraTag;
use crate::plugins::instructions::DebugInfoState;
use crate::plugins::instructions::InstructionState;
use crate::plugins::toolbar::ToolbarRegistry;
use bevy::prelude::UVec2;
use bevy::prelude::Vec2;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use std::sync::Mutex;

const INSTRUCTION_TEXT_A: &str = "Press [Up][Down] to adjust target distance";
const INSTRUCTION_TEXT_B: &str = "Press [Left][Right] to adjust target width";

#[derive(Component)]
pub struct SceneTag;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SceneSystemSet;
pub struct ScenePlugin;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default)]
pub enum DisplayMode{
    #[default]
    Mode2D,
    Mode3D
}

/// Stores global configuration state for the application.
#[derive(Resource)]
pub struct SceneConfiguration {
    // Defines the distance of a target detection plane in modeled physical world in meters.
    pub target_projection_distance: f32,
}

impl Default for SceneConfiguration {
    fn default() -> Self {
        Self {
            target_projection_distance: 25.0, // Default distance of 25 meters
        }
    }
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
       
        app.insert_resource(SceneConfiguration::default());
        app.add_systems(Startup, setup_scene.in_set(SceneSystemSet));
        app.add_systems(Update, update_scene.in_set(SceneSystemSet));
        app.add_systems(EguiPrimaryContextPass, overlay_ui_system);
        app.add_systems(Update, overlay_trigger_system);
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

   pub fn get_world_units_per_camera_input_pixel(&self) -> f32 {
      self.dimensions.x / self.camera_input_size.x as f32
   }

   pub fn get_world_units_per_viewport_pixel(&self) -> f32 {
      self.dimensions.x / self.viewport_size.x as f32
   }
}

#[derive(Resource, Default)]
pub struct OverlayVisible(pub bool);
static OVERLAY_TRIGGER: Mutex<bool> = Mutex::new(false);

fn setup_scene(
    mut commands: Commands, 
    mut toolbar_registry: ResMut<ToolbarRegistry>,
    config: Res<ConfigState>,
    scene_configuration: Res<SceneConfiguration>,
    window: Single<&Window>,
    mut instruction_state: ResMut<InstructionState>,
) {

        instruction_state.instructions.push(INSTRUCTION_TEXT_A.to_string());
        instruction_state.instructions.push(INSTRUCTION_TEXT_B.to_string());
        toolbar_registry.register_icon_button("Target".to_string(), target_button_callback, "\u{f04fe}".to_string());
     

        let scene_data = SceneData::new(
            window.physical_size(),
            config.camera_input_size,
            scene_configuration.target_projection_distance,
            config.scene_width,
            None,
            window.scale_factor()
        );

        commands.spawn((
            SceneTag,
            scene_data,
            Transform::from_xyz(0.0, scene_data.dimensions.y / 2.0, -scene_configuration.target_projection_distance),
            GlobalTransform::default(),
            Name::new("SceneTag"),
        ));
}

fn update_scene(
    camera_query: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut scene_query: Query<(&GlobalTransform, &mut Transform,&mut SceneData), With<SceneTag>>,
    mut config: ResMut<ConfigState>,
    mut scene_configuration: ResMut<SceneConfiguration>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut debug_info: ResMut<DebugInfoState>,
   // time: Res<Time>,
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
                    config.camera_input_size,
                    scene_configuration.target_projection_distance,
                    config.scene_width,
                    mouse_pos,
                    window.scale_factor(),
                );

                update_debug_info(&mut debug_info, &window, &config, &scene_data);

            }
        }
    }
}


fn configure_scene(config: &mut ConfigState, scene_configuration: &mut SceneConfiguration, keyboard: &Res<ButtonInput<KeyCode>>){
            
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        scene_configuration.target_projection_distance += 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        scene_configuration.target_projection_distance -= 1.0;
    }

    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        config.scene_width = config.scene_width - 1.;
    }

    if keyboard.just_pressed(KeyCode::ArrowRight) {
        config.scene_width =  config.scene_width+1.;
    }

}

fn update_debug_info(debug_info: &mut DebugInfoState, window: &Window, config: &ConfigState ,scene_data: &SceneData){
        
        let window_txt =  format!("Window size: {}x{} Camera input size: {}x{} Viewport size: {}x{} scale factor {}", 
            window.physical_size().x ,
            window.physical_size().y, 
            config.camera_input_size.x,
            config.camera_input_size.y,
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

        if(scene_data.mouse_world_pos.is_some()){
            let raypos = scene_data.mouse_world_pos.unwrap();
            let world_txt =  format!("Scene cursor pos: x:{:.2} y{:.2} z{:.2}", raypos.x ,raypos.y,raypos.z);
            debug_info.messages.push(world_txt);
        }

       // let fps = 1.0 / time.delta_secs();
       // let fps_txt =  format!("Time {:.2} FPS: {:.2} ", time.elapsed_secs(), fps);
       // debug_info.messages.push(fps_txt);     

    
}


fn target_button_callback() {
    info!("Target button pressed from toolbar!");
    if let Ok(mut flag) = OVERLAY_TRIGGER.lock() {
        *flag = true;
    }
}

pub fn overlay_ui_system(
    mut egui_context: EguiContexts,
    mut overlay_visible: ResMut<OverlayVisible>,
) {
    if let Ok(ctx) = egui_context.ctx_mut() {
        egui::Window::new("Main UI").show(ctx, |ui| {
            if ui.button("Open Overlay").clicked() {
                overlay_visible.0 = true;
            }
        });

        if overlay_visible.0 {
            egui::Window::new("Small Overlay")
                .collapsible(false)
                .resizable(false)
                .fixed_size([100.0, 100.0])
                .show(ctx, |ui| {
                    ui.label("This is the overlay!");
                    if ui.button("Close").clicked() {
                        overlay_visible.0 = false;
                    }
                });
        }
    }
}

pub fn overlay_trigger_system(mut overlay_visible: ResMut<OverlayVisible>) {
    if let Ok(mut flag) = OVERLAY_TRIGGER.lock() {
        if *flag {
            overlay_visible.0 = true;
            *flag = false;
        }
    }
}
