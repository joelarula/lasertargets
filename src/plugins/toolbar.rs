use bevy::prelude::*;
use log::info;
use std::collections::HashMap;

pub struct ToolbarPlugin;

#[derive(Component)]
struct ToolbarContainer;

#[derive(Component)]
struct DynamicButton {
    pub id: String,
}

#[derive(Clone)]
pub struct ButtonHandler {
    pub callback: fn(),
}

#[derive(Resource, Default)]
pub struct ToolbarRegistry {
    pub buttons: HashMap<String, ButtonHandler>,
    pub registered_buttons: Vec<String>,
}

impl ToolbarRegistry {
    pub fn register_button(&mut self, name: String, callback: fn()) {
        self.buttons.insert(name.clone(), ButtonHandler { callback });
        if !self.registered_buttons.contains(&name) {
            self.registered_buttons.push(name);
        }
    }
}

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ToolbarRegistry::default())
            .add_systems(PostStartup, setup_toolbar)
            .add_systems(Update, (handle_dynamic_buttons, update_toolbar));
    }
}

fn setup_toolbar(mut commands: Commands, registry: Res<ToolbarRegistry>) {
    create_toolbar_ui(&mut commands, &registry);
}

fn update_toolbar(
    mut commands: Commands,
    registry: Res<ToolbarRegistry>,
    toolbar_query: Query<Entity, With<ToolbarContainer>>,
) {
    if registry.is_changed() {
        if let Ok(toolbar_entity) = toolbar_query.single() {
            commands.entity(toolbar_entity).despawn();
            create_toolbar_ui(&mut commands, &registry);
        }
    }
}

fn create_toolbar_ui(commands: &mut Commands, registry: &ToolbarRegistry) {
    commands
        .spawn((
            ToolbarContainer,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            ZIndex(1000), // High z-index to ensure it's on top
            BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|parent| {
            // Create buttons for all registered buttons
            for button_name in &registry.registered_buttons {
                parent
                    .spawn((
                        DynamicButton { id: button_name.clone() },
                        Button,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::all(Val::Px(5.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.4, 0.6, 0.8)),
                        BorderRadius::all(Val::Px(3.0)),
                    ))
                    .with_child((
                        Text::new(button_name.clone()),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
            }
        });
}



fn handle_dynamic_buttons(
    mut interaction_query: Query<
        (&Interaction, &DynamicButton),
        Changed<Interaction>,
    >,
    registry: Res<ToolbarRegistry>,
) {
    for (interaction, button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                if let Some(handler) = registry.buttons.get(&button.id) {
                    (handler.callback)();
                }
            }
            _ => {}
        }
    }
}

