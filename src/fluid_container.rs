use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec2 = Vec2::new(32., 18.);
const FLUID_CONTAINER_POSITION: Vec2 = Vec2::ZERO;


#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FluidContainerGizmo;


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidContainer {
    pub position: Vec2,
    pub size: Vec2,
}


impl Default for FluidContainer {
    fn default() -> Self {
        Self {
            position: FLUID_CONTAINER_POSITION,
            size: FLUID_CONTAINER_SIZE,
        }
    }
}


pub struct GizmoPlugin;


impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_gizmo_group::<FluidContainerGizmo>()
            .init_resource::<FluidContainer>()
            .add_systems(Update, draw_gizmos.in_set(InGameSet::EntityUpdates));
    }
}


fn draw_gizmos(mut border_gizmos: Gizmos<FluidContainerGizmo>, container: Res<FluidContainer>) {
    border_gizmos.rect_2d(container.position, 0., container.size, Color::WHITE);
}
