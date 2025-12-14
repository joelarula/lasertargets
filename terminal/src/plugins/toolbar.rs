use bevy::prelude::*;
use bevy::color::palettes::css::{ STEEL_BLUE, LIGHT_SEA_GREEN};
use std::collections::HashMap;

// Button color palette using Bevy CSS color palettes
mod button_colors {
    use bevy::prelude::Color;
    use bevy::color::palettes::css::{DODGER_BLUE, ORANGE_RED, DIM_GRAY, LIME};
    
    pub const PRESSED: Color = Color::srgba(0.1, 0.3, 0.8, 0.9);
    pub const ACTIVE_HOVERED: Color = Color::srgba(LIME.red, LIME.green, LIME.blue, 1.0);
    pub const INACTIVE_HOVERED: Color = Color::srgba(DODGER_BLUE.red, DODGER_BLUE.green, DODGER_BLUE.blue, 0.9);
    pub const ACTIVE: Color = Color::srgba(ORANGE_RED.red, ORANGE_RED.green, ORANGE_RED.blue, 0.95);
    pub const INACTIVE: Color = Color::srgba(DIM_GRAY.red, DIM_GRAY.green, DIM_GRAY.blue, 0.7);
}

pub struct ToolbarPlugin;

#[derive(Component)]
struct ToolbarContainer;

#[derive(Component)]
pub struct ToolabarButton {
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Docking {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone)]
pub struct ToolbarItem {
    pub name: String,
    pub label: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub docking: Docking,
    pub button_size: f32,
}

#[derive(Resource)]
struct NerdFont(Handle<Font>);

#[derive(Resource, Default)]
pub struct ToolbarRegistry {
    buttons: HashMap<String, ToolbarItem>
}

impl ToolbarRegistry {
 
    pub fn register_button(&mut self, item: ToolbarItem) {
        self.buttons.insert(item.name.clone(), item);
    }
    
    pub fn update_button_state(&mut self, name: &str, is_active: bool) {
        if let Some(handler) = self.buttons.get_mut(name) {
            handler.is_active = is_active;
        }
    }


}

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(ToolbarRegistry::default())
            .add_systems(Startup, load_nerd_font)
            .add_systems(PostStartup, setup_toolbar)
            .add_systems(Update, (
                update_button_states,
                update_toolbar,
            ).chain());
    }
}

fn load_nerd_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let nerd_font = asset_server.load("FiraCodeNerdFont-Regular.ttf");
    commands.insert_resource(NerdFont(nerd_font));
}

fn setup_toolbar(mut commands: Commands, registry: Res<ToolbarRegistry>, nerd_font: Option<Res<NerdFont>>) {
    if let Some(font) = nerd_font {
        create_toolbar_ui(&mut commands, &registry, Some(&font.0));
    } else {
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
  
    let mut left_buttons = Vec::new();
    let mut right_buttons = Vec::new();
    let mut top_buttons = Vec::new();
    let mut bottom_buttons = Vec::new();
    
    for button_name in registry.buttons.keys() {
        if let Some(handler) = registry.buttons.get(button_name) {
            match handler.docking {
                Docking::Left => left_buttons.push(button_name.clone()),
                Docking::Right => right_buttons.push(button_name.clone()),
                Docking::Top => top_buttons.push(button_name.clone()),
                Docking::Bottom => bottom_buttons.push(button_name.clone()),
            }
        }
    }
    
    create_docked_toolbar(commands, registry, nerd_font, &left_buttons, Docking::Left);
    create_docked_toolbar(commands, registry, nerd_font, &right_buttons, Docking::Right);
    create_docked_toolbar(commands, registry, nerd_font, &top_buttons, Docking::Top);
    create_docked_toolbar(commands, registry, nerd_font, &bottom_buttons, Docking::Bottom);
}

fn create_docked_toolbar(
    commands: &mut Commands,
    registry: &ToolbarRegistry,
    nerd_font: Option<&Handle<Font>>,
    button_names: &[String],
    docking: Docking,
) {
    if button_names.is_empty() {
        return;
    }
    
    let (position_style, flex_direction) = match docking {
        Docking::Left => (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            FlexDirection::Column
        ),
        Docking::Right => (
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            FlexDirection::Column
        ),
        Docking::Top => (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            FlexDirection::Row
        ),
        Docking::Bottom => (
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            FlexDirection::Row
        ),
    };
    
    commands
        .spawn((
            ToolbarContainer,
            position_style,
            ZIndex(1000),
            BorderRadius::all(Val::Px(5.0)),
        ))
        .with_children(|parent| {
            for button_name in button_names {
                if let Some(button_handler) = registry.buttons.get(button_name) {
                    let initial_color = if button_handler.is_active {
                        button_colors::ACTIVE
                    } else {
                        button_colors::INACTIVE
                    };
    
                    parent
                        .spawn((
                            ToolabarButton { name: button_name.clone() },
                            Button,
                            Node {
                                width: Val::Px(button_handler.button_size),
                                height: Val::Px(button_handler.button_size),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::all(Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(initial_color),
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






fn update_button_states(
    mut button_query: Query<(&ToolabarButton, &Interaction, &mut BackgroundColor)>,
    registry: Res<ToolbarRegistry>,
) {
    // Update button colors based on handler state
    for (button, interaction, mut background_color) in &mut button_query {
        let is_active = registry.buttons.get(&button.name)
            .map(|h| h.is_active)
            .unwrap_or(false);

        match *interaction {
            Interaction::Pressed => {
                *background_color = BackgroundColor(button_colors::PRESSED);
            }
            Interaction::Hovered => {
                if is_active {
                    *background_color = BackgroundColor(button_colors::ACTIVE_HOVERED);
                } else {
                    *background_color = BackgroundColor(button_colors::INACTIVE_HOVERED);
                }
            }
            Interaction::None => {
                if is_active {
                    *background_color = BackgroundColor(button_colors::ACTIVE);
                } else {
                    *background_color = BackgroundColor(button_colors::INACTIVE);
                }
            }
        }
    }
}

