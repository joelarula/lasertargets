use bevy::prelude::*;
use common::toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem};
use crate::plugins::calibration::CalibrationSystemSet;

use crate::plugins::instructions::InstructionState;
use common::config::ProjectorConfiguration;

const BTN_NAME: &str = "projector";

#[derive(Component)]
struct ProjectorButton;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ProjectorSystemSet;

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ProjectorConfiguration::default())
            .add_systems(Startup, (
                register_projector, 
                register_projector_instructions).in_set(ProjectorSystemSet).after(CalibrationSystemSet))
            .add_systems(Update, (
                handle_projector_button,
                update_projector_system,
                sync_toolbar_with_config,
            ).chain().in_set(ProjectorSystemSet).after(CalibrationSystemSet));
    }
}

fn register_projector(mut commands: Commands) {
    commands.spawn((
        ProjectorButton,
        ToolbarItem {
            name: BTN_NAME.to_string(),
            order: 1,
            text: None,
            icon: Some("\u{f0eb}".to_string()), // Laser icon
            state: ItemState::Disabled,
            docking: Docking::Right,
            button_size: 36.0,
            ..default()
        },
    ));
}

fn register_projector_instructions(mut instructions: ResMut<InstructionState>) {
    instructions.add_instruction("Press [F1] to toggle projector".to_string());
}

fn update_toolbar_state(
    projector_config: &ProjectorConfiguration,
    item_query: &mut Query<&mut ToolbarItem, With<ProjectorButton>>,
) {
    if let Ok(mut item) = item_query.single_mut() {
        item.state = if !projector_config.connected {
            ItemState::Disabled
        } else if projector_config.switched_on { 
            ItemState::On 
        } else { 
            ItemState::Off 
        };
    }
}

fn handle_projector_button(
    button_query: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut item_query: Query<&mut ToolbarItem, With<ProjectorButton>>,
) {
    for (interaction, button) in &button_query {
        if button.name == BTN_NAME && *interaction == Interaction::Pressed {
            projector_config.switched_on = !projector_config.switched_on;
            update_toolbar_state(&projector_config, &mut item_query);
        }
    }
}

fn update_projector_system(
    mut projector_config: ResMut<ProjectorConfiguration>,
    mut item_query: Query<&mut ToolbarItem, With<ProjectorButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let prev_enabled = projector_config.switched_on;
    
    configure_projector(&mut projector_config, &keyboard);
    if prev_enabled != projector_config.switched_on {
        update_toolbar_state(&projector_config, &mut item_query);
    }
}

fn sync_toolbar_with_config(
    projector_config: Res<ProjectorConfiguration>,
    mut item_query: Query<&mut ToolbarItem, With<ProjectorButton>>,
) {
    if projector_config.is_changed() {
        update_toolbar_state(&projector_config, &mut item_query);
    }
}

fn configure_projector(
    projector_config: &mut ResMut<ProjectorConfiguration>,
    keyboard: &Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        projector_config.switched_on = !projector_config.switched_on;
    }
}