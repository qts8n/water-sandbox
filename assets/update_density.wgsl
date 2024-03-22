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

const PI: f32 = 3.1415926;
const DENSITY_PADDING: f32 = 0.00001;
const INF: u32 = 99999999;
const P1: u32 = 15823;  // Some large primes
const P2: u32 = 9737333;

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage, read_write> particles: array<FluidParticle>;
@group(0) @binding(2) var<storage> particle_indicies: array<u32>;
@group(0) @binding(3) var<storage> particle_cell_indicies: array<u32>;
@group(0) @binding(4) var<storage> cell_offsets: array<u32>;


fn smoothing_kernel(radius: f32, dst: f32) -> f32 {
    let volume = 6. / (PI * pow(radius, 4.));
    let v = radius - dst;
    return v * v * volume;
}

fn smoothing_kernel_near(radius: f32, dst: f32) -> f32 {
    let volume = 10. / (PI * pow(radius, 5.));
    let v = radius - dst;
    return v * v * v * volume;
}

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

    let particle_index = particle_indicies[index];
    let origin = particles[particle_index].predicted_position;
    let cell_index = get_cell(origin);

    // Accumulate density
    var density = 0.;
    var near_density = 0.;

    // Iterate neighbour cells
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            let neighbour_cell_index = cell_index + vec2(i, j);
            let hash_index = hash_cell(neighbour_cell_index);
            var neighbour_it = cell_offsets[hash_index];

            // Iterate neighbours in the cell
            while (neighbour_it != INF && neighbour_it < fluid_props.num_particles) {
                let neighbour_index = particle_indicies[neighbour_it];
                if particle_cell_indicies[neighbour_index] != hash_index {
                    break;
                }

                let neighbour = particles[neighbour_index];

                let dst = distance(neighbour.predicted_position, origin);
                if dst > fluid_props.smoothing_radius {
                    neighbour_it++;
                    continue;
                }

                density += smoothing_kernel(fluid_props.smoothing_radius, dst);
                near_density += smoothing_kernel_near(fluid_props.smoothing_radius, dst);

                neighbour_it++;
            }
        }
    }

    // Store density
    density = fluid_props.mass * density + DENSITY_PADDING;
    near_density = fluid_props.mass * near_density + DENSITY_PADDING;
    particles[particle_index].density = vec2(density, near_density);

    // Convert density to pressure
    let pressure = fluid_props.pressure_scalar * (density - fluid_props.target_density);
    let near_pressure = fluid_props.near_pressure_scalar * near_density;
    particles[particle_index].pressure = vec2(pressure, near_pressure);
}
