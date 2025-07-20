use bevy::prelude::*;
//use bevy::window::WindowScaleFactorChanged;
mod plugins;
use crate::plugins::instructions::InstructionsPlugin;
use crate::plugins::config::ConfigPlugin;
use crate::plugins::camera::CameraPlugin;
use crate::plugins::grid::GridPlugin;
use crate::plugins::cursor::CursorPlugin;

#[derive(Component)]
struct InstructionText;

const FIXED_TIMESTEP: f64 = 1.0 / 50.0; 

fn main() {

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Video Targets".to_string(),
                present_mode: bevy::window::PresentMode::AutoNoVsync ,
                ..Default::default()
            }),
            ..Default::default()
    }))
    .insert_resource(ClearColor(Color::BLACK)) 
    .add_plugins(InstructionsPlugin)
    .add_plugins(ConfigPlugin)
    .add_plugins(CameraPlugin)
    .add_plugins(GridPlugin)
    .add_plugins(CursorPlugin)
   // .add_systems(Update, detect_window_scale_factor)
    .insert_resource(Time::<Fixed>::from_seconds(FIXED_TIMESTEP));
    app.run();
}

// NEW: A system that detects when the window's scale factor changes to approximately 1.5 (150%).
//fn detect_window_scale_factor(
//    mut scale_factor_events: EventReader<WindowScaleFactorChanged>,
//) {
//    for event in scale_factor_events.read() {
//        let current_scale_factor = event.scale_factor;


//            println!(
//                "Window scale factor is approximately ({}x)!",
//                current_scale_factor
//            );
       
//    }
//}