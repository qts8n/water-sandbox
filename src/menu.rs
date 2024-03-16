use bevy::{app::AppExit, prelude::*};

use crate::state::GameState;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.35, 0.35);


#[derive(Component, Debug)]
pub struct MainMenuItem;


#[derive(Component, Debug)]
enum MenuButtonAction {
    Play,
    Quit,
}


pub struct MenuPlugin;


impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_menu)
            .add_systems(OnExit(GameState::Menu), despawn_menu)
            .add_systems(Update, (button_system, menu_action).chain());
    }
}


fn setup_menu(mut commands: Commands) {
    // Common style for all buttons on the screen
    let button_style = Style {
        width: Val::Px(250.0),
        height: Val::Px(65.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_text_style = TextStyle {
        font_size: 40.0,
        color: TEXT_COLOR,
        ..default()
    };

    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        MainMenuItem,
    )).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        }).with_children(|parent| {
            // Display the game name
            parent.spawn(TextBundle::from_section(
            "Fluid Simulation",
            TextStyle {
                font_size: 80.0,
                color: TEXT_COLOR,
                ..default()
            }).with_style(Style {
                margin: UiRect::all(Val::Px(50.0)),
                ..default()
            }));

            // Display two buttons for each action available from the main menu:
            // - start
            // - quit
            parent.spawn((
                ButtonBundle {
                    style: button_style.clone(),
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                },
                MenuButtonAction::Play,
            )).with_children(|parent| {
                parent.spawn(TextBundle::from_section("Start", button_text_style.clone()));
            });
            parent.spawn((
                ButtonBundle {
                    style: button_style,
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                },
                MenuButtonAction::Quit,
            )).with_children(|parent| {
                parent.spawn(TextBundle::from_section("Quit", button_text_style));
            });
        });
    });
}


// This system handles changing all buttons color based on mouse interaction
fn button_system(mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>) {
    for (interaction, mut color) in query.iter_mut() {
        *color = match *interaction {
            Interaction::Pressed => PRESSED_BUTTON.into(),
            Interaction::Hovered => HOVERED_BUTTON.into(),
            _ => NORMAL_BUTTON.into(),
        }
    }
}


fn menu_action(
    query: Query<(&Interaction, &MenuButtonAction), (Changed<Interaction>, With<Button>)>,
    mut app_exit_events: EventWriter<AppExit>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, menu_button_action) in query.iter() {
        if *interaction == Interaction::Pressed {
            match menu_button_action {
                MenuButtonAction::Quit => { app_exit_events.send(AppExit); },
                MenuButtonAction::Play => { next_state.set(GameState::InGame); },
            }
        }
    }
}


fn despawn_menu(mut commands: Commands, query: Query<Entity, With<MainMenuItem>>) {
    for entity in query.iter() {
        if let Some(entity_commands) = commands.get_entity(entity) {
            entity_commands.despawn_recursive();
        }
    }
}
