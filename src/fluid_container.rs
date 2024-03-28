use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec4 = Vec4::new(16., 9., 9., 0.);
const FLUID_CONTAINER_POSITION: Vec4 = Vec4::ZERO;


#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FluidContainerGizmo;


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidContainer {
    pub position: Vec4,
    pub size: Vec4,
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
            .add_systems(Startup, setup_gizmo_config)
            .add_systems(Update, draw_gizmos.in_set(InGameSet::EntityUpdates));
    }
}


fn setup_gizmo_config(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<FluidContainerGizmo>();
    config.line_width = 3.;  // Make it chunky
    // config.depth_bias = -1.;  // Draw on top of everything
}


fn draw_gizmos(mut border_gizmos: Gizmos<FluidContainerGizmo>, container: Res<FluidContainer>) {
    let transform = Transform::from_translation(container.position.xyz()).with_scale(container.size.xyz());
    border_gizmos.cuboid(transform, Color::WHITE);
}
