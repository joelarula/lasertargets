use bevy::prelude::*;
use common::config::ProjectorConfiguration;
use common::scene::SceneSetup;
use common::path::{UniversalPath, PathSegment};
use lyon_tessellation::path::PathEvent;
use crate::dac::helios::{HeliosDacController, HeliosPoint, HELIOS_FLAGS_DEFAULT, HELIOS_MAX_COORD};


/// Resource for managing the Helios DAC controller
#[derive(Resource)]
pub struct ProjectorDacController {
    controller: Option<HeliosDacController>,
    initialized: bool,
}

impl Default for ProjectorDacController {
    fn default() -> Self {
        Self {
            controller: None,
            initialized: false,
        }
    }
}

pub struct ProjectorPlugin;

impl Plugin for ProjectorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ProjectorConfiguration {
            enabled: true, // Enable by default for testing
            ..Default::default()
        })
            .insert_resource(ProjectorDacController::default())
            .add_systems(Startup, initialize_projector_dac)
            .add_systems(Update, (update_projector, render_paths_to_projector).chain());
    }
}



/// Initialize the Helios DAC controller
fn initialize_projector_dac(mut dac_controller: ResMut<ProjectorDacController>) {
    info!("Initializing Helios DAC controller...");
    match HeliosDacController::new() {
        Ok(mut controller) => {
            info!("Helios DAC library loaded successfully");
            match controller.open_devices() {
                Ok(num_devices) => {
                    info!("✓ Helios DAC initialized: {} devices found and opened", num_devices);
                    
                    // Open the shutter to enable laser output
                    if let Err(e) = controller.set_shutter(0, true) {
                        warn!("Failed to open laser shutter: {}", e);
                    } else {
                        info!("✓ Laser shutter opened");
                    }
                    
                    dac_controller.controller = Some(controller);
                    dac_controller.initialized = true;
                }
                Err(e) => {
                    error!("✗ Failed to open Helios DAC devices: {}", e);
                }
            }
        }
        Err(e) => {
            error!("✗ Failed to initialize Helios DAC controller: {}", e);
        }
    }
}

fn update_projector(
    mut projector_config: ResMut<ProjectorConfiguration>,
    scene_setup: Res<SceneSetup>,
) {
    if projector_config.is_changed() || scene_setup.is_changed() {
        if projector_config.locked_to_scene {
            // Lock projector to scene center
            let scene_center = scene_setup.scene.origin.translation;
            let new_rotation = Transform::from_translation(projector_config.origin.translation)
                .looking_at(scene_center, Vec3::Y).rotation;
            // Only update if rotation actually changed
            if projector_config.origin.rotation != new_rotation {
                projector_config.origin.rotation = new_rotation;
            }
        }
    }
}

