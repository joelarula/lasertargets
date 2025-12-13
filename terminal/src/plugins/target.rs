use bevy::prelude::*;
use log::info;
use crate::plugins::toolbar::ToolbarRegistry;
use crate::plugins::instructions::InstructionState;
use crate::plugins::scene::{SceneData, SceneTag};
use crate::plugins::basictarget::{BasicTarget, BasicTargetPlugin};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TargetSystemSet;


#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool,
    start_button_id: Option<String>,
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(BasicTargetPlugin)
            .insert_resource(DragState::default())
            .add_systems(Startup, (register_target, register_target_instructions))
            .add_systems(Update, (
                handle_target_drag.in_set(TargetSystemSet),
                update_target_system.in_set(TargetSystemSet),
            ));
    }
}

fn register_target(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_icon_button(
        "Target".to_string(),
        target_callback,
        "\u{f140}".to_string(), // Target/crosshairs icon
        crate::plugins::toolbar::Docking::Bottom,
        36.0,
    );
}

fn register_target_instructions(mut instructions: ResMut<InstructionState>) {
    instructions.add_instruction("Press [T] to toggle target".to_string());
}

fn target_callback() {
    // Callback is handled by drag system
}

fn handle_target_drag(
    mut commands: Commands,
    mut drag_state: ResMut<DragState>,
    button_query: Query<(&Interaction, &crate::plugins::toolbar::DynamicButton)>,
    scene_query: Query<&SceneData, With<SceneTag>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {


    // Check for drag start from Target button
    for (interaction, button) in &button_query {
        if button.id == "Target" {
            if *interaction == Interaction::Pressed && mouse_button.pressed(MouseButton::Left) {
            info!("Target button interaction: {:?}, mouse pressed: {}", interaction, mouse_button.pressed(MouseButton::Left));
            
                if !drag_state.is_dragging {
                    drag_state.is_dragging = true;
                    drag_state.start_button_id = Some(button.id.clone());
                    info!("Started dragging from Target button");
                }
            }
        }
    }

    // Check for drag end
    if drag_state.is_dragging && mouse_button.just_released(MouseButton::Left) {
        info!("Drag ended, checking scene data...");
        if let Ok(scene_data) = scene_query.single() {
            info!("Scene data found, mouse_world_pos: {:?}", scene_data.mouse_world_pos);
            // Drag ended, spawn target at world position if mouse is over scene
            if let Some(world_pos) = scene_data.mouse_world_pos {
                spawn_target_circle(&mut commands, world_pos);
                info!("Spawned target at {:?}", world_pos);
            } else {
                info!("No mouse world position available");
            }
        } else {
            info!("Scene query failed");
        }
        drag_state.is_dragging = false;
        drag_state.start_button_id = None;
    }
}

fn spawn_target_circle(
    commands: &mut Commands,
    position: Vec3,
) {
    info!("Spawning basic target at position: {:?}", position);
    
    // Spawn a BasicTarget entity - rendering is handled by BasicTargetPlugin
    commands.spawn((
        BasicTarget {
            radius: 0.5,
            segments: 32,
            color: Color::srgb(0.0, 0.5, 1.0),
        },
        Transform::from_translation(position),
        Name::new("BasicTarget"),
    ));
}

fn update_target_system() {
    // System placeholder for future target updates
}
