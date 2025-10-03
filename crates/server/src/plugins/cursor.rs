use bevy::prelude::*;
use bevy::color::palettes::css::SILVER;

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        //app.add_systems(FixedUpdate, update_cursor.in_set(CursorSystemSet));
        app.add_systems(PostUpdate, update_cursor.after(TransformSystem::TransformPropagate));
    }
}

fn update_cursor(
    window: Query<&Window>,
    mut gizmos: Gizmos,
    camera: Query<(&Camera, &GlobalTransform), With<Camera>>,
) {
        if let Ok(window) = window.single() {
            if let Ok((camera, camera_transform)) = camera.single()  {
                let cursor_pos = window.cursor_position().unwrap_or(Vec2::ZERO);
                let world_pos = camera
                    .viewport_to_world_2d(camera_transform, cursor_pos)
                    .unwrap_or(Vec2::ZERO);
                
                gizmos.circle_2d(world_pos, 5., Color::from(SILVER));
                gizmos.cross_2d(world_pos, 15., Color::from(SILVER));
        };

    } 


}