use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::core::Pod;
use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::helpers::cube_fluid;
use crate::state::GameState;
use crate::schedule::{InGameSet, ShaderPhysicsSet};
use crate::camera::WorldCursor;
use crate::fluid_container::FluidContainer;
use crate::gravity::Gravity;

const N_SIZE: usize = 128;  // FIXME: only works with powers of 2 now
const WORKGROUP_SIZE: u32 = 32;

// const PARTICLE_MAX_VELOCITY: f32 = 40.;  // Used only in color gradient
const PARTICLE_RADIUS: f32 = 0.05;
const PARTICLE_COLLISION_DAMPING: f32 = 0.95;
const PARTICLE_MASS: f32 = 1.;
const PARTICLE_SMOOTHING_RADIUS: f32 = 0.35;
const PARTICLE_TARGET_DENSITY: f32 = 10.;
const PARTICLE_PRESSURE_SCALAR: f32 = 55.;
const PARTICLE_NEAR_PRESSURE_SCALAR: f32 = 2.;
const PARTICLE_VISCOSITY_STRENGTH: f32 = 0.5;
const PARTICLE_LOOKAHEAD_SCALAR: f32 = 1. / 60.;


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidStaticProps {
    pub delta_time: f32,
    pub num_particles: u32,
    pub collision_damping: f32,
    pub mass: f32,
    pub radius: f32,
    pub smoothing_radius: f32,
    pub target_density: f32,
    pub pressure_scalar: f32,
    pub near_pressure_scalar: f32,
    pub viscosity_strength: f32,
}


impl FluidStaticProps {
    pub fn get_batch_size(&self) -> u32 {
        let mut batch_size = self.num_particles / WORKGROUP_SIZE;
        if self.num_particles % WORKGROUP_SIZE > 0 {
            batch_size += 1;
        }
        return batch_size;
    }
}


impl Default for FluidStaticProps {
    fn default() -> Self {
        Self {
            delta_time: PARTICLE_LOOKAHEAD_SCALAR,
            collision_damping: PARTICLE_COLLISION_DAMPING,
            num_particles: 0,
            mass: PARTICLE_MASS,
            radius: PARTICLE_RADIUS,
            smoothing_radius: PARTICLE_SMOOTHING_RADIUS,
            target_density: PARTICLE_TARGET_DENSITY,
            pressure_scalar: PARTICLE_PRESSURE_SCALAR,
            near_pressure_scalar: PARTICLE_NEAR_PRESSURE_SCALAR,
            viscosity_strength: PARTICLE_VISCOSITY_STRENGTH,
        }
    }
}


#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default)]
#[repr(C)]
pub struct FluidParticle {
    pub position: Vec2,
    pub density: Vec2,
    pub pressure: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub predicted_position: Vec2,
}


#[derive(Resource, Clone, Default)]
pub struct FluidParticlesInitial {
    pub positions: Vec<Vec2>,
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default)]
#[repr(C)]
pub struct BitSorter {
    pub block: u32,
    pub dim: u32,
}


impl BitSorter {
    fn new(block: u32, dim: u32) -> Self {
        Self {
            block,
            dim,
        }
    }
}


struct BitSorterStage {
    bit_sorter: BitSorter,
    workgroups: [u32; 3],
    uniform_name: String,
}


#[derive(TypePath)]
struct IntegrateShader;


impl ComputeShader for IntegrateShader {
    fn shader() -> ShaderRef {
        "simulation.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "integrate"
    }
}


#[derive(TypePath)]
struct UpdateDensityShader;


impl ComputeShader for UpdateDensityShader {
    fn shader() -> ShaderRef {
        "simulation.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "update_density"
    }
}


#[derive(TypePath)]
struct UpdatePressureForceShader;


impl ComputeShader for UpdatePressureForceShader {
    fn shader() -> ShaderRef {
        "simulation.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "update_pressure_force"
    }
}

#[derive(TypePath)]
struct HashParticlesShader;


impl ComputeShader for HashParticlesShader {
    fn shader() -> ShaderRef {
        "simulation.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "hash_particles"
    }
}

#[derive(TypePath)]
struct BitonicSortShader;


impl ComputeShader for BitonicSortShader {
    fn shader() -> ShaderRef {
        "bitonic_sort.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "bitonic_sort"
    }
}


#[derive(TypePath)]
struct CalculateCellOffsetsShader;


impl ComputeShader for CalculateCellOffsetsShader {
    fn shader() -> ShaderRef {
        "bitonic_sort.wgsl".into()
    }

