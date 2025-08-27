use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::plugins::camera::CameraSystemSet;
use crate::plugins::config::ConfigState;
use crate::plugins::scene::SceneData;
use crate::plugins::scene::SceneSystemSet;
use crate::plugins::scene::SceneTag;
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
//use bevy::color::palettes::css::LIGHT_GREY;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::YELLOW;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CalibrationSystemSet;

pub struct CalibrationPlugin;


impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_grid.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, update_grid.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_axes.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_billboard_gizmos.in_set(CalibrationSystemSet).after(CameraSystemSet));
    }
}

fn setup_grid(   mut commands: Commands,config: Res<ConfigState>,) {

}

fn update_grid(mut gizmos: Gizmos, config: Res<ConfigState>,) {

    gizmos.grid(
        Quat::from_rotation_x(PI / 2.),
        UVec2::new((config.scene_width * 4.) as u32, (config.target_projection_distance * 4.) as u32),
        Vec2::new(config.grid_spacing, config.grid_spacing),
        DARK_GREY
    );


}


fn draw_axes(mut gizmos: Gizmos) {
    // Draw the X-axis (Red)
    gizmos.arrow(Vec3::ZERO, Vec3::X * 5.0, RED);

    // Draw the Y-axis (Green)
    gizmos.arrow(Vec3::ZERO, Vec3::Y * 5.0, GREEN);

    // Draw the Z-axis (Blue)
    gizmos.arrow(Vec3::ZERO, Vec3::Z * 5.0, BLUE);
}

fn draw_billboard_gizmos(
    config: Res<ConfigState>,
    window: Single<&Window>, 
    mut gizmos: Gizmos,
    // Query for the 3D camera
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    // Query for your billboard entities
    scene_query: Query<(&GlobalTransform, &SceneData), With<SceneTag>>,
) {


    for(camera, camera_transform) in camera_query.iter(){
        for (scene_transform,scene_data) in scene_query.iter() {
    
            let billboard_position = scene_transform.translation();
            // Make the billboard face the camera, but only rotate on the Y axis.
            // This prevents the billboard from tilting up or down.
            let mut camera_position_flat = camera_transform.translation();
            camera_position_flat.y = billboard_position.y;
            let billboard_rotation = Transform::from_translation(billboard_position)
                .looking_at(camera_position_flat, Vec3::Y).rotation;
        
            let billboard_up = billboard_rotation.mul_vec3(Vec3::Y);
            let billboard_right = billboard_rotation.mul_vec3(Vec3::X);

            // Define the dimensions of your billboard.
            let width = scene_data.dimensions.x;
            let height = scene_data.dimensions.y;
            let grid_size = config.grid_spacing;


            // Draw the frame.
            let p1 = billboard_position - billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
            let p2 = billboard_position + billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
            let p3 = billboard_position + billboard_right * (width / 2.0) - billboard_up * (height / 2.0);
            let p4 = billboard_position - billboard_right * (width / 2.0) - billboard_up * (height / 2.0);

            gizmos.line(p1, p2, SILVER);
            gizmos.line(p2, p3, SILVER);
            gizmos.line(p3, p4, SILVER);
            gizmos.line(p4, p1,SILVER);

            // Draw the grid lines.
            let num_x_lines = (width / grid_size) as usize;
            let num_y_lines = (height / grid_size) as usize;

            // Vertical grid lines.
            for i in 0..=num_x_lines {
                let offset_x = (i as f32) * grid_size - width / 2.0;
                let start = billboard_position + billboard_right * offset_x - billboard_up * (height / 2.0);
                let end = billboard_position + billboard_right * offset_x + billboard_up * (height / 2.0);
                gizmos.line(start, end, SILVER);
            }

            // Horizontal grid lines.
            for i in 0..=num_y_lines {
                let offset_y = (i as f32) * grid_size - height / 2.0;
                let start = billboard_position + billboard_up * offset_y - billboard_right * (width / 2.0);
                let end = billboard_position + billboard_up * offset_y + billboard_right * (width / 2.0);
                gizmos.line(start, end, SILVER);
            }

            if(scene_data.mouse_world_pos.is_some()){
                let intersection_point = scene_data.mouse_world_pos.unwrap();

                // Draw a 3D crosshair at the intersection point.
                let crosshair_size = config.grid_spacing * 0.5;
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
    }


}


