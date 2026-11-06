use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::DerefMut;

/// The clickable UI node drawn for a [`ToolbarItem`]; `item` is the entity holding that item
#[derive(Component, Debug, Clone)]
pub struct ToolbarButton {
    pub name: String,
    pub item: Entity,
}

/// Triggered on a [`ToolbarItem`] entity when its button is pressed. React with
/// `commands.spawn(item).observe(|_: On<ToolbarPressed>, ...| ...)`.
#[derive(EntityEvent, Debug, Clone, Copy)]
pub struct ToolbarPressed {
    pub entity: Entity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Disabled,
    On,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Docking {
    Left,
    Right,
    Top,
    Bottom,
    StatusBar,
}

/// Component that defines a toolbar button item.
/// Spawn this as an entity in your plugin to create a toolbar button.
#[derive(Component, Clone, Debug)]
pub struct ToolbarItem {
    pub name: String,
    pub order: u8,
    pub icon: Option<String>,
    pub text: Option<String>,
    pub state: ItemState,
    pub docking: Docking,
    pub button_size: f32,
    pub margin_before: f32,
    pub margin_after: f32,
}

impl Default for ToolbarItem {
    fn default() -> Self {
        Self {
            name: String::new(),
            order: 0,
            icon: None,
            text: None,
            state: ItemState::Off,
            docking: Docking::Left,
            button_size: 36.0,
            margin_before: 0.0,
            margin_after: 0.0,
        }
    }
}

impl ToolbarItem {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..default() }
    }

    /// A Nerd Font glyph shown on the button
    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    /// Tooltip text shown on hover
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn order(mut self, order: u8) -> Self {
        self.order = order;
        self
    }

    pub fn dock(mut self, docking: Docking) -> Self {
        self.docking = docking;
        self
    }

    pub fn state(mut self, state: ItemState) -> Self {
        self.state = state;
        self
    }

    pub fn on(self, on: bool) -> Self {
        self.state(if on { ItemState::On } else { ItemState::Off })
    }
}

/// Ties a toolbar item to the bool resource `R` it toggles
#[derive(Component)]
struct ToggleOf<R>(PhantomData<R>);

pub trait ToolbarAppExt {
    /// Adds a toolbar button that flips the bool resource `R` and lights up while it is true.
    /// Anything else may change `R` too (a window's close button); the button follows.
    fn add_toolbar_toggle<R>(&mut self, item: ToolbarItem) -> &mut Self
    where
        R: Resource + Default + DerefMut<Target = bool>;
}

impl ToolbarAppExt for App {
    fn add_toolbar_toggle<R>(&mut self, item: ToolbarItem) -> &mut Self
    where
        R: Resource + Default + DerefMut<Target = bool>,
    {
        self.init_resource::<R>()
            .add_systems(Startup, move |mut commands: Commands| {
                commands
                    .spawn((item.clone(), ToggleOf::<R>(PhantomData)))
                    .observe(|_: On<ToolbarPressed>, mut value: ResMut<R>| {
                        **value = !**value;
                    });
            })
            .add_systems(Update, sync_toggle::<R>.run_if(resource_changed::<R>))
    }
}

/// Run condition: the bool resource `R` is true (`.run_if(toggled_on::<SettingsOpen>)`)
pub fn toggled_on<R: Resource + std::ops::Deref<Target = bool>>(value: Res<R>) -> bool {
    **value
}

fn sync_toggle<R: Resource + DerefMut<Target = bool>>(
    value: Res<R>,
    mut items: Query<&mut ToolbarItem, With<ToggleOf<R>>>,
) {
    let state = if **value { ItemState::On } else { ItemState::Off };
    for mut item in &mut items {
        // Comparing first keeps an unchanged item from triggering a toolbar rebuild
        if item.state != state {
            item.state = state;
        }
    }
}

// Button color palette - traditional mild scheme for black background
pub mod button_colors {
    use bevy::prelude::Color;

    pub const PRESSED: Color = Color::srgba(0.25, 0.35, 0.45, 0.95);
    pub const ACTIVE_HOVERED: Color = Color::srgba(0.35, 0.55, 0.65, 0.9);
    pub const INACTIVE_HOVERED: Color = Color::srgba(0.45, 0.50, 0.55, 0.85);
    pub const ACTIVE: Color = Color::srgba(0.30, 0.48, 0.58, 0.85);
    pub const INACTIVE: Color = Color::srgba(0.35, 0.40, 0.45, 0.7);

    // Connection button custom colors
    pub const CONN_CONNECTED: Color = Color::srgba(0.12, 0.65, 0.28, 0.85);
    pub const CONN_CONNECTED_HOVER: Color = Color::srgba(0.16, 0.80, 0.35, 0.95);
    pub const CONN_DISCONNECTED: Color = Color::srgba(0.38, 0.40, 0.44, 0.75);
    pub const CONN_DISCONNECTED_HOVER: Color = Color::srgba(0.50, 0.53, 0.58, 0.88);
}

pub struct ToolbarPlugin;

#[derive(Component)]
struct ToolbarContainer;

#[derive(Component)]
pub struct ToolbarTooltipMarker;

#[derive(Resource)]
struct NerdFont(Handle<Font>);

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_nerd_font)
            .add_systems(PostStartup, setup_toolbar)
            .add_systems(
                Update,
                (
                    trigger_presses,
                    update_button_states,
                    update_button_text,
                    update_hover_tooltips,
                    rebuild_toolbar,
                )
                    .chain(),
            );
    }
}

fn load_nerd_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    let nerd_font = asset_server.load("FiraCodeNerdFont-Regular.ttf");
    commands.insert_resource(NerdFont(nerd_font));
}

