use bevy::prelude::*;
//use bevy::render::mesh::PrimitiveTopology;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
use crate::util::scale::ScaleCalculations;
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
        .add_systems(Update, update_instructions);
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
                
                if let Some(viewport) = &camera.viewport {

                    let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO); 
                    if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)  {
                
                        let fps = 1.0 / time.delta_secs();

                        let calc = ScaleCalculations::new(
                window.physical_size(),
                             config.termocamera_size, 
                             window.scale_factor()
                        ); 

                        let vp_pos = calc.get_viewport_cursor_coordinates(cursor_pos);

                        let window_txt =  format!("Window size: {}x{} scale factor {}\n", window.physical_size().x ,window.physical_size().y, window.scale_factor());
                        let window_scaled_txt =  format!("Window scaled size: {}x{}\n", calc.get_window_scaled_size().x ,calc.get_window_scaled_size().y);
                        let mouse_txt =  format!("Window cursor position: x:{:.2} y{:.2}\n", cursor_pos.x ,cursor_pos.y);
                        let world_txt =  format!("World pos: x:{:.2} y{:.2} z{:.2}\n", ray.origin.x ,ray.origin.y,ray.origin.z);
                        let termocam_size =  format!("Termo camera size: {}x{}\n", config.termocamera_size.x ,config.termocamera_size.y);
                        
                        let viewport_size =  format!("Viewport size: {}x{}\n", calc.get_viewport_size().x ,calc.get_viewport_size().y);
                        let viewport_scaled_size =  format!("Viewport scaled_size: {}x{}\n", calc.get_viewport_scaled_size().x ,calc.get_viewport_scaled_size());
                        let viewport_pos =  format!("Viewport position: {}x{}\n", calc.get_viewport_position().x ,calc.get_viewport_position().y); 
                        let viewport_scaled_pos =  format!("Viewport scaled pos: {}x{}\n", calc.get_viewport_scaled_position().x ,calc.get_viewport_scaled_position());              
                        let viewport_cursor_position =  format!("Viewport cursor position: x:{:.2} y{:.2}\n", calc.get_viewport_cursor_coordinates(cursor_pos).x ,calc.get_viewport_cursor_coordinates(cursor_pos).y);

                        let viewport_to_world_txt =  format!("Viewport position to window position: x:{:.2} y{:.2}\n", calc.translate_viewport_coordinates_to_window_coordinates(vp_pos).x, calc.translate_viewport_coordinates_to_window_coordinates(vp_pos).y);

                        let fps_txt =  format!("FPS: {:.2} \n", fps);
 
                        for mut text in text.iter_mut() {
                            text.clear();
                            text.push_str(INSTRUCTION_TEXT );  
                          
                            text.push_str(&window_txt);
                            text.push_str(&termocam_size);
                            text.push_str(&viewport_size);
                            // text.push_str(&window_scaled_txt); 
                            text.push_str(&mouse_txt);
                             
                            // text.push_str(&viewport_scaled_size);
                            //  text.push_str(&viewport_pos); 
                            //  text.push_str(&viewport_scaled_pos);  
                            text.push_str(&viewport_cursor_position);  
                            text.push_str(&world_txt);  
                            //  text.push_str(&viewport_to_world_txt);  
                            text.push_str(&fps_txt);  
                        }
                
                    }    

                } 

            }



            for mut visibility in visbility.iter_mut() {
                *visibility = if config.instructions_visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }

            if keyboard.just_pressed(KeyCode::Space) {
                config.as_mut().instructions_visible = !config.instructions_visible;
            }

        }


    };


}