/// Render all UniversalPath entities to the projector using Helios DAC
fn render_paths_to_projector(
    projector_config: Res<ProjectorConfiguration>,
    dac_controller: ResMut<ProjectorDacController>,
    path_query: Query<(&UniversalPath, &GlobalTransform)>,
) {
    // Debug: Count available paths
    let path_count = path_query.iter().count();
    if path_count > 0 {
        info!("Found {} UniversalPath entities to render", path_count);
        for (path, transform) in path_query.iter() {
            debug!("UniversalPath at position {:?} with {} segments", transform.translation(), path.segments.len());
        }
    } else {
        debug!("No UniversalPath entities found to render");
    }
    
    // Also check projector configuration
    debug!("Projector enabled: {}, angle: {}", projector_config.enabled, projector_config.angle);
    
    // Only render if projector is enabled
    if !projector_config.enabled {
        if path_count > 0 {
            debug!("Projector disabled, skipping {} paths", path_count);
        }
        return;
    }
    
    // Convert all paths to Helios points for processing
    let mut helios_points = Vec::new();
    
    for (universal_path, global_transform) in path_query.iter() {
        let points = convert_universal_path_to_helios_points(
            universal_path,
            global_transform,
            &projector_config,
        );
        debug!("Converted path to {} Helios points", points.len());
        helios_points.extend(points);
    }
    
    if helios_points.is_empty() {
        return;
    }
    
    info!("Total Helios points to render: {}", helios_points.len());
    
    // Only render if DAC is initialized
    if !dac_controller.initialized {
        warn!("DAC not initialized, cannot render {} points", helios_points.len());
        return;
    }
    
    let Some(ref controller) = dac_controller.controller else {
        warn!("DAC controller not available");
        return;
    };
    
    // Check if any device is ready
    if controller.num_devices() == 0 {
        warn!("No Helios DAC devices available");
        return;
    }
    
    info!("Helios DAC has {} devices available", controller.num_devices());
    
    // For now, use the first device (device 0)
    let device_id: i32 = 0;
    
    // Wait for DAC ready (like bouncing ball example - up to 2000 attempts)
    let mut attempts = 0;
    let mut dac_ready = false;
    
    while attempts < 2000 {
        match controller.get_status(device_id) {
            Ok(ready) => {
                if ready {
                    dac_ready = true;
                    break;
                }
            }
            Err(e) => {
                error!("Failed to get DAC status: {}", e);
                return;
            }
        }
        attempts += 1;
        std::thread::yield_now();
    }
    
    if !dac_ready {
        debug!("DAC not ready after {} attempts, skipping frame", attempts);
        return;
    }
    
    if attempts > 0 {
        debug!("DAC ready after {} attempts", attempts);
    }
    
    // Convert all paths to Helios points
    let mut helios_points = Vec::new();
    
    info!("DAC ready, converting {} UniversalPath entities to Helios points", path_query.iter().count());
                
    for (universal_path, global_transform) in path_query.iter() {
        info!("Processing UniversalPath: {} segments, position {:?}", 
              universal_path.segments.len(), global_transform.translation());
        
        let points = convert_universal_path_to_helios_points(
            universal_path,
            global_transform,
            &projector_config,
        );
        info!("Converted to {} Helios points", points.len());
        
        // Add transition blanking between shapes (if not the first shape)
        if !helios_points.is_empty() && !points.is_empty() {
            // Get the coordinates of the last point of previous shape and first point of new shape
            let (last_x, last_y) = {
                let last_prev: &HeliosPoint = helios_points.last().unwrap();
                (last_prev.x, last_prev.y)
            };
            let (first_x, first_y) = {
                let first_next: &HeliosPoint = &points[0];
                (first_next.x, first_next.y)
            };
            
            // Add 10 blanked transition points to move laser between shapes
            for _ in 0..10 {
                helios_points.push(HeliosPoint::blanked(last_x, last_y));
            }
            for _ in 0..10 {
                helios_points.push(HeliosPoint::blanked(first_x, first_y));
            }
            
            info!("Added 20 blanked transition points between shapes");
        }
        
        helios_points.extend(points);
    }
    
    if helios_points.is_empty() {
        warn!("No Helios points generated from paths");
        return;
    }
    
    info!("Total Helios points to render: {}", helios_points.len());
    
    // Send frame to projector
    let pps = 30000; // Points per second
    
    // Log first, middle, and last points for debugging
    if helios_points.len() > 0 {
        let mid_idx = helios_points.len() / 2;
        let last_idx = helios_points.len() - 1;
        info!("Sample points - First: x={}, y={}, i={}; Mid: x={}, y={}, i={}; Last: x={}, y={}, i={}",
              helios_points[0].x, helios_points[0].y, helios_points[0].i,
              helios_points[mid_idx].x, helios_points[mid_idx].y, helios_points[mid_idx].i,
              helios_points[last_idx].x, helios_points[last_idx].y, helios_points[last_idx].i);
    }
    
    match controller.write_frame_native(0, pps, HELIOS_FLAGS_DEFAULT, &helios_points) {
        Ok(_) => {
            info!("✓ Successfully sent {} points to Helios DAC", helios_points.len());
        }
        Err(e) => {
            error!("✗ Failed to write frame to Helios DAC: {}", e);
        }
    }
}

/// Convert a UniversalPath to Helios points using projector coordinate transformation
fn convert_universal_path_to_helios_points(
    universal_path: &UniversalPath,
    global_transform: &GlobalTransform,
    projector_config: &ProjectorConfiguration,
) -> Vec<HeliosPoint> {
    let mut points = Vec::new();
    
    info!("Converting UniversalPath with {} segments at position {:?}", 
          universal_path.segments.len(), global_transform.translation());
    
    for (i, segment) in universal_path.segments.iter().enumerate() {
        let segment_points = convert_path_segment_to_helios_points(
            segment,
            global_transform,
            projector_config,
        );
        info!("Segment {}: color {:?}, path events: {}, converted to {} points", 
              i, segment.color, segment.path.iter().count(), segment_points.len());
        points.extend(segment_points);
    }
    
    info!("Total points from UniversalPath: {}", points.len());
    points
}

