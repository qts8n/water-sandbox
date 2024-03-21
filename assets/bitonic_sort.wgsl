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

struct BitSorter {
    block_size: u32,
    dim: u32,
}

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<uniform> bit_sorter: BitSorter;
@group(0) @binding(2) var<storage, read_write> particle_indicies: array<u32>;
@group(0) @binding(3) var<storage> particle_cell_indicies: array<u32>;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let i = invocation_id.x + invocation_id.y * 262144u;  // 256 * 1024
    let j = i ^ bit_sorter.block_size;

    if j < i || i >= fluid_props.num_particles {
        return;
    }

    var sign = 1;
    if (i & bit_sorter.dim) != 0 {
        sign = -1;
    }

    let key_i = particle_indicies[i];
    let key_j = particle_indicies[j];
    let value_i = particle_cell_indicies[key_i];
    let value_j = particle_cell_indicies[key_j];

    let diff = i32(value_i - value_j) * sign;
    if diff > 0 {
        particle_indicies[i] = key_j;
        particle_indicies[j] = key_i;
    }
}
