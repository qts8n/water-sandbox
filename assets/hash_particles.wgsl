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

struct FluidParticle {
    position: vec2<f32>,
    density: vec2<f32>,
    pressure: vec2<f32>,
    velocity: vec2<f32>,
    acceleration: vec2<f32>,
    predicted_position: vec2<f32>,
}

const P1 = 15823u;  // Some large primes
const P2 = 9737333u;

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage> particles: array<FluidParticle>;
@group(0) @binding(2) var<storage> particle_indicies: array<u32>;
@group(0) @binding(3) var<storage, read_write> particle_cell_indicies: array<u32>;
@group(0) @binding(4) var<storage, read_write> cell_offsets: array<u32>;

fn get_cell(position: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(floor(position / fluid_props.smoothing_radius));
}

fn hash_cell(cell_index: vec2<i32>) -> u32 {
    let cell = vec2<u32>(cell_index);
    return (cell.x * P1 + cell.y * P2) % fluid_props.num_particles;
}

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    cell_offsets[index] = 99999999u;
    let particle_index = particle_indicies[index];
    particle_cell_indicies[particle_index] = hash_cell(get_cell(particles[index].position));
}
