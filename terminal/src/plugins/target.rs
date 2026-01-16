use bevy::prelude::*;
use log::info;
use crate::plugins::toolbar::{ToolabarButton, ToolbarRegistry, ToolbarItem, Docking};
use crate::plugins::scene::SceneData;
use crate::plugins::basictarget::BasicTarget;
use common::path::PathRenderable;
use common::config::SceneConfiguration;

const BTN_NAME: &str = "target";

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct TargetSystemSet;


#[derive(Resource, Default)]
struct DragState {
    is_dragging: bool
}

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DragState::default())
            .add_systems(Startup, register_target)
            .add_systems(Update, (
                handle_target_drag.in_set(TargetSystemSet),
                update_target_system.in_set(TargetSystemSet),
            ));
    }
}

fn register_target(mut toolbar: ResMut<ToolbarRegistry>) {
    toolbar.register_button(ToolbarItem {
        name: BTN_NAME.to_string(),
        label: "Target".to_string(),
        icon: Some("\u{f140}".to_string()), // Target/crosshairs icon
        is_active: false,
        docking: Docking::Bottom,
        button_size: 36.0,
    });
}



fn handle_target_drag(
    mut commands: Commands,
    mut drag_state: ResMut<DragState>,
    button_query: Query<( &Interaction, &ToolabarButton)>,
    scene_data: Res<SceneData>,
    scene_config: Res<SceneConfiguration>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {

    // Check for drag start from Target button
    for (interaction, button) in &button_query {
        if button.name == BTN_NAME {
            if *interaction == Interaction::Pressed && mouse_button.pressed(MouseButton::Left) {
                info!("Target button interaction: {:?}, mouse pressed: {}", interaction, mouse_button.pressed(MouseButton::Left));
            
                if !drag_state.is_dragging {
                    drag_state.is_dragging = true;
                    info!("Started dragging from Target button: {}", button.name);
                }
            }
        }
    }

    // Check for drag end
    if drag_state.is_dragging && mouse_button.just_released(MouseButton::Left) {
        info!("Drag ended, checking scene data...");
        info!("Scene data found, mouse_world_pos: {:?}", scene_data.mouse_world_pos);
        
        // Drag ended, spawn target at world position if mouse is over scene
        if let Some(world_pos) = scene_data.mouse_world_pos {
            spawn_target_circle(&mut commands, world_pos, &scene_config);
            info!("Spawned target at world position {:?}", world_pos);
        } else {
            info!("No mouse world position available");
        }
        drag_state.is_dragging = false;
    }
}

fn spawn_target_circle(
    commands: &mut Commands,
    world_position: Vec3,
    scene_config: &SceneConfiguration,
) {
    // Adjust target position to match scene z coordinate
    let target_position = Vec3::new(
        world_position.x,
        world_position.y,
        scene_config.origin.translation.z
    );
    
    // Spawn target directly at world position with scene z
    commands.spawn((
        BasicTarget {
            radius: 0.25,
            color: Color::srgb(0.0, 0.5, 1.0),
        },
        PathRenderable { visible: true },
        Transform::from_translation(target_position),
        Name::new("BasicTarget"),
    ));
}

fn update_target_system() {
    // System placeholder for future target updates
}