/// Convert a single path segment to Helios points
fn convert_path_segment_to_helios_points(
    segment: &PathSegment,
    global_transform: &GlobalTransform,
    projector_config: &ProjectorConfiguration,
) -> Vec<HeliosPoint> {
    let mut points = Vec::new();
    
    // Extract RGB components from segment color
    let color_linear = segment.color.to_linear();
    let r = (color_linear.red * 255.0) as u8;
    let g = (color_linear.green * 255.0) as u8;
    let b = (color_linear.blue * 255.0) as u8;
    
    // Collect visible points first
    let mut visible_points = Vec::new();
    
    // Tessellate the path into line segments
    let mut current_point: Option<lyon_tessellation::math::Point> = None;
    
    for event in segment.path.iter() {
        match event {
            PathEvent::Begin { at } => {
                current_point = Some(at);
            }
            PathEvent::Line { to, .. } => {
                if let Some(from) = current_point {
                    // Create interpolated points along the line
                    // Increased density: at least 20 points per line segment
                    let steps = ((from.distance_to(to) * 50.0) as usize).max(20);
                    
                    // Add extra points at the start (corner dwell)
                    for _ in 0..5 {
                        let world_pos = Vec3::new(from.x, from.y, 0.0);
                        let transformed_world_pos = global_transform.transform_point(world_pos);
                        if let Some(projector_coords) = world_to_projector_coordinates(
                            transformed_world_pos,
                            projector_config,
                        ) {
                            visible_points.push(projector_coords);
                        }
                    }
                    
                    for i in 0..=steps {
                        let t = i as f32 / steps as f32;
                        let interpolated = from + (to - from) * t;
                        
                        // Transform to world coordinates
                        let world_pos = Vec3::new(interpolated.x, interpolated.y, 0.0);
                        let transformed_world_pos = global_transform.transform_point(world_pos);
                        
                        // Project to projector coordinates
                        if let Some(projector_coords) = world_to_projector_coordinates(
                            transformed_world_pos,
                            projector_config,
                        ) {
                            visible_points.push(projector_coords);
                        }
                    }
                    
                    // Add extra points at the end (corner dwell)
                    for _ in 0..5 {
                        let world_pos = Vec3::new(to.x, to.y, 0.0);
                        let transformed_world_pos = global_transform.transform_point(world_pos);
                        if let Some(projector_coords) = world_to_projector_coordinates(
                            transformed_world_pos,
                            projector_config,
                        ) {
                            visible_points.push(projector_coords);
                        }
                    }
                }
                current_point = Some(to);
            }
            PathEvent::Quadratic { ctrl, to, .. } => {
                if let Some(from) = current_point {
                    // Tessellate quadratic bezier curve
                    let steps = 20; // Number of steps for curve approximation
                    
                    for i in 0..=steps {
                        let t = i as f32 / steps as f32;
                        let interpolated = quadratic_bezier(from, ctrl, to, t);
                        
                        // Transform to world coordinates
                        let world_pos = Vec3::new(interpolated.x, interpolated.y, 0.0);
                        let transformed_world_pos = global_transform.transform_point(world_pos);
                        
                        // Project to projector coordinates
                        if let Some(projector_coords) = world_to_projector_coordinates(
                            transformed_world_pos,
                            projector_config,
                        ) {
                            visible_points.push(projector_coords);
                        }
                    }
                }
                current_point = Some(to);
            }
            PathEvent::Cubic { ctrl1, ctrl2, to, .. } => {
                if let Some(from) = current_point {
                    // Tessellate cubic bezier curve
                    let steps = 30; // Number of steps for curve approximation
                    
                    for i in 0..=steps {
                        let t = i as f32 / steps as f32;
                        let interpolated = cubic_bezier(from, ctrl1, ctrl2, to, t);
                        
                        // Transform to world coordinates
                        let world_pos = Vec3::new(interpolated.x, interpolated.y, 0.0);
                        let transformed_world_pos = global_transform.transform_point(world_pos);
                        
                        // Project to projector coordinates
                        if let Some(projector_coords) = world_to_projector_coordinates(
                            transformed_world_pos,
                            projector_config,
                        ) {
                            visible_points.push(projector_coords);
                        }
                    }
                }
                current_point = Some(to);
            }
            PathEvent::End { close: _, .. } => {
                current_point = None;
            }
        }
    }
    
    // If we have visible points, add blanking points at start and closure points at end
    if !visible_points.is_empty() {
        let first_point = visible_points[0];
        let last_point = visible_points[visible_points.len() - 1];
        
        // Add 5 blanked points at the start to move laser to first position
        for _ in 0..5 {
            points.push(HeliosPoint::blanked(first_point.0, first_point.1));
        }
        
        // Add all visible points with normal status
        for (x, y) in visible_points.iter() {
            points.push(HeliosPoint::new(
                *x,
                *y,
                r,
                g,
                b,
                255, // Full intensity
            ));
        }
        
        // Add 3 overlapping points at the end to ensure closure
        for _ in 0..3 {
            points.push(HeliosPoint::new(
                last_point.0,
                last_point.1,
                r,
                g,
                b,
                255,
            ));
        }
    }
    
    points
}

