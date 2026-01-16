use bevy::prelude::*;
use common::scene::SceneSetup;
use crate::plugins::camera::CameraSystemSet;
use crate::plugins::camera::DisplayMode;
use crate::plugins::scene::SceneData;
use crate::plugins::instructions::InstructionState;
use common::config::{SceneConfiguration, ProjectorConfiguration};
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::YELLOW;
use bevy::color::palettes::css::ORANGE;
use bevy::color::palettes::css::RED;


pub const DARK_GREY_THIRD: Srgba = Srgba::new(0.663, 0.663, 0.663, 0.3);

pub const GRID_SPACING: f32 = 0.25;

const INSTRUCTION_TEXT_F3: &str = "Press [F3] to toggle calibration gizmos visibility";

#[derive(Resource, Default)]
pub struct CalibrationVisible(pub bool);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CalibrationSystemSet;


pub struct CalibrationPlugin;


impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(CalibrationVisible(true)) // Start with calibration visible
        .add_systems(Startup, setup_calibration_instructions)
        .add_systems(Update, toggle_calibration_visibility.in_set(CalibrationSystemSet))
        .add_systems(Update, update_grid.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_billboard_gizmos.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_projector_billboard.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_scene_crosshair.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_mouse_crosshair.in_set(CalibrationSystemSet).after(CameraSystemSet));
    }
}


fn setup_calibration_instructions(mut instruction_state: ResMut<InstructionState>) {
    instruction_state.instructions.push(INSTRUCTION_TEXT_F3.to_string());
}

fn toggle_calibration_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut calibration_visible: ResMut<CalibrationVisible>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        calibration_visible.0 = !calibration_visible.0;
        info!("Calibration gizmos visibility toggled: {}", calibration_visible.0);
    }
}

fn update_grid(mut gizmos: Gizmos, scene_configuration: Res<SceneConfiguration>, display_mode: Res<DisplayMode>, calibration_visible: Res<CalibrationVisible>) {
    
    if !calibration_visible.0 {
        return;
    }
    
    if *display_mode == DisplayMode::Mode3D {
        gizmos.grid(
            Quat::from_rotation_x(PI / 2.),
            UVec2::new((scene_configuration.scene_dimension.x as f32 * 4.) as u32, (scene_configuration.origin.translation.z.abs() * 4.) as u32),
            Vec2::new(GRID_SPACING, GRID_SPACING),
            DARK_GREY
        );  
    }
}


fn draw_billboard_gizmos(
    mut gizmos: Gizmos,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    scene_data: Res<SceneData>,
    scene_configuration: Res<SceneConfiguration>,
    calibration_visible: Res<CalibrationVisible>,
) {
    if !calibration_visible.0 {
        return;
    }
    
    for(_camera, camera_transform) in camera_query.iter(){ // Prefixed with underscore to ignore unused camera variable
        let billboard_position = scene_configuration.origin.translation;
        let width = scene_configuration.scene_dimension.x as f32;
        let height = scene_configuration.scene_dimension.y as f32;
        
        // Get the scene plane rotation
        let billboard_rotation = scene_configuration.origin.rotation;
        
        draw_billboard_grid(
            &mut gizmos,
            billboard_rotation,
            billboard_position,
            width,
            height,
            SILVER,
            DARK_GREY_THIRD,
            GRID_SPACING,
        );
    }
}


fn draw_mouse_crosshair(
    mut gizmos: Gizmos,
    scene_data: Res<SceneData>,
    scene_configuration: Res<SceneConfiguration>,
) {
    // Mouse crosshair is always visible regardless of calibration toggle
    let billboard_rotation = scene_configuration.origin.rotation;
    let billboard_up = billboard_rotation.mul_vec3(Vec3::Y);
    let billboard_right = billboard_rotation.mul_vec3(Vec3::X);
    
    draw_crosshair(&mut gizmos, &scene_data, &billboard_right, &billboard_up);
}

fn draw_crosshair(
    gizmos: &mut Gizmos,
    scene_data: &SceneData,
    billboard_right: &Vec3,
    billboard_up: &Vec3,
) {
      if scene_data.mouse_world_pos.is_some() {
                let intersection_point = scene_data.mouse_world_pos.unwrap();

        // Draw a 3D crosshair at the intersection point.
        let crosshair_size = GRID_SPACING * 0.5;
        gizmos.line(
            intersection_point - billboard_right * crosshair_size,
            intersection_point + billboard_right * crosshair_size,
            YELLOW,
        );

        gizmos.line(
            intersection_point - billboard_up * crosshair_size,
            intersection_point + billboard_up * crosshair_size,
            YELLOW,
        );
    }
}

