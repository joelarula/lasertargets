use bevy::prelude::UVec2;
use bevy::prelude::Vec2;

/// Utility for window viewport and mouse calculations.
#[derive(Debug, Clone, Copy)] 
pub struct ScaleCalculations{
    /// The size of the window in pixels.
    pub window_size: UVec2,
    /// scale factor of the window.
    pub scale_factor: f32,
    /// The size of the viewport in pixels.
    pub viewport_size: UVec2,
     /// The size of the thermal camera viewport in pixels.   
    pub termocamera_size: UVec2,
}

impl ScaleCalculations {

   pub fn new(window_size: UVec2, termocamera_size: UVec2,scale_factor:f32) -> Self {
      ScaleCalculations{
         window_size,
         scale_factor,
         viewport_size: Self::calculate_viewport_size(window_size, termocamera_size),
         termocamera_size
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
    

   fn calculate_viewport_size(window_size: UVec2, termocamera_size: UVec2) -> UVec2 { 
      
      let hscale = window_size.x as f32 / termocamera_size.x as f32;
      let vscale = window_size.y as f32 / termocamera_size.y as f32;
      let scale = hscale.min(vscale);   
      return  UVec2::new(
      (termocamera_size.x as f32 * scale).round() as u32, 
      (termocamera_size.y as f32 * scale).round() as u32
      );

   }

   pub fn get_scene_height(&self, scene_width: f32) -> f32 {
        let aspect_ratio = self.termocamera_size.y as f32 / self.termocamera_size.x as f32;
        scene_width * aspect_ratio
   }
}