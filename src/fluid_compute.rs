use std::f32::consts::PI;
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

const NI_SIZE: usize = 64;  // FIXME: only works with powers of 2 now
const NJ_SIZE: usize = 32;
const NK_SIZE: usize = 32;
const WORKGROUP_SIZE: u32 = 1024;

const PARTICLE_RADIUS: f32 = 0.1;
const PARTICLE_COLLISION_DAMPING: f32 = 0.95;
const PARTICLE_SMOOTHING_RADIUS: f32 = 0.25;
const PARTICLE_TARGET_DENSITY: f32 = 10.;
const PARTICLE_PRESSURE_SCALAR: f32 = 22.;
const PARTICLE_NEAR_PRESSURE_SCALAR: f32 = 2.;
const PARTICLE_VISCOSITY_STRENGTH: f32 = 0.1;
const PARTICLE_LOOKAHEAD_SCALAR: f32 = 1. / 60.;


#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct SmoothingKernel {
    pub pow2: f32,
    pub pow2_der: f32,
    pub pow3: f32,
    pub pow3_der: f32,
    pub spikey_pow3: f32,
}


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct FluidStaticProps {
    pub delta_time: f32,
    pub collision_damping: f32,
    pub smoothing_radius: f32,
    pub target_density: f32,
    pub pressure_scalar: f32,
    pub near_pressure_scalar: f32,
    pub viscosity_strength: f32,
}


impl FluidStaticProps {
    pub fn get_smoothing_kernel(&self) -> SmoothingKernel {
        SmoothingKernel {
            pow2: 15. / (2. * PI * self.smoothing_radius.powi(5)),
            pow2_der: 15. / (PI * self.smoothing_radius.powi(5)),
            pow3: 15. / (PI * self.smoothing_radius.powi(6)),
            pow3_der: 45. / (PI * self.smoothing_radius.powi(6)),
            spikey_pow3: 315. / (64. * PI * self.smoothing_radius.powi(9)),
        }
    }
}


impl Default for FluidStaticProps {
    fn default() -> Self {
        Self {
            delta_time: PARTICLE_LOOKAHEAD_SCALAR,
            collision_damping: PARTICLE_COLLISION_DAMPING,
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

#[derive(ShaderType, Pod, Zeroable, Clone, Copy)]
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

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Default)]
#[repr(C)]
pub struct FluidParticle {
    pub position: Vec4,
    pub density: Vec2,
    pub pressure: Vec2,
    pub velocity: Vec4,
    pub acceleration: Vec4,
    pub predicted_position: Vec4,
}

impl FluidParticle {
    pub fn make_vec_from_positions(points: Vec<Vec3>) -> Vec<Self> {
        let mut particles = Vec::with_capacity(points.len());
        for point in points {
            particles.push(Self {
                position: point.extend(0.),
                predicted_position: point.extend(0.),
                ..default()
            });
        }
        particles
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


fn get_batch_size(data_length: u32) -> u32 {
    let mut batch_size = data_length / WORKGROUP_SIZE;
    if data_length % WORKGROUP_SIZE > 0 {
        batch_size += 1;
    }
    return batch_size;
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
        // Get static shader resources
        let fluid_props = world.resource::<FluidStaticProps>().clone();
        let gravity = world.resource::<Gravity>().clone();
        let container = world.resource::<FluidContainer>().clone();

        // Init positions
        let points = cube_fluid(NI_SIZE, NJ_SIZE, NK_SIZE, PARTICLE_RADIUS);
        let num_particles = points.len() as u32;

        // Init positions
        let mut fluid_initials = world.resource_mut::<FluidParticlesInitial>();
        fluid_initials.positions = points.clone();

        // Init buffers
        let initial_index_buffer = Self::create_initial_index_buffer(num_particles);
        let initial_particle_buffer = FluidParticle::make_vec_from_positions(points);

        // Init worker
        let batch_size = get_batch_size(num_particles);
        let mut builder = AppComputeWorkerBuilder::new(world);
        builder
            .add_uniform("num_particles", &num_particles)
            .add_uniform("fluid_props", &fluid_props)
            .add_uniform("fluid_container", &container.get_ext(PARTICLE_RADIUS))
            .add_uniform("gravity", &gravity)
            .add_staging("particles", &initial_particle_buffer)
            .add_uniform("smoothing_kernel", &fluid_props.get_smoothing_kernel())
            .add_rw_storage("particle_indicies", &initial_index_buffer)
            .add_rw_storage("particle_cell_indicies", &initial_index_buffer)
            .add_rw_storage("cell_offsets", &initial_index_buffer)
            .add_pass::<HashParticlesShader>([batch_size, 1, 1], &[
                "num_particles",
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
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
                "num_particles",
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
                "smoothing_kernel",
            ])
            .add_pass::<UpdatePressureForceShader>([batch_size, 1, 1], &[
                "num_particles",
                "fluid_props",
                "particles",
                "particle_indicies",
                "particle_cell_indicies",
                "cell_offsets",
                "smoothing_kernel",
            ])
            .add_pass::<IntegrateShader>([batch_size, 1, 1], &[
                "num_particles",
                "fluid_props",
                "particles",
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
    fluid_initials: Res<FluidParticlesInitial>,
) {
    let shape = meshes.add(Sphere::new(PARTICLE_RADIUS).mesh().ico(0).unwrap());
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

    let particles = worker.read_vec::<FluidParticle>("particles");
    worker.write("fluid_props", fluid_props.as_ref());
    worker.write("smoothing_kernel", &fluid_props.get_smoothing_kernel());
    worker.write("gravity", gravity.as_ref());

    query.par_iter_mut().for_each(|(mut transform, particle)| {
        transform.translation = particles[particle.0].position.xyz();
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
    let initial_particle_buffer = FluidParticle::make_vec_from_positions(fluid_initials.positions.clone());

    worker.write_slice("particles", &initial_particle_buffer);
    worker.write_slice("particle_indicies", &initial_index_buffer);
    worker.write_slice("particle_cell_indicies", &initial_index_buffer);
    worker.write_slice("cell_offsets", &initial_index_buffer);
}
