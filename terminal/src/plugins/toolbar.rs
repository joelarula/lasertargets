use bevy::prelude::*;
use common::toolbar::{Docking, ItemState, ToolbarButton, ToolbarItem};
use std::collections::HashMap;

// Button color palette - traditional mild scheme for black background
mod button_colors {
    use bevy::prelude::Color;
    
    pub const PRESSED: Color = Color::srgba(0.25, 0.35, 0.45, 0.95);
    pub const ACTIVE_HOVERED: Color = Color::srgba(0.35, 0.55, 0.65, 0.9);
    pub const INACTIVE_HOVERED: Color = Color::srgba(0.45, 0.50, 0.55, 0.85);
    pub const ACTIVE: Color = Color::srgba(0.30, 0.48, 0.58, 0.85);
    pub const INACTIVE: Color = Color::srgba(0.35, 0.40, 0.45, 0.7);
}

pub struct ToolbarPlugin;

#[derive(Component)]
struct ToolbarContainer;

#[derive(Resource)]
struct NerdFont(Handle<Font>);

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, load_nerd_font)
            .add_systems(PostStartup, setup_toolbar)
            .add_systems(Update, (
                update_button_states,
                update_button_text,
                rebuild_toolbar,
            ).chain());
    }
}

fn load_nerd_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let nerd_font = asset_server.load("FiraCodeNerdFont-Regular.ttf");
    commands.insert_resource(NerdFont(nerd_font));
}

fn setup_toolbar(
    mut commands: Commands,
    items_query: Query<&ToolbarItem>,
    nerd_font: Option<Res<NerdFont>>
) {
    if let Some(font) = nerd_font {
        create_toolbar_ui(&mut commands, &items_query, Some(&font.0));
    } else {
        create_toolbar_ui(&mut commands, &items_query, None);
    }
}

fn rebuild_toolbar(
    mut commands: Commands,
    items_query: Query<&ToolbarItem>,
    toolbar_query: Query<Entity, With<ToolbarContainer>>,
    nerd_font: Option<Res<NerdFont>>,
    changed_items: Query<(), Or<(Changed<ToolbarItem>, Added<ToolbarItem>)>>,
) {

    if changed_items.is_empty() {
        return;
    }

    for toolbar_entity in toolbar_query.iter() {
        commands.entity(toolbar_entity).despawn();
    }
    
    if let Some(font) = nerd_font {
        create_toolbar_ui(&mut commands, &items_query, Some(&font.0));
    } else {
        create_toolbar_ui(&mut commands, &items_query, None);
    }
}

fn create_toolbar_ui(
    commands: &mut Commands,
    items_query: &Query<&ToolbarItem>,
    nerd_font: Option<&Handle<Font>>
) {
    let mut left_buttons = Vec::new();
    let mut right_buttons = Vec::new();
    let mut top_buttons = Vec::new();
    let mut bottom_buttons = Vec::new();
    let mut statusbar_buttons = Vec::new();

    // Build a hashmap for quick lookup and collect buttons by docking
    let mut buttons_map = HashMap::new();
    for item in items_query.iter() {
        buttons_map.insert(item.name.clone(), item.clone());
        match item.docking {
            Docking::Left => left_buttons.push((item.order, item.name.clone())),
            Docking::Right => right_buttons.push((item.order, item.name.clone())),
            Docking::Top => top_buttons.push((item.order, item.name.clone())),
            Docking::Bottom => bottom_buttons.push((item.order, item.name.clone())),
            Docking::StatusBar => statusbar_buttons.push((item.order, item.name.clone())),
        }
    }

    // Sort each group by order ascending
    left_buttons.sort_by_key(|(order, _)| *order);
    right_buttons.sort_by_key(|(order, _)| *order);
    top_buttons.sort_by_key(|(order, _)| *order);
    bottom_buttons.sort_by_key(|(order, _)| *order);
    statusbar_buttons.sort_by_key(|(order, _)| *order);

    // Extract just the names in sorted order
    let left_names: Vec<String> = left_buttons.into_iter().map(|(_, n)| n).collect();
    let right_names: Vec<String> = right_buttons.into_iter().map(|(_, n)| n).collect();
    let top_names: Vec<String> = top_buttons.into_iter().map(|(_, n)| n).collect();
    let bottom_names: Vec<String> = bottom_buttons.into_iter().map(|(_, n)| n).collect();
    let statusbar_names: Vec<String> = statusbar_buttons.into_iter().map(|(_, n)| n).collect();

    create_docked_toolbar(commands, &buttons_map, nerd_font, &left_names, Docking::Left);
    create_docked_toolbar(commands, &buttons_map, nerd_font, &right_names, Docking::Right);
    create_docked_toolbar(commands, &buttons_map, nerd_font, &top_names, Docking::Top);
    create_docked_toolbar(commands, &buttons_map, nerd_font, &bottom_names, Docking::Bottom);
    create_docked_toolbar(commands, &buttons_map, nerd_font, &statusbar_names, Docking::StatusBar);
}

fn create_docked_toolbar(
    commands: &mut Commands,
    buttons_map: &HashMap<String, ToolbarItem>,
    nerd_font: Option<&Handle<Font>>,
    button_names: &[String],
    docking: Docking,
) {
    if button_names.is_empty() {
        return;
    }
    
    let position_style = match docking {
        Docking::Left => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        Docking::Right => Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        Docking::Top => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        },
        Docking::Bottom => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(10.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            ..default()
        },
        Docking::StatusBar => Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            height: Val::Px(28.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(10.0)),
            column_gap: Val::Px(8.0),
            ..default()
        },
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
                if let Some(button_handler) = buttons_map.get(button_name) {
                    let initial_color = if button_handler.state == ItemState::On {
                        button_colors::ACTIVE
                    } else {
                        button_colors::INACTIVE
                    };
    
                    parent
                        .spawn((
                            ToolbarButton { name: button_name.clone() },
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
                                font_size: 24.0,
                                ..default()
                            },
                            TextColor(Color::WHITE)
                        ));
                }
            }
        });
}

fn update_button_states(
    mut button_query: Query<(&ToolbarButton, &Interaction, &mut BackgroundColor)>,
    items_query: Query<&ToolbarItem>,
) {
    // Build a quick lookup map
    let mut items_map = HashMap::new();
    for item in items_query.iter() {
        items_map.insert(item.name.clone(), item.state == ItemState::On);
    }

    // Update button colors based on item state
    for (button, interaction, mut background_color) in &mut button_query {
        let is_active = items_map.get(&button.name).copied().unwrap_or(false);

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

fn update_button_text(
    items_query: Query<&ToolbarItem, Changed<ToolbarItem>>,
    button_query: Query<(&ToolbarButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if items_query.is_empty() {
        return;
    }
    
    // Build a map of changed items
    let mut items_map = HashMap::new();
    for item in items_query.iter() {
        items_map.insert(item.name.clone(), item);
    }
    
    for (button, children) in &button_query {
        if let Some(button_data) = items_map.get(&button.name) {
            for child_entity in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child_entity) {
                    let new_text = button_data.icon.as_ref()
                        .unwrap_or(&button.name)
                        .clone();
                    **text = new_text;
                }
            }
        }
    }
}
