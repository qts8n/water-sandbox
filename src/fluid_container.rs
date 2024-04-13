use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec3 = Vec3::new(16., 9., 9.);
const FLUID_CONTAINER_POSITION: Vec3 = Vec3::ZERO;
const FLUID_CONTAINER_ROTATOR_RADIUS: f32 = 2.;


#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FluidContainerGizmo;


#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidContainerTransform {
    pub world_to_local: Mat4,
    pub local_to_world: Mat4,
}


impl FluidContainerTransform {
    pub fn new(transform: Transform) -> Self {
        let matrix = transform.compute_matrix();
        Self {
            world_to_local: matrix.inverse(),
            local_to_world: matrix,
        }
    }
}


#[derive(Resource, Clone)]
pub struct FluidContainer {
    pub transform: Transform,
}


impl Default for FluidContainer {
    fn default() -> Self {
        Self {
            transform: Transform::from_translation(FLUID_CONTAINER_POSITION)
                .with_scale(FLUID_CONTAINER_SIZE),
        }
    }
}

impl FluidContainer {
    pub fn get_transform(&self) -> FluidContainerTransform {
        FluidContainerTransform::new(self.transform)
    }
}


#[derive(Resource, Clone)]
pub struct FluidContainerRotator {
    pub position: Vec3,
    pub radius: f32,
}


impl Default for FluidContainerRotator {
    fn default() -> Self {
        Self {
            position: FLUID_CONTAINER_POSITION,
            radius: FLUID_CONTAINER_ROTATOR_RADIUS,
        }
    }
}


pub struct GizmoPlugin;


impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_gizmo_group::<FluidContainerGizmo>()
            .init_resource::<FluidContainer>()
            .init_resource::<FluidContainerRotator>()
            .add_systems(Startup, setup_gizmo_config)
            .add_systems(Update, draw_gizmos.in_set(InGameSet::EntityUpdates));
    }
}


fn setup_gizmo_config(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<FluidContainerGizmo>();
    config.line_width = 3.;  // Make it chunky
    config.depth_bias = -1.;  // Draw on top of everything
}


fn draw_gizmos(
    mut fluid_container_gizmos: Gizmos<FluidContainerGizmo>,
    container: Res<FluidContainer>,
    rotator: Res<FluidContainerRotator>,
) {
    fluid_container_gizmos.cuboid(container.transform, Color::WHITE);
    fluid_container_gizmos.circle(rotator.position, Direction3d::X, rotator.radius, Color::RED);
    fluid_container_gizmos.circle(rotator.position, Direction3d::Y, rotator.radius, Color::GREEN);
    fluid_container_gizmos.circle(rotator.position, Direction3d::Z, rotator.radius, Color::BLUE);
}
