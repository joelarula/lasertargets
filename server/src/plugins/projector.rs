use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneSetup;

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectorConfiguration::default());
        app.add_systems(Update, update_projector);
    }
}



fn update_projector(
    mut projector_config: ResMut<ProjectorConfiguration>,
    scene_setup: Res<SceneSetup>,
) {
    if projector_config.is_changed() || scene_setup.is_changed() {
        if projector_config.locked_to_scene {
            // Lock projector to scene center
            let scene_center = scene_setup.transform.translation;
            let new_rotation = Transform::from_translation(projector_config.transform.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
            // Only update if rotation actually changed
            if projector_config.transform.rotation != new_rotation {
                projector_config.transform.rotation = new_rotation;
            }
        }
    }
}