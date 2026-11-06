//! The projector button: switches the laser on and off, greyed out while no projector is
//! connected. [F1] does the same on the server.

use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use terminal_ui::{Docking, ItemState, ToolbarItem, ToolbarPressed};

use super::InstructionsAppExt;

#[derive(Component)]
struct ProjectorButton;

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_instructions(&["Press [F1] to toggle projector"])
            .add_systems(Startup, spawn_button)
            .add_systems(Update, show_projector.run_if(resource_changed::<ProjectorConfiguration>));
    }
}

fn spawn_button(mut commands: Commands) {
    commands
        .spawn((
            ProjectorButton,
            ToolbarItem::new("projector").icon("\u{f0eb}").order(1).dock(Docking::Right).state(ItemState::Disabled),
        ))
        .observe(|_: On<ToolbarPressed>, mut projector: ResMut<ProjectorConfiguration>| {
            projector.switched_on = !projector.switched_on;
        });
}

fn show_projector(projector: Res<ProjectorConfiguration>, mut button: Single<&mut ToolbarItem, With<ProjectorButton>>) {
    button.state = match (projector.connected, projector.switched_on) {
        (false, _) => ItemState::Disabled,
        (true, on) => if on { ItemState::On } else { ItemState::Off },
    };
}