    fn entry_point<'a>() -> &'a str {
        "calculate_cell_offsets"
    }
}


pub struct FluidWorker;


impl FluidWorker {
    pub fn create_initial_data_buffer(positions: &Vec<Vec2>) -> (Vec<FluidParticle>, Vec<u32>) {
        let n_points = positions.len();
        let mut initial_data = Vec::with_capacity(n_points);
        let mut initial_indicies = Vec::with_capacity(n_points);
        for (it, &position) in positions.iter().enumerate() {
            initial_data.push(FluidParticle {
                position,
                ..default()
            });
            initial_indicies.push(it as u32);
        }
        return (initial_data, initial_indicies);
    }

    fn get_bit_sorter_stages(data_length: u32, batch_size: u32) -> Vec<BitSorterStage> {
        let input_length = match data_length.checked_next_power_of_two() {
            Some(pot) => pot,
            None => data_length,
        };
        let mut uniform_id = 1;
        let mut dim = 2;
        let mut block_stages = Vec::new();
        while dim <= input_length {
            let mut block = dim >> 1;
            while block > 0 {
                block_stages.push(BitSorterStage {
                    bit_sorter: BitSorter::new(block, dim),
                    workgroups: [batch_size, 1, 1],
                    uniform_name: format!("bit_sorter_{}", uniform_id),
                });
                block >>= 1;
                uniform_id += 1;
            }
            dim <<= 1;
        }
        return block_stages;
    }
}


impl ComputeWorker for FluidWorker {
    fn build(world: &mut World) -> AppComputeWorker<Self> {
        // Init static props
        let mut fluid_props = world.get_resource_or_insert_with(FluidStaticProps::default);
        let points = cube_fluid(N_SIZE, N_SIZE, fluid_props.radius);
        let num_particles = points.len() as u32;
        fluid_props.num_particles = num_particles;
        let batch_size = fluid_props.get_batch_size();
        let static_fluid_props = fluid_props.clone();

        // Init positions
        let mut fluid_initials = world.get_resource_or_insert_with(FluidParticlesInitial::default);
        fluid_initials.positions = points.clone();
        let (initial_data, initial_indicies) = Self::create_initial_data_buffer(&points);

        // Get static shader resources
        let world_cursor = world.get_resource_or_insert_with(WorldCursor::default).clone();
        let gravity = world.get_resource_or_insert_with(Gravity::default).clone();
        let container = world.get_resource_or_insert_with(FluidContainer::default).clone();

        // Init bit sorter stages
        let bit_sorter_stages = Self::get_bit_sorter_stages(num_particles, batch_size);

        let mut builder = AppComputeWorkerBuilder::new(world);
        builder
            .add_uniform("num_particles", &num_particles)
            .add_uniform("fluid_props", &static_fluid_props)
            .add_uniform("world_cursor", &world_cursor)
            .add_uniform("fluid_container", &container)
            .add_uniform("gravity", &gravity)
            .add_staging("particles", &initial_data)
            .add_rw_storage("particle_indicies", &initial_indicies)
            .add_rw_storage("particle_cell_indicies", &initial_indicies)
            .add_rw_storage("cell_offsets", &initial_indicies)
            .add_pass::<HashParticlesShader>([batch_size, 1, 1], &[
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ]);

        // Bitonic sort passes
        println!("Bit sort passes: {}", bit_sorter_stages.len());
        for stage in bit_sorter_stages {
            builder.add_uniform(&stage.uniform_name, &stage.bit_sorter)
                .add_pass::<BitonicSortShader>(stage.workgroups, &[
                    "num_particles",
                    "particle_indicies",
                    "particle_cell_indicies",
                    &stage.uniform_name,
                ]);
        }

        builder
            .add_pass::<CalculateCellOffsetsShader>([batch_size, 1, 1], &[
                "num_particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ])
            .add_pass::<UpdateDensityShader>([batch_size, 1, 1], &[
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ])
            .add_pass::<UpdatePressureForceShader>([batch_size, 1, 1], &[
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ])
            .add_pass::<IntegrateShader>([batch_size, 1, 1], &[
                "fluid_props",
                "particles",
                "world_cursor",
                "fluid_container",
                "gravity",
            ])
            .build()
    }
}


pub struct FluidComputeWorkerPlugin<W: ComputeWorker> {
    _phantom: PhantomData<W>,
}


impl<W: ComputeWorker> Default for FluidComputeWorkerPlugin<W> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}


impl<W: ComputeWorker> Plugin for FluidComputeWorkerPlugin<W> {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(PARTICLE_LOOKAHEAD_SCALAR.into()));
    }

    fn finish(&self, app: &mut App) {
        let worker = W::build(&mut app.world);

        app
            .insert_resource(worker)
            .add_systems(Update, AppComputeWorker::<W>::extract_pipelines)
            .add_systems(PostUpdate, (
                AppComputeWorker::<W>::unmap_all.in_set(ShaderPhysicsSet::Prepare),
                AppComputeWorker::<W>::run.in_set(ShaderPhysicsSet::Pass)
            ));
    }
}


