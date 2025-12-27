use bevy::prelude::*;
use common::config::ProjectorConfiguration;

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectorConfiguration::default());
    }
}