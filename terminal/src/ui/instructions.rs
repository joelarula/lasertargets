//! On-screen help, toggled with [I]. Plugins add their lines with `app.add_instructions(...)`.

use bevy::prelude::*;

#[derive(Resource, Default)]
struct Instructions {
    lines: Vec<String>,
    visible: bool,
}

pub trait InstructionsAppExt {
    /// Adds help lines, shown in the order plugins are built
    fn add_instructions(&mut self, lines: &[&str]) -> &mut Self;
}

impl InstructionsAppExt for App {
    fn add_instructions(&mut self, lines: &[&str]) -> &mut Self {
        let mut instructions = self.world_mut().get_resource_or_init::<Instructions>();
        instructions.lines.extend(lines.iter().map(|line| line.to_string()));
        self
    }
}

#[derive(Component)]
struct InstructionsText;

pub struct InstructionsPlugin;

impl Plugin for InstructionsPlugin {
    fn build(&self, app: &mut App) {
        let mut instructions = app.world_mut().get_resource_or_init::<Instructions>();
        instructions.lines.insert(0, "Press [I] to toggle instructions".to_string());
        app.add_systems(Startup, spawn_text)
            .add_systems(Update, (toggle, show.run_if(resource_changed::<Instructions>)).chain());
    }
}

fn spawn_text(mut commands: Commands) {
    commands.spawn((
        InstructionsText,
        Text::default(),
        TextFont { font_size: 14.0, ..default() },
        Node { position_type: PositionType::Absolute, top: Val::Px(40.0), left: Val::Px(100.0), ..default() },
        Visibility::Hidden,
    ));
}

fn toggle(keys: Res<ButtonInput<KeyCode>>, mut instructions: ResMut<Instructions>) {
    if keys.just_pressed(KeyCode::KeyI) {
        instructions.visible = !instructions.visible;
    }
}

fn show(instructions: Res<Instructions>, text: Single<(&mut Text, &mut Visibility), With<InstructionsText>>) {
    let (mut text, mut visibility) = text.into_inner();
    text.0 = instructions.lines.join("\n");
    *visibility = if instructions.visible { Visibility::Inherited } else { Visibility::Hidden };
}
