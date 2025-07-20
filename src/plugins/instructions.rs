use bevy::prelude::*;
//use bevy::render::mesh::PrimitiveTopology;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
//use crate::plugins::cursor::CursorSystemSet;
//use bevy::render::render_asset::RenderAssetUsages;
//use bevy::color::palettes::css::GREY;

#[derive(Component)]
struct InstructionText;

//#[derive(Component)]
//struct Crosshair;

pub struct InstructionsPlugin;

const INSTRUCTION_TEXT: &str = "Press [Space] to toggle instructions\n";

impl Plugin for InstructionsPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup,setup_instructions)
        //.add_systems(FixedUpdate, update_instructions.after(CursorSystemSet));
        .add_systems(FixedUpdate, update_instructions);
    }
}

fn setup_instructions(
    mut commands: Commands,
    //mut meshes: ResMut<Assets<Mesh>>, 
    //mut materials: ResMut<Assets<ColorMaterial>>,
) {
    
    commands.spawn((
        InstructionText,
        Name::new("Instructions"),
        Text::new(INSTRUCTION_TEXT), 
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

   // let size = 10.0; // Half-length of each line
   // let vertices = [
   //     // Horizontal line
   //     [ -size, 0.0, 0.0], // Left
   //     [ size, 0.0, 0.0],  // Right
   //     // Vertical line
   //     [ 0.0, -size, 0.0], // Bottom
   //     [ 0.0, size, 0.0],  // Top
   // ];
   // let indices = Indices::U32(vec![0, 1, 2, 3]); // Two lines: 0-1 (horizontal), 2-3 (vertical)

   // let mut crosshair = Mesh::new(PrimitiveTopology::LineList,RenderAssetUsages::RENDER_WORLD);
   // crosshair.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
   // crosshair.se set_indices(Some(indices));


   // commands.spawn((
   //     Crosshair,
   //     Name::new("CrossHair"),
   //     Mesh2d( meshes.add(crosshair)),
   //     MeshMaterial2d(materials.add(Color::from(GREY))),
   //     Transform::from_xyz(  0.0,  0.0,  0.0, ),
   // ));

}

fn update_instructions( 
    window: Query<&Window, With<PrimaryWindow>> ,
    // mut query: Query<&mut Transform, With<Crosshair>>, 
    mut text: Query<&mut Text, With<InstructionText>>,
    mut visbility: Query<&mut Visibility, With<InstructionText>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera>>,
    mut config: ResMut<ConfigState>) {

    if let Ok(window) = window.single()  {

        if let Ok((camera, camera_transform)) = camera.single()  {

            if config.as_mut().instructions_visible{
                
                let fps = 1.0 / time.delta_secs();
                let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO);
                let world_pos = camera
                    .viewport_to_world_2d(camera_transform, cursor_pos)
                    .unwrap_or(Vec2::ZERO);
            

                let window_size = window. resolution. physical_size(); 
                
                let window_logcial_x= window_size.x as f32 / window.scale_factor();
                
                let window_logical_y = window_size.y as f32 / window.scale_factor();
                
                let scale_factor = window.scale_factor();
                

                let viewport_pos = if let Some(viewport) = &camera.viewport {
                    viewport.physical_position.as_vec2()
                } else {
                    Vec2::ZERO
                };

                let viewport_pos_x = viewport_pos.x / scale_factor;
                let viewport_pos_y = viewport_pos.y / scale_factor;
                
                let viewport_relative_pos_x = cursor_pos.x - viewport_pos_x;
                let viewport_relative_pos_y = cursor_pos.y - viewport_pos_y;
            
                let viewport_size = if let Some(viewport) = &camera.viewport {
                    viewport.physical_size.as_vec2()
                } else {
                    window_size.as_vec2()
                };

                let viewport_x = viewport_relative_pos_x * scale_factor;
                let viewport_y = viewport_relative_pos_y * scale_factor;


                let window_txt =  format!("Window size: {}x{} scale factor {}\n", window_size.x ,window_size.y, window.scale_factor());
                let window_logical_txt =  format!("Window logical size: {}x{} {}\n", window_logcial_x ,window_logical_y, window.scale_factor());
                let viewport_txt =  format!("Viewport size: {}x{}\n", viewport_size.x ,viewport_size.y);
                let viewport_pos =  format!("Viewport pos: {}x{}\n", viewport_pos_x ,viewport_pos_y);
                let mouse_txt =  format!("Window Mouse Position: x:{:.2} y{:.2}\n", cursor_pos.x ,cursor_pos.y);
                let vp_txt =  format!("Viewport Mouse Position: x:{:.2} y{:.2}\n", viewport_x ,viewport_y);
                let world_txt =  format!("World Position: x:{:.2} y{:.2}\n", world_pos.x ,world_pos.y);
                let fps_txt =  format!("FPS: {:.2} \n", fps);
 
                for mut text in text.iter_mut() {
                    text.clear();
                    text.push_str(INSTRUCTION_TEXT );  
                    text.push_str(&fps_txt);  
                    text.push_str(&window_txt); 
                    text.push_str(&window_logical_txt); 
                    text.push_str(&viewport_txt); 
                    text.push_str(&viewport_pos);  
                    text.push_str(&mouse_txt);  
                    text.push_str(&vp_txt);  
                    text.push_str(&world_txt);  

                }

            }



            for mut visibility in visbility.iter_mut() {
                *visibility = if config.instructions_visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }

            //for mut transform in query.iter_mut() {
            //    transform.translation = Vec3::new(config.world_position.x, config.world_position.y, 1.5);
            //}



            if keyboard.just_pressed(KeyCode::Space) {
                config.as_mut().instructions_visible = !config.instructions_visible;
            }




        }


    };


}
