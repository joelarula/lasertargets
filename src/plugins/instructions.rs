use bevy::prelude::*;
use crate::plugins::config::ConfigState;
use crate::plugins::scene::SceneSystemSet;

#[derive(Component)]
struct InstructionTag;

pub struct InstructionsPlugin;

const INSTRUCTION_F1: &str = "Press [F1] to toggle instructions";


#[derive(Resource, Default)]
pub struct InstructionState {
    pub instructions: Vec<String>,
}

#[derive(Resource, Default)]
pub struct DebugInfoState {
    pub messages: Vec<String>,
}

impl Plugin for InstructionsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InstructionState::default());
        app.insert_resource(DebugInfoState::default());
        app.add_systems(PreUpdate, clear_debug_state);
        app.add_systems(PostStartup, setup_instructions.in_set(SceneSystemSet));
        app.add_systems(PostUpdate, update_instructions.in_set(SceneSystemSet));
    }
}

fn clear_debug_state(mut debug_state: ResMut<DebugInfoState>) {
    debug_state.messages.clear();
}

fn setup_instructions(
    mut commands: Commands,
    mut instruction_state: ResMut<InstructionState>,
) {
    
    instruction_state.instructions.insert(0, INSTRUCTION_F1.to_string());

    commands.spawn((
        InstructionTag,
        Name::new("Instructions"),
        Text::new(instruction_state.instructions.join("\n")), 
        TextFont {
            font_size: 14.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(100.0),
            ..default()
        },
    ));


}

fn update_instructions( 
    mut text: Query<&mut Text, With<InstructionTag>>,
    mut visbility: Query<&mut Visibility, With<InstructionTag>>,
    debug_info: Res<DebugInfoState>,
    instruction_state: Res<InstructionState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<ConfigState>) {

    if keyboard.just_pressed(KeyCode::F1) {
        config.as_mut().instructions_visible = !config.instructions_visible;
    }

    for mut visibility in visbility.iter_mut() {
        *visibility = if config.instructions_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    
    if !config.instructions_visible {
        return;
    }


    for mut text in text.iter_mut() {
        text.clear();
        text.push_str(instruction_state.instructions.join("\n").as_str());
        text.push_str("\n\n");
        text.push_str(debug_info.messages.join("\n").as_str());
    }


}
