use bevy::prelude::*;
use crate::plugins::config::ConfigState;
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
pub struct GridPlugin;

#[derive(Component)]
struct Ground;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_grid)
        .add_systems(FixedUpdate, update_grid);
    }
}

fn setup_grid(  
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<ConfigState>,
    mut materials: ResMut<Assets<StandardMaterial>>,) {

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(config.target_projection_distance, config.target_projection_distance))),
        MeshMaterial3d(materials.add(Color::from(DARK_GREY).with_alpha(0.1))),
        Ground,
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
    ));


}

fn update_grid(  
    window: Query<&Window>, 
    mut gizmos: Gizmos, 
    ground: Single<&GlobalTransform, With<Ground>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera>>,
    config: Res<ConfigState>) {


    gizmos.grid(
        Quat::from_rotation_x(PI / 2.),
        UVec2::new(20,100),
        Vec2::new(config.grid_spacing, config.grid_spacing),
        LinearRgba::gray(0.55),
    );

        if let Ok(window) = window.single()  {

            if let Ok((camera, camera_transform)) = camera.single()  {

                let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO);
                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)  {

                    if let Some(distance) = ray.intersect_plane(ground.translation(), InfinitePlane3d::new(ground.up())){
                        
                        let point = ray.get_point(distance);
                        gizmos.circle(
                            Isometry3d::new(
                                point + ground.up() * 0.01,
                                Quat::from_rotation_arc(Vec3::Z, ground.up().as_vec3()),
                            ),
                            0.2,
                            Color::from(SILVER),
                        );

                    }
                }
            }
        }
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


