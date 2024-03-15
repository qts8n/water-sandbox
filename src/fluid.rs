use std::f32::consts::PI;

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle}
};
use rand::Rng;

use crate::schedule::{InGameSet, PhysicsSet};
use crate::state::GameState;
use crate::fluid_container::FluidContainer;
use crate::gravity::Gravity;

const N_SIZE: usize = 50;
const STARTING_COLOR: Color = Color::rgb(0.16, 0.71, 0.97);

const PARTICLE_RADIUS: f32 = 0.05;
const PARTICLE_COLLISION_DAMPING: f32 = 0.95;
const PARTICLE_MASS: f32 = 1.;
const PARTICLE_SMOOTHING_RADIUS: f32 = 0.1;
const PARTICLE_TARGET_DENSITY: f32 = 6.;
const PARTICLE_PRESSURE_SCALAR: f32 = 10.;
const PARTICLE_NEAR_PRESSURE_SCALAR: f32 = 1.;


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
            .add_systems(Startup, spawn_liquid)
            .add_systems(OnEnter(GameState::GameOver), spawn_liquid)
            .add_systems(Update, despawn_liquid.in_set(InGameSet::DespawnEntities))
            .add_systems(FixedUpdate, (
                apply_external_forces,
                update_predicted_positions,
                update_density_and_pressure,
                update_pressure_force,
                update_acceleration_and_velocity,
            ).chain().in_set(PhysicsSet::PropertyUpdates))
            .add_systems(FixedUpdate, (
                update_position,
                update_velocity_on_collision,
            ).chain().in_set(PhysicsSet::PositionUpdates));
    }
}


pub fn cube_fluid(ni: usize, nj: usize, particle_rad: f32) -> Vec<Vec2> {
    let mut points = Vec::new();
    let half_extents = Vec2::new(ni as f32, nj as f32) * particle_rad;
    let offset = Vec2::new(particle_rad, particle_rad) - half_extents;
    let diam = particle_rad * 2.;
    for i in 0..ni {
        let x = (i as f32) * diam;
        for j in 0..nj {
            let y = (j as f32) * diam;
            points.push(Vec2::new(x, y) + offset);
        }
    }

    points
}


pub fn random_fluid(nparticles: usize, ext_min: Vec2, ext_max: Vec2) -> Vec<Vec2> {
    let mut points = Vec::new();
    let mut rng = rand::thread_rng();
    for _ in 0..nparticles {
        points.push(Vec2::new(rng.gen_range(ext_min.x..ext_max.x), rng.gen_range(ext_min.y..ext_max.y)));
    }

    points
}


fn spawn_liquid(
    mut commands: Commands,
    fluid_props: Res<FluidParticleStaticProperties>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    container: Res<FluidContainer>,
) {
    let shape = Mesh2dHandle(meshes.add(Circle { radius: fluid_props.radius }));
    let material = materials.add(STARTING_COLOR);
    let mut rng = rand::thread_rng();
    let coin_flip = rng.gen_bool(0.5);
    let points: Vec<Vec2>;
    if coin_flip {
        points = cube_fluid(N_SIZE, N_SIZE, fluid_props.radius);
    } else {
        let (ext_min, ext_max) = container.get_extents();
        points = random_fluid(N_SIZE * N_SIZE, ext_min, ext_max);
    }
    let particle_bundles: Vec<(FluidParticleBundle, FluidParticle)> = points.iter().map(|point| {
        (
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
        )
    }).collect();
    commands.spawn_batch(particle_bundles);
}


fn apply_external_forces(
    mut query: Query<&mut Velocity, With<FluidParticle>>,
    gravity: Res<Gravity>,
    time: Res<Time<Fixed>>,
) {
    query.par_iter_mut().for_each(|mut velocity| {
        velocity.value += gravity.value * time.delta_seconds();
    });
}


fn update_predicted_positions(mut query: Query<(&mut PredictedPosition, &Velocity, &Transform), With<FluidParticle>>) {
    query.par_iter_mut().for_each(|(mut predicted_position, velocity, transform)| {
        predicted_position.value = transform.translation.xy() + velocity.value * 1. / 120.;
    });
}


pub fn smoothing_kernel(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }
    let volume = 6. / (PI * radius.powi(4));
    let v = radius - distance;
    v * v * volume
}


pub fn smoothing_kernel_near(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }
    let volume = 10. / (PI * radius.powi(5));
    let v = radius - distance;
    v * v * v * volume
}


pub fn convert_density_to_pressure(density: f32, target_density: f32, pressure_scalar: f32) -> f32 {
    let density_error = density - target_density;
    density_error * pressure_scalar
}