/// Transform world coordinates to projector coordinates using perspective projection
fn world_to_projector_coordinates(
    world_pos: Vec3,
    projector_config: &ProjectorConfiguration,
) -> Option<(u16, u16)> {
    // Create projector view matrix
    let projector_transform = Mat4::from_scale_rotation_translation(
        projector_config.origin.scale,
        projector_config.origin.rotation,
        projector_config.origin.translation,
    );
    
    // Transform world position to projector local space
    let local_pos = projector_transform.inverse().transform_point3(world_pos);
    
    // Check if point is in front of projector
    // In projector local space, negative Z means the point is in the direction the projector is looking
    if local_pos.z >= 0.0 {
        warn!("Point {:?} behind projector (local_z: {})", world_pos, local_pos.z);
        return None; // Behind projector
    }
    
    // Use absolute value for projection calculations
    let distance = local_pos.z.abs();
    
    // Apply perspective projection
    let fov_rad = projector_config.angle.to_radians();
    let half_fov = fov_rad / 2.0;
    
    // Project to normalized device coordinates [-1, 1]
    let projected_x = (local_pos.x / distance) / half_fov.tan();
    let projected_y = (local_pos.y / distance) / half_fov.tan();
    
    // Clip to visible range
    if projected_x.abs() > 1.0 || projected_y.abs() > 1.0 {
        warn!("Point {:?} outside FOV (projected: {}, {})", world_pos, projected_x, projected_y);
        return None; // Outside field of view
    }
    
    // Convert normalized coordinates [-1, 1] to DAC coordinates [0, 4095]
    // -1 maps to 0, 0 maps to 2048 (center), +1 maps to 4095
    let x = ((projected_x + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    let y = ((projected_y + 1.0) * (HELIOS_MAX_COORD as f32 / 2.0)) as u16;
    
    Some((x, y))
}

/// Quadratic bezier interpolation
fn quadratic_bezier(
    p0: lyon_tessellation::math::Point,
    p1: lyon_tessellation::math::Point,
    p2: lyon_tessellation::math::Point,
    t: f32,
) -> lyon_tessellation::math::Point {
    let inv_t = 1.0 - t;
    let a = inv_t * inv_t;
    let b = 2.0 * inv_t * t;
    let c = t * t;
    
    lyon_tessellation::math::point(
        a * p0.x + b * p1.x + c * p2.x,
        a * p0.y + b * p1.y + c * p2.y,
    )
}

/// Cubic bezier interpolation
fn cubic_bezier(
    p0: lyon_tessellation::math::Point,
    p1: lyon_tessellation::math::Point,
    p2: lyon_tessellation::math::Point,
    p3: lyon_tessellation::math::Point,
    t: f32,
) -> lyon_tessellation::math::Point {
    let inv_t = 1.0 - t;
    let a = inv_t * inv_t * inv_t;
    let b = 3.0 * inv_t * inv_t * t;
    let c = 3.0 * inv_t * t * t;
    let d = t * t * t;
    
    lyon_tessellation::math::point(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}
