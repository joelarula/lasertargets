use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use server::create_server_app;
use std::time::Duration;

fn main() {
    create_server_app(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f64(1.0 / 60.0), // 60 FPS
    ))
    .run();
}
