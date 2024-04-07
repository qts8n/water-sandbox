use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec3 = Vec3::new(16., 9., 9.);
const FLUID_CONTAINER_POSITION: Vec3 = Vec3::ZERO;
const FLUID_CONTAINER_ROTATOR_RADIUS: f32 = 2.;
const FLUID_CONTAINER_ROTATOR_THICKNESS: f32 = 0.2;


#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FluidContainerGizmo;


#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidContainerExt {
    pub ext_min: Vec4,
    pub ext_max: Vec4,
}


#[derive(Resource, Clone)]
pub struct FluidContainer {
    pub position: Vec3,
    pub size: Vec3,
}


impl Default for FluidContainer {
    fn default() -> Self {
        Self {
            position: FLUID_CONTAINER_POSITION,
            size: FLUID_CONTAINER_SIZE,
        }
    }
}

impl FluidContainer {
    pub fn get_ext(&self, padding: f32) -> FluidContainerExt {
        let half_size = self.size / 2.;
        let ext_min = (self.position - half_size + padding).extend(0.);
        let ext_max = (self.position + half_size - padding).extend(0.);
        FluidContainerExt {
            ext_min,
            ext_max,
        }
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
            .add_systems(Startup, (setup_gizmo, setup_gizmo_config))
            .add_systems(Update, draw_gizmos.in_set(InGameSet::EntityUpdates));
    }
}


fn setup_gizmo(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rotator: Res<FluidContainerRotator>,
) {
    let thickness_padding = FLUID_CONTAINER_ROTATOR_THICKNESS / 2.;
    let shape = meshes.add(Torus::new(
        rotator.radius - thickness_padding,
        rotator.radius + thickness_padding
    ));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        ..default()
    });

    let horizontal_target_anchor = rotator.position + Vec3::X;
    let horizontal_up_anchor = rotator.position + Vec3::Y;
    let transform_horizontal = Transform::from_translation(rotator.position)
        .looking_at(horizontal_target_anchor, horizontal_up_anchor);

    let vertical_target_anchor = rotator.position + Vec3::X;
    let vertical_up_anchor = rotator.position + Vec3::Z;
    let transform_vertical = Transform::from_translation(rotator.position)
        .looking_at(vertical_target_anchor, vertical_up_anchor);

    commands.spawn_batch([
        PbrBundle {
            mesh: shape.clone(),
            material: material.clone(),
            transform: transform_horizontal,
            ..default()
        },
        PbrBundle {
            mesh: shape.clone(),
            material: material.clone(),
            transform: transform_vertical,
            ..default()
        },
    ]);
}


fn setup_gizmo_config(mut config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = config_store.config_mut::<FluidContainerGizmo>();
    config.line_width = 3.;  // Make it chunky
    config.depth_bias = -1.;  // Draw on top of everything
}


fn draw_gizmos(
    mut border_gizmos: Gizmos<FluidContainerGizmo>,
    container: Res<FluidContainer>,
) {
    let transform = Transform::from_translation(container.position).with_scale(container.size);
    border_gizmos.cuboid(transform, Color::WHITE);
}
