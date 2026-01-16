use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use common::config::SceneConfiguration;
use common::scene::{SceneSystemSet, SceneSetup};
use crate::plugins::camera::{CameraTag};
use crate::plugins::instructions::InstructionState;
use bevy::prelude::Vec2;
use bevy_quinnet::client::QuinnetClient;
use common::network::NetworkMessage;


const INSTRUCTION_TEXT_A: &str = "Press [Up][Down] to adjust target distance";


pub struct ScenePlugin;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PreviousMouseWorldPos(pub Option<Vec3>);

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
       
        app.insert_resource(SceneConfiguration::default());
        app.init_resource::<SceneSetup>();
        app.init_resource::<SceneData>();
        app.init_resource::<PreviousMouseWorldPos>();
        app.add_systems(Startup, setup_scene.in_set(SceneSystemSet));
        app.add_systems(Update, update_scene.in_set(SceneSystemSet));  
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)] 
pub struct SceneData{
    pub mouse_world_pos: Option<Vec3>
}

impl SceneData {
    pub fn new(mouse_pos: Option<Vec3>) -> Self {
        SceneData { mouse_world_pos: mouse_pos }
    }
}



fn setup_scene( mut instruction_state: ResMut<InstructionState>) {
    instruction_state.instructions.push(INSTRUCTION_TEXT_A.to_string());    
}

fn update_scene(
    mut client: ResMut<QuinnetClient>,
    camera_query: Query<(&Camera, &GlobalTransform), With<CameraTag>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    scene_setup: Res<SceneSetup>,
    mut scene_data: ResMut<SceneData>,
    mut previous_mouse_world_pos: ResMut<PreviousMouseWorldPos>,
) {
    if let Ok(window) = window_query.single() {
        if let Ok((camera, camera_transform)) = camera_query.single() {
            let mut mouse_pos: Option<Vec3> = None;

            let cursor_ray = window
                .cursor_position()
                .and_then(|cursor_pos| camera.viewport_to_world(camera_transform, cursor_pos).ok());

            let scene_transform: GlobalTransform = Transform::from_translation(scene_setup.scene.origin.translation)
                .with_rotation(scene_setup.scene.origin.rotation)
                .with_scale(scene_setup.scene.origin.scale)
                .into(); // Convert to GlobalTransform

            let scene_dimensions = Vec2::new(scene_setup.scene.scene_dimension.x as f32, scene_setup.scene.scene_dimension.y as f32);

            if let Some(ray) = cursor_ray {
                let scene_position = scene_transform.translation();
                let scene_plane = InfinitePlane3d::new(scene_transform.forward());

                if let Some(distance) = ray.intersect_plane(scene_position, scene_plane) {
                    let intersection_point = ray.get_point(distance);
                    let local_pos_3d = scene_transform.affine().inverse().transform_point(intersection_point);

                    if local_pos_3d.x.abs() <= scene_dimensions.x / 2.0
                        && local_pos_3d.y.abs() <= scene_dimensions.y / 2.0
                    {
                        mouse_pos = Some(intersection_point);
                    }
                }
            }

            scene_data.mouse_world_pos = mouse_pos;

            if previous_mouse_world_pos.0 != mouse_pos {
                if let Some(connection) = client.get_connection_mut() {
                    let payload = NetworkMessage::UpdateMousePosition(mouse_pos)
                        .to_bytes()
                        .expect("Failed to serialize mouse position");
                    if let Err(e) = connection.send_payload(payload) {
                        error!("Failed to send mouse position update: {}", e);
                    }
                }
                previous_mouse_world_pos.0 = mouse_pos;
            }
        }
    }
}