fn draw_projector_billboard(
    mut gizmos: Gizmos,
    scene_setup: Res<SceneSetup>,
    projector_config: Res<ProjectorConfiguration>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    calibration_visible: Res<CalibrationVisible>,
) {
    if !calibration_visible.0 || !projector_config.enabled {
        return;
    }

    let billboard_position = scene_setup.scene.origin.translation;
    
    let (width, height) = {
        let dims = scene_setup.get_projector_view_dimensions();
        (dims.x, dims.y)
    };

    // Calculate camera-facing rotation for better 3D perspective
    let billboard_rotation = if let Ok(camera_transform) = camera_query.single() {
        // Make the projector billboard face the camera for better 3D visualization
        let mut camera_position_flat = camera_transform.translation();
        camera_position_flat.y = billboard_position.y;
        Transform::from_translation(billboard_position)
            .looking_at(camera_position_flat, Vec3::Y).rotation
    } else {
        // Fallback to projector configuration rotation
        projector_config.origin.rotation
    };

    let orange_alpha = Srgba::new(1.0, 0.647, 0.0, 0.3);

    draw_billboard_grid(
        &mut gizmos,
        billboard_rotation,
        billboard_position,
        width,
        height,
        ORANGE,
        orange_alpha,
        GRID_SPACING * 2.0,
    );

}


fn draw_billboard_grid(
    gizmos: &mut Gizmos,
    billboard_rotation: Quat,
    billboard_position: Vec3,
    width: f32,
    height: f32,
    frame_color: impl Into<LinearRgba>,
    grid_color: impl Into<LinearRgba>,
    grid_size: f32,
) {
    use bevy::prelude::Color;

    // Convert colors to bevy::prelude::Color
    let frame_color: Color = Color::from(frame_color.into());
    let grid_color: Color = Color::from(grid_color.into());

    
    let billboard_up = billboard_rotation.mul_vec3(Vec3::Y);
    let billboard_right = billboard_rotation.mul_vec3(Vec3::X);
    
    // Draw the frame
    let p1 = billboard_position - billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
    let p2 = billboard_position + billboard_right * (width / 2.0) + billboard_up * (height / 2.0);
    let p3 = billboard_position + billboard_right * (width / 2.0) - billboard_up * (height / 2.0);
    let p4 = billboard_position - billboard_right * (width / 2.0) - billboard_up * (height / 2.0);
    
    gizmos.line(p1, p2, frame_color);
    gizmos.line(p2, p3, frame_color);
    gizmos.line(p3, p4, frame_color);
    gizmos.line(p4, p1, frame_color);
    
    // Draw grid lines
    let num_x_lines = (width / grid_size) as usize;
    let num_y_lines = (height / grid_size) as usize;
    
    // Vertical grid lines
    for i in 0..=num_x_lines {
        let offset_x = (i as f32) * grid_size - width / 2.0;
        let start = billboard_position + billboard_right * offset_x - billboard_up * (height / 2.0);
        let end = billboard_position + billboard_right * offset_x + billboard_up * (height / 2.0);
        gizmos.line(start, end, grid_color);
    }
    
    // Horizontal grid lines
    for i in 0..=num_y_lines {
        let offset_y = (i as f32) * grid_size - height / 2.0;
        let start = billboard_position + billboard_up * offset_y - billboard_right * (width / 2.0);
        let end = billboard_position + billboard_up * offset_y + billboard_right * (width / 2.0);
        gizmos.line(start, end, grid_color);
    }
}

fn draw_scene_crosshair(
    mut gizmos: Gizmos,
    scene_setup: Res<SceneSetup>,
    calibration_visible: Res<CalibrationVisible>,
) {
    if !calibration_visible.0 {
        return;
    }
    
    let center = scene_setup.scene.origin.translation;
    let crosshair_size = GRID_SPACING * 2.0; // Larger crosshair for scene center
    gizmos.line(center - Vec3::X * crosshair_size, center + Vec3::X * crosshair_size, RED);
    gizmos.line(center - Vec3::Y * crosshair_size, center + Vec3::Y * crosshair_size, RED);
}