fn setup_toolbar(
    mut commands: Commands,
    items_query: Query<(Entity, &ToolbarItem)>,
    nerd_font: Option<Res<NerdFont>>,
) {
    if let Some(font) = nerd_font {
        create_toolbar_ui(&mut commands, &items_query, Some(&font.0));
    } else {
        create_toolbar_ui(&mut commands, &items_query, None);
    }
}

fn rebuild_toolbar(
    mut commands: Commands,
    items_query: Query<(Entity, &ToolbarItem)>,
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
    items_query: &Query<(Entity, &ToolbarItem)>,
    nerd_font: Option<&Handle<Font>>,
) {
    let mut left_buttons = Vec::new();
    let mut right_buttons = Vec::new();
    let mut top_buttons = Vec::new();
    let mut bottom_buttons = Vec::new();
    let mut statusbar_buttons = Vec::new();

    let mut buttons_map = HashMap::new();
    for (entity, item) in items_query.iter() {
        buttons_map.insert(item.name.clone(), (entity, item.clone()));
        match item.docking {
            Docking::Left => left_buttons.push((item.order, item.name.clone())),
            Docking::Right => right_buttons.push((item.order, item.name.clone())),
            Docking::Top => top_buttons.push((item.order, item.name.clone())),
            Docking::Bottom => bottom_buttons.push((item.order, item.name.clone())),
            Docking::StatusBar => statusbar_buttons.push((item.order, item.name.clone())),
        }
    }

    left_buttons.sort_by_key(|(order, _)| *order);
    right_buttons.sort_by_key(|(order, _)| *order);
    top_buttons.sort_by_key(|(order, _)| *order);
    bottom_buttons.sort_by_key(|(order, _)| *order);
    statusbar_buttons.sort_by_key(|(order, _)| *order);

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
    buttons_map: &HashMap<String, (Entity, ToolbarItem)>,
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
                if let Some((item, button_handler)) = buttons_map.get(button_name) {
                    let initial_color = if button_handler.state == ItemState::On {
                        button_colors::ACTIVE
                    } else {
                        button_colors::INACTIVE
                    };

                    parent
                        .spawn((
                            ToolbarButton {
                                name: button_name.clone(),
                                item: *item,
                            },
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
                                button_handler
                                    .icon
                                    .as_ref()
                                    .unwrap_or(button_name)
                                    .clone(),
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
                            TextColor(Color::WHITE),
                        ));
                }
            }
        });
}

/// Tells the pressed button's item entity, so its plugin's observer runs
fn trigger_presses(
    mut commands: Commands,
    buttons: Query<(&Interaction, &ToolbarButton), Changed<Interaction>>,
) {
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            commands.trigger(ToolbarPressed { entity: button.item });
        }
    }
}

fn update_button_states(
    mut button_query: Query<(&ToolbarButton, &Interaction, &mut BackgroundColor)>,
    items_query: Query<&ToolbarItem>,
) {
    let mut items_map = HashMap::new();
    for item in items_query.iter() {
        items_map.insert(item.name.clone(), item.state == ItemState::On);
    }

    for (button, interaction, mut background_color) in &mut button_query {
        let is_active = items_map.get(&button.name).copied().unwrap_or(false);

        if button.name == "connection_status" {
            match *interaction {
                Interaction::Pressed => {
                    *background_color = BackgroundColor(button_colors::PRESSED);
                }
                Interaction::Hovered => {
                    if is_active {
                        *background_color = BackgroundColor(button_colors::CONN_CONNECTED_HOVER);
                    } else {
                        *background_color = BackgroundColor(button_colors::CONN_DISCONNECTED_HOVER);
                    }
                }
                Interaction::None => {
                    if is_active {
                        *background_color = BackgroundColor(button_colors::CONN_CONNECTED);
                    } else {
                        *background_color = BackgroundColor(button_colors::CONN_DISCONNECTED);
                    }
                }
            }
        } else {
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
}

fn update_button_text(
    items_query: Query<&ToolbarItem, Changed<ToolbarItem>>,
    button_query: Query<(&ToolbarButton, &Children)>,
    mut text_query: Query<&mut Text>,
) {
    if items_query.is_empty() {
        return;
    }

    let mut items_map = HashMap::new();
    for item in items_query.iter() {
        items_map.insert(item.name.clone(), item);
    }

    for (button, children) in &button_query {
        if let Some(button_data) = items_map.get(&button.name) {
            for child_entity in children.iter() {
                if let Ok(mut text) = text_query.get_mut(child_entity) {
                    let new_text = button_data
                        .icon
                        .as_ref()
                        .unwrap_or(&button.name)
                        .clone();
                    **text = new_text;
                }
            }
        }
    }
}

fn update_hover_tooltips(
    mut commands: Commands,
    tooltip_query: Query<Entity, With<ToolbarTooltipMarker>>,
    button_query: Query<(&ToolbarButton, &Interaction)>,
    items_query: Query<&ToolbarItem>,
) {
    let mut hovered_text: Option<String> = None;

    for (button, interaction) in button_query.iter() {
        if *interaction == Interaction::Hovered {
            for item in items_query.iter() {
                if item.name == button.name {
                    if let Some(text) = &item.text {
                        hovered_text = Some(text.clone());
                    }
                    break;
                }
            }
        }
    }

    for entity in tooltip_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(text) = hovered_text {
        commands
            .spawn((
                ToolbarTooltipMarker,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(60.0),
                    top: Val::Px(16.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.95)),
                BorderRadius::all(Val::Px(6.0)),
                ZIndex(2000),
            ))
            .with_child((
                Text::new(text),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.95, 1.0)),
            ));
    }
}

