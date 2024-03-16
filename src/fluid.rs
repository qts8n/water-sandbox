use bevy::{prelude::*, sprite::{MaterialMesh2dBundle, Mesh2dHandle}};

use crate::smoothing;
use crate::helpers;
use crate::schedule::{InGameSet, PhysicsSet};
use crate::state::GameState;
use crate::fluid_container::FluidContainer;
use crate::gravity::Gravity;

const N_SIZE: usize = 50;
const STARTING_COLOR: Color = Color::rgb(0.16, 0.71, 0.97);

const PARTICLE_RADIUS: f32 = 0.05;
const PARTICLE_COLLISION_DAMPING: f32 = 0.3;
const PARTICLE_MASS: f32 = 1.;
const PARTICLE_SMOOTHING_RADIUS: f32 = 0.15;
const PARTICLE_TARGET_DENSITY: f32 = 10.;
const PARTICLE_PRESSURE_SCALAR: f32 = 30.;
const PARTICLE_NEAR_PRESSURE_SCALAR: f32 = 2.;


#[derive(Component, Default, Debug)]
pub struct Velocity {
    pub value: Vec2,
}


#[derive(Component, Default, Debug)]
pub struct Acceleration {
    pub value: Vec2,
}


#[derive(Component, Default, Debug)]
pub struct PressureForce {
    pub value: Vec2,
}


#[derive(Component, Default, Debug)]
pub struct PredictedPosition {
    pub value: Vec2,
}


#[derive(Bundle, Default)]
pub struct MovingObjectBundle {
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    pub pressure_force: PressureForce,
    pub predicted_position: PredictedPosition,
}


#[derive(Component, Default, Debug)]
pub struct Density {
    pub value: f32,
}


#[derive(Component, Default, Debug)]
pub struct NearDensity {
    pub value: f32,
}


#[derive(Component, Default, Debug)]
pub struct Pressure {
    pub value: f32,
}


#[derive(Component, Default, Debug)]
pub struct NearPressure {
    pub value: f32,
}


#[derive(Bundle, Default)]
pub struct FluidParticleBundle {
    density: Density,
    near_density: NearDensity,
    pressure: Pressure,
    near_pressure: NearPressure,
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
    moving_object_bundle: MovingObjectBundle,
}


#[derive(Resource, Debug)]
pub struct FluidParticleStaticProperties {
    pub radius: f32,
    pub collision_damping: f32,
    pub mass: f32,
    pub smoothing_radius: f32,
    pub target_density: f32,
    pub pressure_scalar: f32,
    pub near_pressure_scalar: f32,
}


impl Default for FluidParticleStaticProperties {
    fn default() -> Self {
        Self {
            radius: PARTICLE_RADIUS,
            collision_damping: PARTICLE_COLLISION_DAMPING,
            mass: PARTICLE_MASS,
            smoothing_radius: PARTICLE_SMOOTHING_RADIUS,
            target_density: PARTICLE_TARGET_DENSITY,
            pressure_scalar: PARTICLE_PRESSURE_SCALAR,
            near_pressure_scalar: PARTICLE_NEAR_PRESSURE_SCALAR,
        }
    }
}


#[derive(Component, Debug)]
pub struct FluidParticle;


pub struct FluidPlugin;


impl Plugin for FluidPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FluidParticleStaticProperties>()
            .add_systems(OnExit(GameState::Menu), spawn_liquid)
            .add_systems(OnEnter(GameState::GameOver), spawn_liquid)
            .add_systems(Update, despawn_liquid.in_set(InGameSet::DespawnEntities))
            .add_systems(FixedUpdate, integrate_positions.in_set(PhysicsSet::PositionUpdates))
            .add_systems(FixedUpdate, (
                update_density_and_pressure,
                update_pressure_force,
            ).chain().in_set(PhysicsSet::PropertyUpdates));
    }
}


fn spawn_liquid(
    mut commands: Commands,
    fluid_props: Res<FluidParticleStaticProperties>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    // container: Res<FluidContainer>,
) {
    let shape = Mesh2dHandle(meshes.add(Circle { radius: fluid_props.radius }));
    let material = materials.add(STARTING_COLOR);

    let points = helpers::cube_fluid(N_SIZE, N_SIZE, fluid_props.radius);

    // let (ext_min, ext_max) = container.get_extents();
    // let points = random_fluid(N_SIZE * N_SIZE, ext_min, ext_max);

    let particle_bundles: Vec<(FluidParticleBundle, FluidParticle)> = points.iter().map(|point| {(
        FluidParticleBundle {
            mesh_bundle: MaterialMesh2dBundle {
                mesh: shape.clone(),
                material: material.clone(),
                transform: Transform::from_xyz(point[0], point[1], 0.),
                ..default()
            },
            ..default()
        },
        FluidParticle,
    )}).collect();
    commands.spawn_batch(particle_bundles);
}


