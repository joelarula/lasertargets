use bevy::prelude::*;
use log::info;
use std::collections::HashMap;
use bevy::color::palettes::css::LIGHT_SKY_BLUE;
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
    pub icon: Option<String>,
}

#[derive(Resource)]
struct NerdFont(Handle<Font>);

#[derive(Resource, Default)]
pub struct ToolbarRegistry {
    buttons: HashMap<String, ButtonHandler>,
    registered_buttons: Vec<String>,
}

impl ToolbarRegistry {

    
    pub fn register_icon_button(&mut self, name: String, callback: fn(), icon: String) {
        self.buttons.insert(name.clone(), ButtonHandler { 
            callback,
            icon: Some(icon),
        });
        if !self.registered_buttons.contains(&name) {
            self.registered_buttons.push(name);
        }
    }
}

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ToolbarRegistry::default())
            .add_systems(Startup, load_nerd_font)
            .add_systems(PostStartup, setup_toolbar)
            .add_systems(Update, (handle_dynamic_buttons, update_toolbar));
    }
}

fn load_nerd_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let nerd_font = asset_server.load("FiraCodeNerdFont-Regular.ttf");
    commands.insert_resource(NerdFont(nerd_font));
    info!("Loading Nerd Font from FiraCodeNerdFont-Regular.ttf");
}

fn setup_toolbar(mut commands: Commands, registry: Res<ToolbarRegistry>, nerd_font: Option<Res<NerdFont>>) {
    if let Some(font) = nerd_font {
        info!("Setting up toolbar with Nerd Font");
        create_toolbar_ui(&mut commands, &registry, Some(&font.0));
    } else {
        info!("Setting up toolbar without Nerd Font (font not loaded)");
        create_toolbar_ui(&mut commands, &registry, None);
    }
}

fn update_toolbar(
    mut commands: Commands,
    registry: Res<ToolbarRegistry>,
    toolbar_query: Query<Entity, With<ToolbarContainer>>,
    nerd_font: Option<Res<NerdFont>>,
) {
    if registry.is_changed() {
        if let Ok(toolbar_entity) = toolbar_query.single() {
            commands.entity(toolbar_entity).despawn();
            if let Some(font) = nerd_font {
                create_toolbar_ui(&mut commands, &registry, Some(&font.0));
            } else {
                create_toolbar_ui(&mut commands, &registry, None);
            }
        }
    }
}

fn create_toolbar_ui(commands: &mut Commands, registry: &ToolbarRegistry, nerd_font: Option<&Handle<Font>>) {
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
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|parent| {
            // Create buttons for all registered buttons
            for button_name in &registry.registered_buttons {
                if let Some(button_handler) = registry.buttons.get(button_name) {
                    
                    let button_size = 36.0 ;
    
                    parent
                        .spawn((
                            DynamicButton { id: button_name.clone() },
                            Button,
                            Node {
                                width: Val::Px(button_size),
                                height: Val::Px(button_size),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(LIGHT_SKY_BLUE.into()),
                            BorderRadius::all(Val::Px(6.0)),
                        ))
                        .with_child((
                            Text::new(
                                button_handler.icon.as_ref()
                                    .unwrap_or(button_name)
                                    .clone()
                            ),
                            TextFont {
                                font: if button_handler.icon.is_some() && nerd_font.is_some() { 
                                    nerd_font.unwrap().clone()
                                } else { 
                                    default() 
                                },
                                font_size: 12.0 ,
                                ..default()
                            },
                            TextColor(Color::WHITE)
                        ));
                }
            }
        });
}



fn handle_dynamic_buttons(
    mut interaction_query: Query<
        (&Interaction, &DynamicButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
    registry: Res<ToolbarRegistry>,
) {
    for (interaction, button, mut background_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *background_color = BackgroundColor(Color::srgba(0.2, 0.4, 0.6, 0.7));
                if let Some(handler) = registry.buttons.get(&button.id) {
                    (handler.callback)();
                }
            }
            Interaction::Hovered => {
                *background_color = BackgroundColor(Color::srgba(0.5, 0.7, 0.9, 0.6));
            }
            Interaction::None => {
                *background_color = BackgroundColor(Color::srgba(0.4, 0.6, 0.8, 0.5));
            }
        }
    }
}

