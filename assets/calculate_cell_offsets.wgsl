struct FluidProps {
    delta_time: f32,
    num_particles: u32,
    collision_damping: f32,
    mass: f32,
    radius: f32,
    smoothing_radius: f32,
    target_density: f32,
    pressure_scalar: f32,
    near_pressure_scalar: f32,
    viscosity_strength: f32,
}

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage> particle_indicies: array<u32>;
@group(0) @binding(2) var<storage> particle_cell_indicies: array<u32>;
@group(0) @binding(3) var<storage, read_write> cell_offsets: array<atomic<u32>>;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    let particle_index = particle_indicies[index];
    let cell_index = particle_cell_indicies[particle_index];
    atomicMin(&cell_offsets[cell_index], index);
}