pub fn convert_near_density_to_near_pressure(near_density: f32, near_pressure_scalar: f32) -> f32 {
    near_density * near_pressure_scalar
}


fn update_density_and_pressure(
    mut query: Query<(&mut Density, &mut NearDensity, &mut Pressure, &mut NearPressure, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<&PredictedPosition, With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(
        |(
            mut density,
            mut near_density,
            mut pressure,
            mut near_pressure,
            position
        )| {
            let mut new_density = 0.;
            let mut new_near_density = 0.;
            for neighbor_position in neighbor_query.iter() {
                let distance = position.value.distance(neighbor_position.value);

                let influence = smoothing_kernel(fluid_props.smoothing_radius, distance);
                new_density += fluid_props.mass * influence;

                let influence = smoothing_kernel_near(fluid_props.smoothing_radius, distance);
                new_near_density += fluid_props.mass * influence;
            }
            density.value = new_density;
            near_density.value = new_near_density;
            pressure.value = convert_density_to_pressure(
                new_density,
                fluid_props.target_density,
                fluid_props.pressure_scalar,
            );
            near_pressure.value = convert_near_density_to_near_pressure(
                new_near_density,
                fluid_props.near_pressure_scalar
            );
        }
    );
}


pub fn smoothing_kernel_derivative(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }

    // Slope calculation
    let scale = 12. / (PI * radius.powi(4));
    (distance - radius) * scale
}


pub fn smoothing_kernel_derivative_near(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }

    // Slope calculation
    let scale = 30. / (PI * radius.powi(5));
    let v = distance - radius;
    v * v * scale
}


pub fn calculate_shared_pressure(pressure_1: f32, pressure_2: f32) -> f32 {
    (pressure_1 + pressure_2) / 2.
}


fn update_pressure_force(
    mut query: Query<(Entity, &mut PressureForce, &Pressure, &NearPressure, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<(Entity, &Density, &NearDensity, &Pressure, &NearPressure, &PredictedPosition), With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(
        |(
            particle,
            mut pressure_force,
            pressure,
            near_pressure,
            position
        )| {
            let mut new_pressure_force = Vec2::ZERO;
            for (
                neighbor,
                density,
                near_density,
                neighbor_pressure,
                neighbor_near_pressure,
                neighbor_position
            ) in neighbor_query.iter() {
                if particle == neighbor {
                    continue;
                }

                let distance = position.value.distance(neighbor_position.value);

                let direction: Vec2;
                if distance > 0. {
                    direction = (position.value - neighbor_position.value) / distance * -1.;
                } else {
                    direction = Vec2::Y;
                }

                let slope = smoothing_kernel_derivative(fluid_props.smoothing_radius, distance);
                let shared_pressure = calculate_shared_pressure(pressure.value, neighbor_pressure.value);

                let slope_near = smoothing_kernel_derivative_near(fluid_props.smoothing_radius, distance);
                let shared_pressure_near = calculate_shared_pressure(near_pressure.value, neighbor_near_pressure.value);

                new_pressure_force += direction * shared_pressure * slope * fluid_props.mass / density.value;
                new_pressure_force += direction * shared_pressure_near * slope_near * fluid_props.mass / near_density.value;
            }
            pressure_force.value = new_pressure_force;
        }
    );
}


fn update_acceleration_and_velocity(
    mut query: Query<(&mut Acceleration, &mut Velocity, &PressureForce, &Density), With<FluidParticle>>,
    time: Res<Time<Fixed>>,
) {
    query.par_iter_mut().for_each(|(mut acceleration, mut velocity, pressure_force, density)| {
        let pressure_acceleration = pressure_force.value / density.value;
        acceleration.value = pressure_acceleration;
        velocity.value += acceleration.value * time.delta_seconds();
    });
}


fn update_velocity_on_collision(
    mut query: Query<(&mut Velocity, &mut Transform), With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
    container: Res<FluidContainer>,
) {
    let (mut ext_min, mut ext_max) = container.get_extents();
    let rad_vec = Vec2::ONE * fluid_props.radius;
    ext_min += rad_vec;
    ext_max -= rad_vec;

    query.par_iter_mut().for_each(|(mut velocity, mut transform)| {
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
    });
}


fn update_position(mut query: Query<(&Velocity, &mut Transform), With<FluidParticle>>, time: Res<Time<Fixed>>) {
    query.par_iter_mut().for_each(|(velocity, mut transform)| {
        transform.translation += velocity.value.extend(0.) * time.delta_seconds();
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