pub struct FluidComputePlugin;


impl Plugin for FluidComputePlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<FluidStaticProps>()
            .init_resource::<FluidParticlesInitial>()
            .add_plugins(AppComputePlugin)
            .add_plugins(FluidComputeWorkerPlugin::<FluidWorker>::default());
    }
}


#[derive(Component, Debug)]
struct FluidParticleLabel(usize);


pub struct FluidPlugin;


impl Plugin for FluidPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(FluidComputePlugin)
            .add_systems(OnExit(GameState::Menu), setup)
            .add_systems(Update, update.in_set(InGameSet::EntityUpdates))
            .add_systems(Update, despawn_liquid.in_set(InGameSet::DespawnEntities));
    }
}


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    fluid_props: Res<FluidStaticProps>,
    fluid_initials: Res<FluidParticlesInitial>,
) {
    let shape = Mesh2dHandle(meshes.add(Circle { radius: fluid_props.radius }));
    let mut particle_bundles = Vec::new();
    let mut particle_id: usize = 0;
    for point in &fluid_initials.positions {
        particle_bundles.push((
            MaterialMesh2dBundle {
                mesh: shape.clone(),
                material: materials.add(Color::WHITE),
                transform: Transform::from_xyz(point.x, point.y, 0.),
                ..default()
            },
            FluidParticleLabel(particle_id),
        ));
        particle_id += 1;
    }
    commands.spawn_batch(particle_bundles);
}


fn update(
    mut query: Query<(&mut Transform, &FluidParticleLabel)>,
    mut worker: ResMut<AppComputeWorker<FluidWorker>>,
    fluid_props: Res<FluidStaticProps>,
    world_cursor: Res<WorldCursor>,
    fluid_container: Res<FluidContainer>,
    gravity: Res<Gravity>,
    // color_query: Query<(&Handle<ColorMaterial>, &FluidParticleLabel)>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if !worker.ready() {
        return;
    }

    let Ok(particles) = worker.try_read_vec::<FluidParticle>("particles") else { return };
    let Ok(()) = worker.try_write("fluid_props", fluid_props.as_ref()) else { return };
    let Ok(()) = worker.try_write("world_cursor", world_cursor.as_ref()) else { return };
    let Ok(()) = worker.try_write("fluid_container", fluid_container.as_ref()) else { return };
    let Ok(()) = worker.try_write("gravity", gravity.as_ref()) else { return };

    query.par_iter_mut().for_each(|(mut transform, particle)| {
        transform.translation = particles[particle.0].position.extend(0.);
    });

    // Update color
    // Color gradient depending on the velocity
    // HSL: 20 <= H <= 200, S = 100, L = 50
    // for (material_handle, particle) in color_query.iter() {
    //     let Some(material) = materials.get_mut(material_handle) else { continue };
    //     let magnitude = particles[particle.0].velocity.length_squared();
    //     if magnitude < PARTICLE_MAX_VELOCITY {
    //         let h = (1. - magnitude / PARTICLE_MAX_VELOCITY) * 180. + 20.;
    //         material.color = Color::hsl(h, 1., 0.5);
    //     }
    // }
}


fn despawn_liquid(
    mut worker: ResMut<AppComputeWorker<FluidWorker>>,
    mut next_state: ResMut<NextState<GameState>>,
    fluid_initials: Res<FluidParticlesInitial>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if !keyboard_input.just_pressed(KeyCode::Space) || !worker.ready() {
        return;
    }

    next_state.set(GameState::GameOver);

    let (initial_data, initial_indicies) = FluidWorker::create_initial_data_buffer(&fluid_initials.positions);
    let Ok(()) = worker.try_write_slice("particles", &initial_data) else { return };
    let Ok(()) = worker.try_write_slice("particle_indicies", &initial_indicies) else { return };
    let Ok(()) = worker.try_write_slice("particle_cell_indicies", &initial_indicies) else { return };
    let Ok(()) = worker.try_write_slice("cell_offsets", &initial_indicies) else { return };
}
