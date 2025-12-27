use bevy::prelude::*;
use common::config::SceneConfiguration;
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SceneConfiguration::default());
    }
}