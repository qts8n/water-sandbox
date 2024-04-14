use bevy::prelude::*;
use bevy::core::Pod;

use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec3 = Vec3::new(16., 9., 9.);
const FLUID_CONTAINER_POSITION: Vec3 = Vec3::ZERO;
const FLUID_CONTAINER_ROTATOR_RADIUS: f32 = 2.;
const FLUID_CONTAINER_ROTATOR_THICKNESS: f32 = 0.1;
const FLUID_CONTAINER_ROTATOR_STEP: f32 = 0.005;


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


#[derive(PartialEq, Clone)]
pub enum FluidContainerRotatorState {
    Idle,
    AroundX,
    AroundY,
    AroundZ,
}


#[derive(Resource, Clone)]
pub struct FluidContainerRotator {
    pub position: Vec3,
    pub radius: f32,
    pub thickness: f32,
    pub step: f32,
    pub state: FluidContainerRotatorState,
}


impl Default for FluidContainerRotator {
    fn default() -> Self {
        Self {
            position: FLUID_CONTAINER_POSITION,
            radius: FLUID_CONTAINER_ROTATOR_RADIUS,
            thickness: FLUID_CONTAINER_ROTATOR_THICKNESS,
            step: FLUID_CONTAINER_ROTATOR_STEP,
            state: FluidContainerRotatorState::Idle,
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
            .add_systems(Update, (
                change_rotator_state,
                rotate_container,
            ).in_set(InGameSet::UserInput))
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

    let mut color_x = Color::WHITE;
    let mut color_y = Color::WHITE;
    let mut color_z = Color::WHITE;
    match rotator.state {
        FluidContainerRotatorState::AroundX => {
            color_x = Color::RED;
        },
        FluidContainerRotatorState::AroundY => {
            color_y = Color::GREEN;
        },
        FluidContainerRotatorState::AroundZ => {
            color_z = Color::BLUE;
        },
        _ => (),
    }
    fluid_container_gizmos.circle(rotator.position, Direction3d::X, rotator.radius, color_x);
    fluid_container_gizmos.circle(rotator.position, Direction3d::Y, rotator.radius, color_y);
    fluid_container_gizmos.circle(rotator.position, Direction3d::Z, rotator.radius, color_z);
}


fn change_rotator_state(
    mut rotator: ResMut<FluidContainerRotator>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::ShiftLeft) {
        rotator.state = match rotator.state {
            FluidContainerRotatorState::Idle => FluidContainerRotatorState::AroundX,
            FluidContainerRotatorState::AroundX => FluidContainerRotatorState::AroundY,
            FluidContainerRotatorState::AroundY => FluidContainerRotatorState::AroundZ,
            FluidContainerRotatorState::AroundZ => FluidContainerRotatorState::Idle,
        };
    }
}


fn rotate_container(
    mut container: ResMut<FluidContainer>,
    rotator: Res<FluidContainerRotator>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if rotator.state == FluidContainerRotatorState::Idle {
        return;
    }

    let mut angle = rotator.step;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        angle *= -1.;
    } else if keyboard_input.pressed(KeyCode::ArrowRight) {
        // angle *= 1.;
    } else {
        return;
    }

    match rotator.state {
        FluidContainerRotatorState::AroundX => {
            container.transform.rotate_local_x(angle);
        },
        FluidContainerRotatorState::AroundY => {
            container.transform.rotate_local_y(angle);
        },
        FluidContainerRotatorState::AroundZ => {
            container.transform.rotate_local_z(angle);
        },
        _ => (),
    }
}
