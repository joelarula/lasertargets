use bevy::prelude::*;
use common::config::CameraConfiguration;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraConfiguration::default());
    }
}