use bevy::prelude::*;
use crate::plugins::camera::CameraSystemSet;
use crate::plugins::camera::DisplayMode;
use crate::plugins::scene::SceneData;
use crate::plugins::scene::SceneTag;
use common::config::{SceneConfiguration, ProjectorConfiguration};
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::YELLOW;
use bevy::color::palettes::css::ORANGE;

pub const DARK_GREY_THIRD: Srgba = Srgba::new(0.663, 0.663, 0.663, 0.3);

pub const GRID_SPACING: f32 = 0.25;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CalibrationSystemSet;


pub struct CalibrationPlugin;


impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Update, update_grid.in_set(CalibrationSystemSet).after(CameraSystemSet))
        //.add_systems(Update, draw_axes.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_billboard_gizmos.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_projector_billboard.in_set(CalibrationSystemSet).after(CameraSystemSet));
    }
}


fn update_grid(mut gizmos: Gizmos, scene_configuration: Res<SceneConfiguration>, display_mode: Res<DisplayMode>) {
    
    if *display_mode == DisplayMode::Mode3D {
        gizmos.grid(
            Quat::from_rotation_x(PI / 2.),
            UVec2::new((scene_configuration.scene_width * 4.) as u32, (scene_configuration.target_projection_distance * 4.) as u32),
            Vec2::new(GRID_SPACING, GRID_SPACING),
            DARK_GREY
        );  
    }
}


fn draw_axes(
    mut gizmos: Gizmos,
    scene_query: Query<&GlobalTransform, With<SceneTag>>,
) {
    // Draw axes at the scene center, not world origin
    for scene_transform in scene_query.iter() {
        let center = scene_transform.translation();
        
        // Draw the X-axis (Red)
        gizmos.arrow(center, center + Vec3::X * 5.0, RED);

        // Draw the Y-axis (Green)
        gizmos.arrow(center, center + Vec3::Y * 5.0, GREEN);

        // Draw the Z-axis (Blue)
        gizmos.arrow(center, center + Vec3::Z * 5.0, BLUE);
    }
}

fn draw_billboard_gizmos(
    mut gizmos: Gizmos,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    scene_query: Query<(&GlobalTransform, &SceneData), With<SceneTag>>,
) {
    for(camera, camera_transform) in camera_query.iter(){
        for (scene_transform, scene_data) in scene_query.iter() {
            let billboard_position = scene_transform.translation();
            let width = scene_data.dimensions.x;
            let height = scene_data.dimensions.y;
            
            draw_billboard_grid(
                &mut gizmos,
                camera_transform,
                billboard_position,
                width,
                height,
                SILVER,
                DARK_GREY_THIRD,
                GRID_SPACING,
            );
            
            // Calculate billboard orientation for crosshair
            let mut camera_position_flat = camera_transform.translation();
            camera_position_flat.y = billboard_position.y;
            let billboard_rotation = Transform::from_translation(billboard_position)
                .looking_at(camera_position_flat, Vec3::Y).rotation;
            let billboard_up = billboard_rotation.mul_vec3(Vec3::Y);
            let billboard_right = billboard_rotation.mul_vec3(Vec3::X);
            
            draw_crosshair(&mut gizmos, &scene_data, &billboard_right, &billboard_up);
        }
    }
}


fn draw_crosshair(
    gizmos: &mut Gizmos,
    scene_data: &SceneData,
    billboard_right: &Vec3,
    billboard_up: &Vec3,
) {
      if(scene_data.mouse_world_pos.is_some()){
                let intersection_point = scene_data.mouse_world_pos.unwrap();

        // Draw a 3D crosshair at the intersection point.
        let crosshair_size = GRID_SPACING * 0.5;
        gizmos.line(
            intersection_point - billboard_right * crosshair_size,
            intersection_point + billboard_right * crosshair_size,
            YELLOW,
        );

        gizmos.line(
            intersection_point - billboard_up * crosshair_size,
            intersection_point + billboard_up * crosshair_size,
            YELLOW,
        );
    }
}

fn draw_projector_billboard(
    mut gizmos: Gizmos,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    scene_query: Query<(&GlobalTransform, &SceneData), With<SceneTag>>,
    projector_config: Res<ProjectorConfiguration>,
    scene_config: Res<SceneConfiguration>,
) {
    for (camera, camera_transform) in camera_query.iter() {
        for (scene_transform, scene_data) in scene_query.iter() {
            let billboard_position = scene_transform.translation();
            
            // Calculate projected size based on angle and distance
            let angle_rad = projector_config.angle.to_radians();
            let distance = scene_config.target_projection_distance;
            let half_angle = angle_rad / 2.0;
            let projected_size = 2.0 * distance * half_angle.tan();
            
            // Square dimensions for projector
            let width = projected_size;
            let height = projected_size;
            
            let orange_alpha = Srgba::new(1.0, 0.647, 0.0, 0.3);
            
            draw_billboard_grid(
                &mut gizmos,
                camera_transform,
                billboard_position,
                width,
                height,
                ORANGE,
                orange_alpha,
                GRID_SPACING * 2.0,
            );
        }
    }
}

fn draw_billboard_grid(
    gizmos: &mut Gizmos,
    camera_transform: &GlobalTransform,
    billboard_position: Vec3,
    width: f32,
    height: f32,
    frame_color: impl Into<LinearRgba>,
    grid_color: impl Into<LinearRgba>,
    grid_size: f32,
) {
    use bevy::prelude::Color;

    // Convert colors to bevy::prelude::Color
    let frame_color: Color = Color::from(frame_color.into());
    let grid_color: Color = Color::from(grid_color.into());

    // Make billboard face the camera (Y-axis locked)
    let mut camera_position_flat = camera_transform.translation();
    camera_position_flat.y = billboard_position.y;
    let billboard_rotation = Transform::from_translation(billboard_position)
        .looking_at(camera_position_flat, Vec3::Y).rotation;
    
    let billboard_up = billboard_rotation.mul_vec3(Vec3::Y);
    let billboard_right = billboard_rotation.mul_vec3(Vec3::X);
    
    // Draw the frame
    let p1 = billboard_position - billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
    let p2 = billboard_position + billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
    let p3 = billboard_position + billboard_right * (width / 2.0) - billboard_up * (height / 2.0);
    let p4 = billboard_position - billboard_right * (width / 2.0) - billboard_up * (height / 2.0);
    
    gizmos.line(p1, p2, frame_color);
    gizmos.line(p2, p3, frame_color);
    gizmos.line(p3, p4, frame_color);
    gizmos.line(p4, p1, frame_color);
    
    // Draw grid lines
    let num_x_lines = (width / grid_size) as usize;
    let num_y_lines = (height / grid_size) as usize;
    
    // Vertical grid lines
    for i in 0..=num_x_lines {
        let offset_x = (i as f32) * grid_size - width / 2.0;
        let start = billboard_position + billboard_right * offset_x - billboard_up * (height / 2.0);
        let end = billboard_position + billboard_right * offset_x + billboard_up * (height / 2.0);
        gizmos.line(start, end, grid_color);
    }
    
    // Horizontal grid lines
    for i in 0..=num_y_lines {
        let offset_y = (i as f32) * grid_size - height / 2.0;
        let start = billboard_position + billboard_up * offset_y - billboard_right * (width / 2.0);
        let end = billboard_position + billboard_up * offset_y + billboard_right * (width / 2.0);
        gizmos.line(start, end, grid_color);
    }
}
