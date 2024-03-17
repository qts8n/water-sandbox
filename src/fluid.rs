use bevy::{prelude::*, sprite::{MaterialMesh2dBundle, Mesh2dHandle}};

use crate::smoothing;
use crate::helpers;
use crate::schedule::{InGameSet, PhysicsSet};
use crate::state::GameState;
use crate::fluid_container::FluidContainer;
use crate::gravity::Gravity;

const N_SIZE: usize = 50;

const PARTICLE_MAX_VELOCITY: f32 = 40.;  // Used only in color gradient
const PARTICLE_RADIUS: f32 = 0.05;
const PARTICLE_COLLISION_DAMPING: f32 = 0.95;
const PARTICLE_MASS: f32 = 1.;
const PARTICLE_SMOOTHING_RADIUS: f32 = 0.2;
const PARTICLE_TARGET_DENSITY: f32 = 10.;
const PARTICLE_PRESSURE_SCALAR: f32 = 30.;
const PARTICLE_NEAR_PRESSURE_SCALAR: f32 = 1.;
const PARTICLE_VISCOSITY_STRENGTH: f32 = 0.1;
const PARTICLE_LOOKAHEAD_SCALAR: f32 = 1. / 60.;  // 60 Hz


#[derive(Component, Default, Debug)]
pub struct Velocity {
    pub value: Vec2,
}


#[derive(Component, Default, Debug)]
pub struct Acceleration {
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
    pub predicted_position: PredictedPosition,
}


#[derive(Component, Default, Debug)]
pub struct FluidParticleProperties {
    pub density: f32,
    pub near_density: f32,
    pub pressure: f32,
    pub near_pressure: f32,
}


#[derive(Bundle, Default)]
pub struct FluidParticleBundle {
    pub properties: FluidParticleProperties,
    pub mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
    pub moving_object_bundle: MovingObjectBundle,
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
    pub viscosity_strength: f32,
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
            viscosity_strength: PARTICLE_VISCOSITY_STRENGTH,
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
            .add_systems(Update, update_color.in_set(InGameSet::EntityUpdates))
            .add_systems(Update, despawn_liquid.in_set(InGameSet::DespawnEntities))
            .add_systems(FixedUpdate, integrate_positions.in_set(PhysicsSet::PositionUpdates))
            .add_systems(FixedUpdate, (
                // update_color,
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
    let points = helpers::cube_fluid(N_SIZE, N_SIZE, fluid_props.radius);

    // let (ext_min, ext_max) = container.get_extents();
    // let points = random_fluid(N_SIZE * N_SIZE, ext_min, ext_max);

    let particle_bundles: Vec<(FluidParticleBundle, FluidParticle)> = points.iter().map(|point| {(
        FluidParticleBundle {
            mesh_bundle: MaterialMesh2dBundle {
                mesh: shape.clone(),
                material: materials.add(Color::WHITE),
                transform: Transform::from_xyz(point.x, point.y, 0.),
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
        // Integrate positions using accumulated acceleration
        velocity.value += (gravity.value + acceleration.value) / fluid_props.mass * time.delta_seconds();
        transform.translation += velocity.value.extend(0.) * time.delta_seconds();

        // Handle collisions
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

        // Predict future position values
        predicted_position.value = transform.translation.xy() + velocity.value * PARTICLE_LOOKAHEAD_SCALAR;
    });
}


fn update_density_and_pressure(
    mut query: Query<(&mut FluidParticleProperties, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<&PredictedPosition, With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(|(mut props, position)| {
        let mut new_density = 0.;
        let mut new_near_density = 0.;

        // Accumulate density amongst neighbours
        for neighbor_position in neighbor_query.iter() {
            let distance = position.value.distance(neighbor_position.value);
            if distance > fluid_props.smoothing_radius {
                continue;
            }

            new_density += smoothing::smoothing_kernel(fluid_props.smoothing_radius, distance);
            new_near_density += smoothing::smoothing_kernel_near(fluid_props.smoothing_radius, distance);
        }

        // Take mass into account and calculate pressure by converting the density
        props.density = fluid_props.mass * new_density + smoothing::DENSITY_PADDING;
        props.pressure = fluid_props.pressure_scalar * (props.density - fluid_props.target_density);

        props.near_density = fluid_props.mass * new_near_density + smoothing::DENSITY_PADDING;
        props.near_pressure = fluid_props.near_pressure_scalar * props.near_density;
    });
}


fn update_pressure_force(
    mut query: Query<(Entity, &mut Acceleration, &Velocity, &FluidParticleProperties, &PredictedPosition), With<FluidParticle>>,
    neighbor_query: Query<(Entity, &Velocity, &FluidParticleProperties, &PredictedPosition), With<FluidParticle>>,
    fluid_props: Res<FluidParticleStaticProperties>,
) {
    query.par_iter_mut().for_each(|(
        particle,
        mut acceleration,
        velocity,
        props,
        position,
    )| {
        let mut pressure_force = Vec2::ZERO;
        let mut viscosity_force = Vec2::ZERO;

        for (
            neighbor,
            neighbor_velocity,
            neighbor_props,
            neighbor_position,
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
            direction *= fluid_props.mass;

            // Calculate pressure contribution taking into account shared pressure
            let slope = smoothing::smoothing_kernel_derivative(fluid_props.smoothing_radius, distance);
            let shared_pressure = (props.pressure + neighbor_props.pressure) / 2.;

            // Calculate near pressure contribution
            let slope_near = smoothing::smoothing_kernel_derivative_near(fluid_props.smoothing_radius, distance);
            let shared_pressure_near = (props.near_pressure + neighbor_props.near_pressure) / 2.;

            pressure_force += direction * shared_pressure * slope / neighbor_props.density;
            pressure_force += direction * shared_pressure_near * slope_near / neighbor_props.near_density;

            // Calculate viscosity contribution
            let viscosity = smoothing::smoothing_kernel_viscosity(fluid_props.smoothing_radius, distance);
            viscosity_force += (neighbor_velocity.value - velocity.value) * viscosity;
        }
        acceleration.value = pressure_force / props.density + viscosity_force * fluid_props.viscosity_strength;
    });
}


fn update_color(
    query: Query<(&Handle<ColorMaterial>, &Velocity), With<FluidParticle>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (material_handle, velocity) in query.iter() {
        let Some(material) = materials.get_mut(material_handle) else { continue };
        // Color gradient depending on the velocity
        // HSL: 20 <= H <= 200, S = 100, L = 50
        let magnitude = velocity.value.length_squared();
        if magnitude > PARTICLE_MAX_VELOCITY {
            continue;
        } else {
            let h = (1. - magnitude / PARTICLE_MAX_VELOCITY) * 180. + 20.;
            material.color = Color::hsl(h, 1., 0.5);
        }
    }
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