fn integrate_positions(
    mut query: Query<(&mut PredictedPosition, &mut Velocity, &mut Transform, &Acceleration), With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
    container: Res<FluidContainer>,
    gravity: Res<Gravity>,
    time: Res<Time<Fixed>>,
) {
    let (mut ext_min, mut ext_max) = container.get_extents();
    let rad_vec = Vec2::ONE * fluid_props.radius;
    ext_min += rad_vec;
    ext_max -= rad_vec;

    query.par_iter_mut().for_each(|(
        mut predicted_position,
        mut velocity,
        mut transform,
        acceleration,
    )| {
        velocity.value += (gravity.value + acceleration.value) / fluid_props.mass * time.delta_seconds();
        transform.translation += velocity.value.extend(0.) * time.delta_seconds();

        if transform.translation.x < ext_min.x {
            velocity.value.x *= -1. * fluid_props.collision_damping;
            transform.translation.x = ext_min.x;
        } else if transform.translation.x > ext_max.x {
            velocity.value.x *= -1. * fluid_props.collision_damping;
            transform.translation.x = ext_max.x;
        }

        if transform.translation.y < ext_min.y {
            velocity.value.y *= -1. * fluid_props.collision_damping;
            transform.translation.y = ext_min.y;
        } else if transform.translation.y > ext_max.y {
            velocity.value.y *= -1. * fluid_props.collision_damping;
            transform.translation.y = ext_max.y;
        }

        predicted_position.value = transform.translation.xy() + velocity.value * 1. / 120.;
    });
}


fn update_density_and_pressure(
    mut query: Query<(&mut Density, &mut NearDensity, &mut Pressure, &mut NearPressure, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<&PredictedPosition, With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(|(
        mut density,
        mut near_density,
        mut pressure,
        mut near_pressure,
        position,
    )| {
        let mut new_density = 0.;
        let mut new_near_density = 0.;

        for neighbor_position in neighbor_query.iter() {
            let distance = position.value.distance(neighbor_position.value);
            if distance > fluid_props.smoothing_radius {
                continue;
            }

            new_density += smoothing::smoothing_kernel(fluid_props.smoothing_radius, distance);
            new_near_density += smoothing::smoothing_kernel_near(fluid_props.smoothing_radius, distance);
        }

        density.value = fluid_props.mass * new_density + smoothing::DENSITY_PADDING;
        pressure.value = fluid_props.pressure_scalar * (density.value - fluid_props.target_density);

        near_density.value = fluid_props.mass * new_near_density + smoothing::DENSITY_PADDING;
        near_pressure.value = fluid_props.near_pressure_scalar * near_density.value;
    });
}


fn update_pressure_force(
    mut query: Query<(Entity, &mut PressureForce, &mut Acceleration, &Density, &Pressure, &NearPressure, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<(Entity, &Density, &NearDensity, &Pressure, &NearPressure, &PredictedPosition), With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(|(
        particle,
        mut pressure_force,
        mut acceleration,
        density,
        pressure,
        near_pressure,
        position,
    )| {
        let mut new_pressure_force = Vec2::ZERO;

        for (
            neighbor,
            neighbor_density,
            neighbor_near_density,
            neighbor_pressure,
            neighbor_near_pressure,
            neighbor_position
        ) in neighbor_query.iter() {
            if particle == neighbor {
                continue;
            }

            let mut direction = neighbor_position.value - position.value;
            let distance = direction.length();
            if distance > fluid_props.smoothing_radius {
                continue;
            }
            if distance > 0. {
                direction /= distance;
            } else {
                direction = Vec2::Y;
            }

            let slope = smoothing::smoothing_kernel_derivative(fluid_props.smoothing_radius, distance);
            let shared_pressure = (pressure.value + neighbor_pressure.value) / 2.;

            let slope_near = smoothing::smoothing_kernel_derivative_near(fluid_props.smoothing_radius, distance);
            let shared_pressure_near = (near_pressure.value + neighbor_near_pressure.value) / 2.;

            new_pressure_force += direction * shared_pressure * slope * fluid_props.mass / neighbor_density.value;
            new_pressure_force += direction * shared_pressure_near * slope_near * fluid_props.mass / neighbor_near_density.value;
        }
        pressure_force.value = new_pressure_force;
        acceleration.value = new_pressure_force / density.value;  // maybe div by density
    });
}


fn despawn_liquid(
    mut commands: Commands,
    query: Query<Entity, With<FluidParticle>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) {
        return;
    }

    for particle in query.iter() {
        let Some(particle_commands) = commands.get_entity(particle) else { continue };
        particle_commands.despawn_recursive();
    }

    next_state.set(GameState::GameOver);
}
