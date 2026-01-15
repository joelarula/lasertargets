use bevy::prelude::*;
use common::config::CameraConfiguration;
use common::scene::SceneSetup;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraConfiguration::default());
        app.add_systems(Update, update_server_camera);
    }
}



fn update_server_camera(
    mut camera_config: ResMut<CameraConfiguration>,
    scene_setup: Res<SceneSetup>,
) {
    if camera_config.is_changed() || scene_setup.is_changed() {
        if camera_config.locked_to_scene {
            // Lock camera to scene center
            let scene_center = scene_setup.transform.translation;
            let new_rotation = Transform::from_translation(camera_config.transform.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
            // Only update if rotation actually changed
            if camera_config.transform.rotation != new_rotation {
                camera_config.transform.rotation = new_rotation;
            }
        }
    }
}

