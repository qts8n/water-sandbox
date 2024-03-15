use bevy::prelude::*;

use crate::schedule::InGameSet;
use crate::gravity::Gravity;
use crate::fluid::FluidParticleStaticProperties;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const TEXT_FONT_SIZE: f32 = 20.;

const FLUID_PROPS_CHANGE_STEP: f32 = 0.05;


#[derive(Component, Debug)]
pub struct HudItem;


#[derive(Component, Debug)]
pub struct PressureHudItem;


#[derive(Component, Debug)]
pub struct NearPressureHudItem;


#[derive(Component, Debug)]
pub struct TargetDensityHudItem;


#[derive(Component, Debug)]
pub struct SmoothingRadiusHudItem;


#[derive(Component, Debug)]
pub struct GravityHudItem;


pub struct HudPlugin;


impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                update_fluid_props,
                (
                    update_pressure_in_hud,
                    update_near_pressure_in_hud,
                    update_target_density_in_hud,
                    update_smoothing_radius_in_hud,
                    update_gravity_in_hud,
                ),
            ).chain().in_set(InGameSet::EntityUpdates))
            .add_systems(Startup, setup_hud);
    }
}


fn setup_hud(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceAround,
                ..default()
            },
            ..default()
        },
        HudItem,
    )).with_children(|parent| {
        parent.spawn((
            TextBundle::from_section("P: 0", TextStyle {
                font_size: TEXT_FONT_SIZE,
                color: TEXT_COLOR,
                ..default()
            }),
            PressureHudItem,
        ));
        parent.spawn((
            TextBundle::from_section("nP: 0", TextStyle {
                font_size: TEXT_FONT_SIZE,
                color: TEXT_COLOR,
                ..default()
            }),
            NearPressureHudItem,
        ));
        parent.spawn((
            TextBundle::from_section("tD: 0", TextStyle {
                font_size: TEXT_FONT_SIZE,
                color: TEXT_COLOR,
                ..default()
            }),
            TargetDensityHudItem,
        ));
        parent.spawn((
            TextBundle::from_section("Smoothing Radius: 0", TextStyle {
                font_size: TEXT_FONT_SIZE,
                color: TEXT_COLOR,
                ..default()
            }),
            SmoothingRadiusHudItem,
        ));
        parent.spawn((
            TextBundle::from_section("Gravity: 0", TextStyle {
                font_size: TEXT_FONT_SIZE,
                color: TEXT_COLOR,
                ..default()
            }),
            GravityHudItem,
        ));
    });
}


fn update_fluid_props(
    mut fluid_props: ResMut<FluidParticleStaticProperties>,
    mut gravity: ResMut<Gravity>,
    keyboard_input: Res<ButtonInput<KeyCode>>
) {
    if keyboard_input.pressed(KeyCode::Digit1) && fluid_props.smoothing_radius - FLUID_PROPS_CHANGE_STEP > 0. {
        fluid_props.smoothing_radius -= FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::Digit2) {
        fluid_props.smoothing_radius += FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::Digit3) {
        gravity.value.y += FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::Digit4) {
        gravity.value.y -= FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyQ) {
        fluid_props.pressure_scalar -= FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyW) {
        fluid_props.pressure_scalar += FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyA) {
        fluid_props.near_pressure_scalar -= FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyS) {
        fluid_props.near_pressure_scalar += FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyZ) {
        fluid_props.target_density -= FLUID_PROPS_CHANGE_STEP;
    } else if keyboard_input.pressed(KeyCode::KeyX) {
        fluid_props.target_density += FLUID_PROPS_CHANGE_STEP;
    }

}


fn update_pressure_in_hud(mut query: Query<&mut Text, With<PressureHudItem>>, fluid_props: Res<FluidParticleStaticProperties>) {
    let Ok(mut pressure_hud_item) = query.get_single_mut() else { return };
    if pressure_hud_item.sections.is_empty() {
        return;
    }
    pressure_hud_item.sections[0].value = format!("P: {:.3}", fluid_props.pressure_scalar);
}


fn update_near_pressure_in_hud(mut query: Query<&mut Text, With<NearPressureHudItem>>, fluid_props: Res<FluidParticleStaticProperties>) {
    let Ok(mut near_pressure_hud_item) = query.get_single_mut() else { return };
    if near_pressure_hud_item.sections.is_empty() {
        return;
    }
    near_pressure_hud_item.sections[0].value = format!("nP: {:.3}", fluid_props.near_pressure_scalar);
}


fn update_target_density_in_hud(mut query: Query<&mut Text, With<TargetDensityHudItem>>, fluid_props: Res<FluidParticleStaticProperties>) {
    let Ok(mut target_density_hud_item) = query.get_single_mut() else { return };
    if target_density_hud_item.sections.is_empty() {
        return;
    }
    target_density_hud_item.sections[0].value = format!("tD: {:.3}", fluid_props.target_density);
}


fn update_smoothing_radius_in_hud(mut query: Query<&mut Text, With<SmoothingRadiusHudItem>>, fluid_props: Res<FluidParticleStaticProperties>) {
    let Ok(mut smoothing_radius_hud_item) = query.get_single_mut() else { return };
    if smoothing_radius_hud_item.sections.is_empty() {
        return;
    }
    smoothing_radius_hud_item.sections[0].value = format!("Smoothing Radius: {:.3}", fluid_props.smoothing_radius);
}


fn update_gravity_in_hud(mut query: Query<&mut Text, With<GravityHudItem>>, gravity: Res<Gravity>) {
    let Ok(mut gravity_hud_item) = query.get_single_mut() else { return };
    if gravity_hud_item.sections.is_empty() {
        return;
    }
    gravity_hud_item.sections[0].value = format!("Gravity: {:.3}", -gravity.value.y);
}
