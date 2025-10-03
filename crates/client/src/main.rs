use bevy::prelude::*;
use log::info;

fn main() {
    // Initialize logger
    pretty_env_logger::init();
    info!("Starting LaserTargets Client...");

    App::new()
        .add_plugins(DefaultPlugins.set(
            WindowPlugin {
                primary_window: Some(Window {
                    title: "LaserTargets Client".into(),
                    resolution: (1280.0, 720.0).into(),
                    ..default()
                }),
                ..default()
            }
        ))
        .add_systems(Startup, setup_client)
        .add_systems(Update, (
            handle_input,
        ))
        .run();
}

fn setup_client(mut commands: Commands) {
    info!("Setting up client...");
    
    // Add a camera
    commands.spawn(Camera2d);
    
    // Add some basic UI
    commands.spawn((
        Text::new("LaserTargets Client\nPress ESC to exit"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn handle_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        info!("Exit requested by user");
        exit.send(AppExit::Success);
    }
}