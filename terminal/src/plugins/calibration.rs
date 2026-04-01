use bevy::prelude::*;
use bevy_quinnet::client::QuinnetClient;
use common::config::{ProjectorConfiguration, SceneConfiguration};
use common::network::NetworkMessage;
use common::scene::SceneData;
use common::scene::SceneSetup;
use common::state::CalibrationState;
use crate::plugins::camera::CameraSystemSet;
use crate::plugins::camera::DisplayMode;
use crate::plugins::instructions::InstructionState;
use std::f32::consts::PI;
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::css::SILVER;
use bevy::color::palettes::css::YELLOW;
use bevy::color::palettes::css::ORANGE;
use bevy::color::palettes::css::RED;



pub const DARK_GREY_THIRD: Srgba = Srgba::new(0.663, 0.663, 0.663, 0.3);

pub const GRID_SPACING: f32 = 0.25;

const INSTRUCTION_TEXT_F3: &str = "Press [F3] to toggle calibration gizmos visibility";
const INSTRUCTION_TEXT_MOUSE_DISTANCE: &str = "Press [Mouse Back/Forward] to move scene distance further/closer";

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct CalibrationSystemSet;


pub struct CalibrationPlugin;

#[derive(Component)]
struct MouseCoordsText;

#[derive(Component)]
struct SceneDimsText;


impl Plugin for CalibrationPlugin {
    fn build(&self, app: &mut App) {
        app

        .add_systems(Startup, setup_calibration_instructions)
        .add_systems(Update, toggle_calibration_visibility.in_set(CalibrationSystemSet))
        .add_systems(Update, adjust_scene_distance_with_mouse.in_set(CalibrationSystemSet))
        .add_systems(Update, ensure_calibration_text.in_set(CalibrationSystemSet))
        .add_systems(Update, update_grid.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_billboard_gizmos.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_projector_billboard.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_scene_crosshair.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, draw_mouse_crosshair.in_set(CalibrationSystemSet).after(CameraSystemSet))
        .add_systems(Update, update_calibration_text.in_set(CalibrationSystemSet).after(CameraSystemSet));
    }
}


fn setup_calibration_instructions(mut instruction_state: ResMut<InstructionState>) {
    instruction_state.instructions.push(INSTRUCTION_TEXT_F3.to_string());
    instruction_state.instructions.push(INSTRUCTION_TEXT_MOUSE_DISTANCE.to_string());
}

fn adjust_scene_distance_with_mouse(
    calibration_state: Res<State<CalibrationState>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut scene_config: ResMut<SceneConfiguration>,
) {
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }

    const DISTANCE_STEP: f32 = 0.25;

    let mut delta = 0.0;
    if mouse_buttons.just_pressed(MouseButton::Back) {
        delta -= DISTANCE_STEP;
    }
    if mouse_buttons.just_pressed(MouseButton::Forward) {
        delta += DISTANCE_STEP;
    }

    if delta == 0.0 {
        return;
    }

    let z = scene_config.origin.translation.z;
    let signed_delta = if z <= 0.0 { delta } else { -delta };
    scene_config.origin.translation.z = z + signed_delta;
}

fn ensure_calibration_text(
    mut commands: Commands,
    mouse_query: Query<Entity, With<MouseCoordsText>>,
    dims_query: Query<Entity, With<SceneDimsText>>,
) {
    if !mouse_query.is_empty() || !dims_query.is_empty() {
        return;
    }

    commands.spawn((
        MouseCoordsText,
        Text::new("Mouse: --"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        ZIndex(1000),
        Visibility::Hidden,
    ));

    commands.spawn((
        SceneDimsText,
        Text::new("Scene: --"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        ZIndex(1000),
        Visibility::Hidden,
    ));
}

fn toggle_calibration_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    calibration_state: Res<State<CalibrationState>>,
    mut commands: Commands,
    mut client: ResMut<QuinnetClient>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        let new_state = match calibration_state.get() {
            CalibrationState::On => CalibrationState::Off,
            CalibrationState::Off => CalibrationState::On,
        };
        
        // Update local state
        commands.insert_resource(NextState::Pending(new_state.clone()));
        
        // Send to server
        if let Some(connection) = client.get_connection_mut() {
            let message = NetworkMessage::UpdateCalibrationState(new_state.clone());
            if let Ok(payload) = message.to_bytes() {
                if let Err(e) = connection.send_payload(payload) {
                    error!("Failed to send calibration state update: {:?}", e);
                } else {
                    info!("Calibration state toggled to: {:?}", new_state);
                }
            }
        }
    }
}

