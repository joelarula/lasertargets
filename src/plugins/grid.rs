use bevy::{color::palettes::css::*, math::Isometry2d, prelude::*};
use crate::plugins::config::ConfigState;
pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(FixedUpdate, update_grid);
    }
}

fn update_grid(  
    window: Query<&Window>, 
    mut gizmos: Gizmos, 
    config: Res<ConfigState>) {

    let Ok(window) = window.single() else {
        return;
    };

    let window_size = window.resolution.physical_size(); 
    let x_cells = (window_size.x as f32 / config.grid_spacing ).round() as u32;
    let y_cells = (window_size.y as f32 / config.grid_spacing ).round() as u32;

    gizmos
        .grid_2d(
            Isometry2d::IDENTITY,
            UVec2::new( x_cells, y_cells),
            Vec2::new(config.grid_spacing , config.grid_spacing),
            LIGHT_GRAY,
        )
        .outer_edges();


}
