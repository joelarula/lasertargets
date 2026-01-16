use bevy::prelude::*;
use common::scene::{SceneSetup, SceneSystemSet};

pub struct ScenePlugin;

/// Marker component for the scene entity
#[derive(Component)]
pub struct SceneEntity;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_scene_entity.in_set(SceneSystemSet))
            .add_systems(Update, sync_scene_transform.in_set(SceneSystemSet));
    }
}

fn setup_scene_entity(
    mut commands: Commands,
    scene_setup: Res<SceneSetup>,
) {
    // Spawn the scene entity with its initial transform
    commands.spawn((
        SceneEntity,
        Transform::from_translation(scene_setup.scene.origin.translation)
            .with_rotation(scene_setup.scene.origin.rotation)
            .with_scale(scene_setup.scene.origin.scale),
        Visibility::default(),
    ));
}

fn sync_scene_transform(
    scene_setup: Res<SceneSetup>,
    mut scene_query: Query<&mut Transform, With<SceneEntity>>,
) {
    if scene_setup.is_changed() {
        if let Ok(mut transform) = scene_query.single_mut() {
            // Update the scene entity's transform to match SceneSetup
            transform.translation = scene_setup.scene.origin.translation;
            transform.rotation = scene_setup.scene.origin.rotation;
            transform.scale = scene_setup.scene.origin.scale;
        }
    }
}