fn update_grid(mut gizmos: Gizmos, scene_configuration: Res<SceneConfiguration>, display_mode: Res<DisplayMode>, calibration_state: Res<State<CalibrationState>>) {
    
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    if *display_mode == DisplayMode::Mode3D {
        gizmos.grid(
            Quat::from_rotation_x(PI / 2.),
            UVec2::new((scene_configuration.scene_dimension.x * 4.) as u32, (scene_configuration.origin.translation.z.abs() * 4.) as u32),
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
    calibration_state: Res<State<CalibrationState>>,
) {
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    for(_camera, camera_transform) in camera_query.iter(){ // Prefixed with underscore to ignore unused camera variable
        let billboard_position = scene_configuration.origin.translation;
        let width = scene_configuration.scene_dimension.x;
        let height = scene_configuration.scene_dimension.y;
        
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
            true
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
    calibration_state: Res<State<CalibrationState>>,
) {
    if *calibration_state.get() == CalibrationState::Off || !projector_config.switched_on {
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
        false
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
    draw_grid: bool,
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
    
    if !draw_grid {
        return;
    }

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
    calibration_state: Res<State<CalibrationState>>,
) {
    if *calibration_state.get() == CalibrationState::Off {
        return;
    }
    
    let center = scene_setup.scene.origin.translation;
    let crosshair_size = GRID_SPACING * 2.0; // Larger crosshair for scene center
    gizmos.line(center - Vec3::X * crosshair_size, center + Vec3::X * crosshair_size, RED);
    gizmos.line(center - Vec3::Y * crosshair_size, center + Vec3::Y * crosshair_size, RED);
}

fn update_calibration_text(
    scene_setup: Res<SceneSetup>,
    scene_data: Res<SceneData>,
    calibration_state: Res<State<CalibrationState>>,
    mut mouse_query: Query<(&mut Text, &mut Node, &mut Visibility), (With<MouseCoordsText>, Without<SceneDimsText>)>,
    mut dims_query: Query<(&mut Text, &mut Node, &mut Visibility), (With<SceneDimsText>, Without<MouseCoordsText>)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let is_visible = *calibration_state.get() == CalibrationState::On;

    let scene_dims = scene_setup.scene.scene_dimension;
    let scene_distance = scene_setup.scene.origin.translation.z.abs();
    let half_width = scene_dims.x / 2.0;
    let half_height = scene_dims.y / 2.0;
    let padding = 0.25;
    let depth = 0.01;

    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    let scene_origin = scene_setup.scene.origin.translation;
    let scene_rotation = scene_setup.scene.origin.rotation;
    let right = scene_rotation.mul_vec3(Vec3::X);
    let up = scene_rotation.mul_vec3(Vec3::Y);
    let normal = scene_rotation.mul_vec3(Vec3::Z);

    let left_bottom_world = scene_origin
        + right * (-half_width + padding)
        + up * (-half_height + padding)
        + normal * depth;

    let right_bottom_world = scene_origin
        + right * (half_width - padding)
        + up * (-half_height + padding)
        + normal * depth;

    let left_screen = camera.world_to_viewport(camera_transform, left_bottom_world).ok();
    let right_screen = camera.world_to_viewport(camera_transform, right_bottom_world).ok();

    const LABEL_FONT_SIZE: f32 = 14.0;
    const AVG_CHAR_WIDTH_FACTOR: f32 = 0.6;
    const LABEL_PADDING_PX: f32 = 6.0;

    let mouse_offset = Vec2::new(0.0, -14.0);

    if let Ok((mut text, mut node, mut visibility)) = mouse_query.single_mut() {
        let mut show = is_visible;
        let label = if let Some(world_pos) = scene_data.mouse_world_pos {
            let scene_matrix = Mat4::from_scale_rotation_translation(
                scene_setup.scene.origin.scale,
                scene_setup.scene.origin.rotation,
                scene_setup.scene.origin.translation,
            );
            let local_pos = scene_matrix.inverse().transform_point3(world_pos);
            let display_y = -local_pos.y;
            format!("Mouse: ({:.2}, {:.2})", local_pos.x, display_y)
        } else {
            "Mouse: --".to_string()
        };

        **text = label;

        if let Some(pos) = left_screen {
            node.left = Val::Px(pos.x + mouse_offset.x);
            node.top = Val::Px(pos.y + mouse_offset.y);
        } else {
            show = false;
        }

        *visibility = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    if let Ok((mut text, mut node, mut visibility)) = dims_query.single_mut() {
        let mut show = is_visible;
        let label = format!(
            "Scene: {:.2}m x {:.2}m | Dist: {:.2}m",
            scene_dims.x,
            scene_dims.y,
            scene_distance
        );

        **text = label.clone();

        if let Some(pos) = right_screen {
            let estimated_width = label.chars().count() as f32 * LABEL_FONT_SIZE * AVG_CHAR_WIDTH_FACTOR;
            node.left = Val::Px(pos.x - estimated_width - LABEL_PADDING_PX);
            node.top = Val::Px(pos.y - LABEL_FONT_SIZE);
        } else {
            show = false;
        }

        *visibility = if show { Visibility::Visible } else { Visibility::Hidden };
    }
}