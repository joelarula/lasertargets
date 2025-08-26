use bevy::prelude::*;
//use bevy::render::mesh::PrimitiveTopology;
use bevy::window::PrimaryWindow;
use crate::plugins::config::ConfigState;
use crate::plugins::config::DisplayMode;
use crate::util::scale::ScaleCalculations;
//use crate::plugins::cursor::CursorSystemSet;
//use bevy::render::render_asset::RenderAssetUsages;
//use bevy::color::palettes::css::GREY;

#[derive(Component)]
struct InstructionText;

//#[derive(Component)]
//struct Crosshair;

pub struct InstructionsPlugin;

const INSTRUCTION_F1: &str = "Press [F1] to toggle instructions\n";
const INSTRUCTION_F2: &str = "Press [F2] to toggle between 2d and 3d display mode\n";

const INSTRUCTION_TEXT_B: &str = "Press [Up][Down] to adjust target distance\n";
const INSTRUCTION_TEXT_C: &str = "Press [Left][Right] to adjust target width\n";

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
        Text::new(""), 
        TextFont {
            font_size: 10.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));



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

                        //let vp_pos = calc.get_viewport_cursor_coordinates(cursor_pos);

                        let window_txt =  format!("Window size: {}x{} scale factor {}\n", window.physical_size().x ,window.physical_size().y, window.scale_factor());
                        //let window_scaled_txt =  format!("Window scaled size: {}x{}\n", calc.get_window_scaled_size().x ,calc.get_window_scaled_size().y);
                        let mouse_txt =  format!("Window cursor position: x:{:.2} y{:.2}\n", cursor_pos.x ,cursor_pos.y);
                        let world_txt =  format!("World pos: x:{:.2} y{:.2} z{:.2}\n", ray.origin.x ,ray.origin.y,ray.origin.z);
                        let termocam_size =  format!("Termo camera size: {}x{}\n", config.termocamera_size.x ,config.termocamera_size.y);
                        
                        let viewport_size =  format!("Viewport size: {}x{}\n", calc.get_viewport_size().x ,calc.get_viewport_size().y);
                        //let viewport_scaled_size =  format!("Viewport scaled_size: {}x{}\n", calc.get_viewport_scaled_size().x ,calc.get_viewport_scaled_size());
                        //let viewport_pos =  format!("Viewport position: {}x{}\n", calc.get_viewport_position().x ,calc.get_viewport_position().y); 
                        //let viewport_scaled_pos =  format!("Viewport scaled pos: {}x{}\n", calc.get_viewport_scaled_position().x ,calc.get_viewport_scaled_position());              
                        let viewport_cursor_position =  format!("Viewport cursor position: x:{:.2} y{:.2}\n", calc.get_viewport_cursor_coordinates(cursor_pos).x ,calc.get_viewport_cursor_coordinates(cursor_pos).y);

                        //let viewport_to_world_txt =  format!("Viewport position to window position: x:{:.2} y{:.2}\n", calc.translate_viewport_coordinates_to_window_coordinates(vp_pos).x, calc.translate_viewport_coordinates_to_window_coordinates(vp_pos).y);

                        let fps_txt =  format!("Time {:.2} FPS: {:.2} \n", time.elapsed_secs(), fps);
                        let target_txt =  format!("Target distance {:.2} m , width {:.2}, height {:.2} \n", config.target_projection_distance,config.scene_width, calc.get_scene_height(config.scene_width));
 
                        for mut text in text.iter_mut() {
                            text.clear();
                            text.push_str(INSTRUCTION_F1);  
                            text.push_str(INSTRUCTION_F2);  
                            text.push_str(INSTRUCTION_TEXT_B);  
                            text.push_str(INSTRUCTION_TEXT_C);
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
                            text.push_str(&target_txt);   
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

            if keyboard.just_pressed(KeyCode::F1) {
                config.as_mut().instructions_visible = !config.instructions_visible;
            }





        }


    };


}
