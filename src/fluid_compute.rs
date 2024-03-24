use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

use crate::helpers::cube_fluid;
use crate::state::GameState;
use crate::schedule::{InGameSet, ShaderPhysicsSet};
use crate::fluid_container::FluidContainer;
use crate::gravity::Gravity;

const N_SIZE: usize = 16;  // FIXME: only works with powers of 2 now
const WORKGROUP_SIZE: u32 = 1024;

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


#[derive(Resource, Clone, Default)]
pub struct FluidParticlesInitial {
    pub positions: Vec<Vec3>,
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
        "bitonic_sort.wgsl".into()
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
    pub fn create_initial_index_buffer(data_length: u32) -> Vec<u32> {
        let mut initial_indicies = Vec::with_capacity(data_length as usize);
        for it in 0..data_length {
            initial_indicies.push(it);
        }
        return initial_indicies;
    }

    pub fn create_initial_buffer<T: Default>(data_length: u32) -> Vec<T> {
        let mut initial_buffer = Vec::with_capacity(data_length as usize);
        for _ in 0..data_length {
            initial_buffer.push(T::default());
        }
        return initial_buffer;
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
        let mut fluid_props = world.resource_mut::<FluidStaticProps>();
        let points = cube_fluid(N_SIZE, N_SIZE, N_SIZE, fluid_props.radius);
        let num_particles = points.len() as u32;
        fluid_props.num_particles = num_particles;
        let batch_size = fluid_props.get_batch_size();
        println!("NUM PARTICLES: {}; BATCH_SIZE: {}", num_particles, batch_size);
        let static_fluid_props = fluid_props.clone();

        // Init positions
        let mut fluid_initials = world.resource_mut::<FluidParticlesInitial>();
        fluid_initials.positions = points.clone();

        // Get static shader resources
        let gravity = world.resource::<Gravity>().clone();
        let container = world.resource::<FluidContainer>().clone();

        // Init buffers
        let initial_index_buffer = Self::create_initial_index_buffer(num_particles);
        let initial_vec2_buffer = Self::create_initial_buffer::<Vec2>(num_particles);
        let initial_vec3_buffer = Self::create_initial_buffer::<Vec3>(num_particles);

        let mut builder = AppComputeWorkerBuilder::new(world);
        builder
            .add_uniform("num_particles", &static_fluid_props.num_particles)
            .add_uniform("smoothing_radius", &static_fluid_props.smoothing_radius)
            .add_uniform("fluid_props", &static_fluid_props)
            .add_uniform("fluid_container", &container)
            .add_uniform("gravity", &gravity)
            .add_staging("positions", &points)
            .add_rw_storage("densities", &initial_vec2_buffer)
            .add_rw_storage("pressures", &initial_vec2_buffer)
            .add_rw_storage("velocities", &initial_vec3_buffer)
            .add_rw_storage("accelerations", &initial_vec3_buffer)
            .add_rw_storage("predicted_positions", &points)
            .add_rw_storage("particle_indicies", &initial_index_buffer)
            .add_rw_storage("particle_cell_indicies", &initial_index_buffer)
            .add_rw_storage("cell_offsets", &initial_index_buffer)
            .add_pass::<HashParticlesShader>([batch_size, 1, 1], &[
                "num_particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
                "predicted_positions",
                "smoothing_radius"
            ]);

        // Bitonic sort passes
        // Init bit sorter stages
        let bit_sorter_stages = Self::get_bit_sorter_stages(num_particles, batch_size);
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
                "positions",
                "velocities",
                "accelerations",
                "predicted_positions",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ])
            .add_pass::<UpdatePressureForceShader>([batch_size, 1, 1], &[
                "fluid_props",
                "positions",
                "velocities",
                "accelerations",
                "predicted_positions",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
            ])
            .add_pass::<IntegrateShader>([batch_size, 1, 1], &[
                "fluid_props",
                "positions",
                "velocities",
                "accelerations",
                "predicted_positions",
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


#[derive(Component, Debug)]
struct Velocity(Vec3);


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
    mut materials: ResMut<Assets<StandardMaterial>>,
    fluid_props: Res<FluidStaticProps>,
    fluid_initials: Res<FluidParticlesInitial>,
) {
    let shape = meshes.add(Sphere::new(fluid_props.radius).mesh().ico(3).unwrap());
    let material = materials.add(StandardMaterial {
        base_color: Color::CYAN,
        ..default()
    });
    let mut particle_bundles = Vec::new();
    let mut particle_id: usize = 0;
    for &point in &fluid_initials.positions {
        particle_bundles.push((
            PbrBundle {
                mesh: shape.clone(),
                material: material.clone(),
                transform: Transform::from_translation(point),
                ..default()
            },
            Velocity(Vec3::ZERO),
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
    gravity: Res<Gravity>,
) {
    if !worker.ready() {
        return;
    }

    // NOTE: for some reason Vec3 becomes Vec4 in staging buffer
    let positions = worker.read_vec::<Vec4>("positions");
    worker.write("num_particles", &fluid_props.num_particles);
    worker.write("smoothing_radius", &fluid_props.smoothing_radius);
    worker.write("fluid_props", fluid_props.as_ref());
    worker.write("gravity", gravity.as_ref());

    query.par_iter_mut().for_each(|(mut transform, particle)| {
        transform.translation = positions[particle.0].xyz();
    });
}


// fn update_color(
//     color_query: Query<(&Handle<ColorMaterial>, &Velocity), With<FluidParticleLabel>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
// ) {
//     // Color gradient depending on the velocity HSL: 20 <= H <= 200, S = 100, L = 50
//     for (material_handle, velocity) in color_query.iter() {
//         let Some(material) = materials.get_mut(material_handle) else { continue };
//         let magnitude = velocity.0.length_squared();
//         if magnitude < 40. {
//             let h = (1. - magnitude / 40.) * 180. + 20.;
//             material.color = Color::hsl(h, 1., 0.5);
//         }
//     }
// }


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

    let num_particles = fluid_initials.positions.len() as u32;
    let initial_index_buffer = FluidWorker::create_initial_index_buffer(num_particles);
    let initial_vec2_buffer = FluidWorker::create_initial_buffer::<Vec2>(num_particles);
    let initial_vec3_buffer = FluidWorker::create_initial_buffer::<Vec3>(num_particles);

    worker.write_slice("positions", &fluid_initials.positions);
    worker.write_slice("densities", &initial_vec2_buffer);
    worker.write_slice("pressures", &initial_vec2_buffer);
    worker.write_slice("velocities", &initial_vec3_buffer);
    worker.write_slice("accelerations", &initial_vec3_buffer);
    worker.write_slice("predicted_positions", &fluid_initials.positions);

    worker.write_slice("particle_indicies", &initial_index_buffer);
    worker.write_slice("particle_cell_indicies", &initial_index_buffer);
    worker.write_slice("cell_offsets", &initial_index_buffer);
}
