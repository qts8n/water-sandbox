use bevy::prelude::*;
use bevy::render::camera::ScalingMode;

use crate::fluid_container::FluidContainer;
use crate::schedule::InGameSet;

const CAMERA_ZOOM_STEP: f32 = 0.1;  // 10% step


#[derive(Component, Debug)]
pub struct Observer;


pub struct CameraPlugin;


impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera)
            .add_systems(Update, update_camera_zoom.in_set(InGameSet::UserInput));
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
