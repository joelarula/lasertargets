use bevy::app::ScheduleRunnerPlugin;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use clap::Parser;
use common::settings::{ConfigSection, ConfigSource};
use server::{add_server_plugins, RuntimeSettings};
use server_core::plugins::keyboard::ConsoleKeyboardPlugin;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(about = "Laser targets server")]
struct Args {
    /// YAML config file [default: config/server.yaml in the working directory or next to the executable]
    #[arg(long, env = "LASER_CONFIG")]
    config: Option<PathBuf>,
    /// Profile applied on top of `default` (a top-level key in the config file, e.g. rpi)
    #[arg(long, env = "LASER_PROFILE", default_value = "default")]
    profile: String,
}

fn main() {
    let args = Args::parse();
    if let Some(path) = args.config.as_ref().filter(|path| !path.is_file()) {
        eprintln!("config file not found: {}", path.display());
        std::process::exit(1);
    }
    let file = args.config.or_else(|| ConfigSource::locate("server.yaml"));
    let config = ConfigSource::load(file.as_deref(), &args.profile);

    // Read before the app exists: the schedule runner and logger are built first.
    let runtime = config
        .section(RuntimeSettings::KEY, RuntimeSettings::default())
        .unwrap_or_else(|e| panic!("invalid config section 'runtime': {e}"));
    let level = runtime
        .log_level
        .parse()
        .unwrap_or_else(|_| panic!("invalid runtime.log_level '{}'", runtime.log_level));

    let mut app = App::new();
    app.add_plugins(LogPlugin { level, filter: runtime.log_filter.clone(), ..default() });
    match &file {
        Some(path) => info!("Config: {} (profile '{}')", path.display(), args.profile),
        None => warn!("No config file found, using defaults (profile '{}')", args.profile),
    }

    let tick = Duration::from_secs_f64(1.0 / runtime.tick_hz);
    app.insert_resource(config).insert_resource(runtime);
    add_server_plugins(&mut app, ScheduleRunnerPlugin::run_loop(tick));
    app.add_plugins(ConsoleKeyboardPlugin);
    app.run();
}
