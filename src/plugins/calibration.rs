use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
use crate::plugins::scene::SceneData;
use crate::plugins::scene::SceneSystemSet;
use crate::plugins::scene::SceneTag;
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::LIGHT_GREY;
use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::BLUE;
use bevy::color::palettes::css::YELLOW;

use crate::util::scale::ScaleCalculations;

pub struct CalibrationPlugin;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CalibrationSystemSet;


#[derive(Component)]
pub struct Billboard;

impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_grid)
        .add_systems(Update, update_grid.in_set(CalibrationSystemSet))
        .add_systems(Update, draw_axes)
        .add_systems(Update, draw_billboard_gizmos.after(SceneSystemSet));
    }
}

fn setup_grid(  

    //window: Query<&Window, With<PrimaryWindow>> ,
    mut commands: Commands,
    //mut meshes: ResMut<Assets<Mesh>>,
    config: Res<ConfigState>,
    //mut materials: ResMut<Assets<StandardMaterial>>,
    ) {


       // if let Ok(window) = window.single()  {
       //     
       //     let calc = ScaleCalculations::new(
      //          window.physical_size(),config.termocamera_size,window.scale_factor()
      //      ); 
      //
      //      commands.spawn((
      //              Mesh3d(meshes.add(Plane3d::default().mesh().size(config.scene_width,calc.get_scene_height(config.scene_width) ))),
      //              MeshMaterial3d(materials.add(
      //       StandardMaterial{
      //                  base_color: Color::from(LIGHT_GREY).with_alpha(1.),
      //                  alpha_mode: AlphaMode::Blend,
      //                  ..default()
      //          })),
      //          
      //          Transform::from_xyz(0.0, 0.0, 0.)     
      //              .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),Screen,
      //      ));
      //
      //  }



   // commands.spawn((
  //      Mesh3d(meshes.add(Plane3d::default().mesh().size( config.scene_width as f32,config.target_projection_distance))),
   //     MeshMaterial3d(materials.add(
   //         StandardMaterial{
   //             base_color: Color::from(DARK_GREY).with_alpha(0.1),
   //             alpha_mode: AlphaMode::Blend,
   //             ..default()
   //     })),
   //     Ground,
   // ));

    commands.spawn((
        Billboard,
        // Position the billboard in front of the camera
        Transform::from_xyz(0.0, 2.5, -config.target_projection_distance),
        GlobalTransform::default(),
        Name::new("Billboard"),
    ));


}

fn update_grid(  
  //  window: Query<&Window>, 
    mut gizmos: Gizmos, 
    //ground: Single<&GlobalTransform, With<Ground>>,
    mut billboard_query: Query<&mut Transform, With<Billboard>>,
   // camera: Query<(&Camera, &GlobalTransform), With<Camera>>,
    config: Res<ConfigState>,
    //keyboard: Res<ButtonInput<KeyCode>>
) {


    

    //for mut transform in billboard_query.iter_mut() {
    //    transform.translation.z = -config.target_projection_distance;
    //}

    gizmos.grid(
        Quat::from_rotation_x(PI / 2.),
        UVec2::new((config.scene_width * 4.) as u32, (config.target_projection_distance * 4.) as u32),
        Vec2::new(config.grid_spacing, config.grid_spacing),
        LIGHT_GREY
    );

//    if let Ok(window) = window.single()  {
//
//            if let Ok((camera, camera_transform)) = camera.single()  {
//
//                let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO);
//                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)  {
//
//                    if let Some(distance) = ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up())){
//                        
//                        let point = ray.get_point(distance);
//                        gizmos.circle(
//                            Isometry3d::new(
//                                point + ground.up() * 0.01,
//                                Quat::from_rotation_arc(Vec3::Z, ground.up().as_vec3()),
//                            ),
//                            0.2,
//                            Color::from(SILVER),
//                        );
//
    //               }
    //            }
    //        }
    //    }


    }


 //   let Ok(window) = window.single() else {
 //       return;
 //   };

 //   let window_size = window.resolution.physical_size(); 
 //  let x_cells = (window_size.x as f32 / config.grid_spacing ).round() as u32;
 //   let y_cells = (window_size.y as f32 / config.grid_spacing ).round() as u32;

   // gizmos
   //     .grid_2d(
   //         Isometry2d::IDENTITY,
   //         UVec2::new( x_cells, y_cells),
   //         Vec2::new(config.grid_spacing , config.grid_spacing),
   //         LIGHT_GRAY,
   //     )
   //     .outer_edges();


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
    billboard_query: Query<&GlobalTransform, With<Billboard>>,
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


