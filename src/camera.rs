use bevy::prelude::*;
use bevy::render::camera::ScalingMode;
use bevy::window::PrimaryWindow;

use crate::fluid_container::FluidContainer;
use crate::schedule::InGameSet;

const CAMERA_ZOOM_STEP: f32 = 0.1;  // 10% step

const CURSOR_RADIUS: f32 = 2.;
const CURSOR_FORCE: f32 = 20.;


#[derive(PartialEq, Default, Debug)]
pub enum WorldCursorAction {
    #[default]
    Idle,
    Inward,
    Outward,
}


#[derive(Resource, Debug)]
pub struct WorldCursor {
    pub position: Vec2,
    pub radius: f32,
    pub force: f32,
    pub action: WorldCursorAction,
}


impl Default for WorldCursor {
    fn default() -> Self {
        Self {
            radius: CURSOR_RADIUS,
            force: CURSOR_FORCE,
            position: Vec2::default(),
            action: WorldCursorAction::default(),
        }
    }
}


#[derive(Component, Debug)]
pub struct Observer;


pub struct CameraPlugin;


impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<WorldCursor>()
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, (
                update_camera_zoom,
                update_cursor,
            ).in_set(InGameSet::UserInput));
    }
}


fn spawn_camera(mut commands: Commands, container: Res<FluidContainer>) {
    let offset = (container.size.y / 10.).round();  // 10% margin
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(container.size.y + offset);
    commands.spawn((camera_bundle, Observer));
}


fn update_camera_zoom(
    mut query: Query<&mut OrthographicProjection, With<Observer>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut projection) = query.get_single_mut() else { return };


    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        // Zoom in
        projection.scale /= 1. + CAMERA_ZOOM_STEP;
    } else if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        // Zoom out
        projection.scale *= 1. + CAMERA_ZOOM_STEP;
    }
}


fn update_cursor(
    mut cursor_position: ResMut<WorldCursor>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Observer>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    cursor_position.action = WorldCursorAction::Idle;

    let Ok((camera, transform)) = camera_query.get_single() else { return };
    let Ok(window) = window_query.get_single() else { return };

    if let Some(world_position) = window.cursor_position()
        .and_then(|cursor| camera.viewport_to_world(transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        cursor_position.position = world_position;
        // eprintln!("World coords: {}/{}", world_position.x, world_position.y);
    } else {
        return;
    }

    if mouse_input.pressed(MouseButton::Left) {
        cursor_position.action = WorldCursorAction::Inward;
    } else if mouse_input.pressed(MouseButton::Right) {
        cursor_position.action = WorldCursorAction::Outward;
    }
}
