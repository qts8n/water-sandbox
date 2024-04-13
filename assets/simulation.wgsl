const WORKGROUP_SIZE: u32 = 1024;

const LOOKAHEAD_FACTOR: f32 = 1. / 50.;
const DENSITY_PADDING: f32 = 0.00001;

const OFFSET_TABLE: array<vec3i, 27> = array<vec3i, 27>(
    vec3i(-1, -1, -1),
    vec3i(-1, -1, 0),
    vec3i(-1, -1, 1),
    vec3i(-1, 0, -1),
    vec3i(-1, 0, 0),
    vec3i(-1, 0, 1),
    vec3i(-1, 1, -1),
    vec3i(-1, 1, 0),
    vec3i(-1, 1, 1),
    vec3i(0, -1, -1),
    vec3i(0, -1, 0),
    vec3i(0, -1, 1),
    vec3i(0, 0, -1),
    vec3i(0, 0, 0),
    vec3i(0, 0, 1),
    vec3i(0, 1, -1),
    vec3i(0, 1, 0),
    vec3i(0, 1, 1),
    vec3i(1, -1, -1),
    vec3i(1, -1, 0),
    vec3i(1, -1, 1),
    vec3i(1, 0, -1),
    vec3i(1, 0, 0),
    vec3i(1, 0, 1),
    vec3i(1, 1, -1),
    vec3i(1, 1, 0),
    vec3i(1, 1, 1),
);

const INF: u32 = 999999999;

const P1: u32 = 15823;  // Some large primes for hashing
const P2: u32 = 9737333;
const P3: u32 = 440817757;

const HALF_SIZE: f32 = 0.5; // Half of the unit cube side

struct FluidProps {
    delta_time: f32,
    collision_damping: f32,
    smoothing_radius: f32,
    target_density: f32,
    pressure_scalar: f32,
    near_pressure_scalar: f32,
    viscosity_strength: f32,
}

struct SmoothingKernel {
    pow2: f32,
    pow2_der: f32,
    pow3: f32,
    pow3_der: f32,
    spikey_pow3: f32,
}

struct FluidContainer {
    world_to_local: mat4x4<f32>,
    local_to_world: mat4x4<f32>,
}

struct Gravity {
    value: vec4<f32>,
}

struct FluidParticle {
    position: vec4<f32>,
    density: vec2<f32>,
    pressure: vec2<f32>,
    velocity: vec4<f32>,
    acceleration: vec4<f32>,
    predicted_position: vec4<f32>,
}

// Shared between passes
@group(0) @binding(0) var<uniform> num_particles: u32;
@group(0) @binding(1) var<uniform> fluid_props: FluidProps;
@group(0) @binding(2) var<storage, read_write> particles: array<FluidParticle>;
// Only used in integrate
@group(0) @binding(3) var<uniform> fluid_container: FluidContainer;
@group(0) @binding(4) var<uniform> gravity: Gravity;
// Used elsewhere
@group(0) @binding(3) var<storage> particle_indicies: array<u32>;
@group(0) @binding(4) var<storage, read_write> particle_cell_indicies: array<u32>;
@group(0) @binding(5) var<storage, read_write> cell_offsets: array<u32>;
@group(0) @binding(6) var<uniform> kernel: SmoothingKernel;

// Smothing radius kernel functions

fn smoothing_kernel(dst: f32) -> f32 {
    let v = fluid_props.smoothing_radius - dst;
    return v * v * kernel.pow2;
}

fn smoothing_kernel_near(dst: f32) -> f32 {
    let v = fluid_props.smoothing_radius - dst;
    return v * v * v * kernel.pow3;
}

// Slope calculation

fn smoothing_kernel_derivative(dst: f32) -> f32 {
    return (dst - fluid_props.smoothing_radius) * kernel.pow2_der;
}

fn smoothing_kernel_derivative_near(dst: f32) -> f32 {
    let v = dst - fluid_props.smoothing_radius;
    return v * v * kernel.pow3_der;
}

fn smoothing_kernel_viscosity(dst: f32) -> f32 {
    let v = fluid_props.smoothing_radius * fluid_props.smoothing_radius - dst * dst;
    return v * v * v * kernel.spikey_pow3;
}

// Hashing cell indicies

fn get_cell(position: vec3<f32>) -> vec3<i32> {
    return vec3<i32>(floor(position / fluid_props.smoothing_radius));
}

fn hash_cell(cell_index: vec3<i32>) -> u32 {
    let cell = vec3<u32>(cell_index);
    return (cell.x * P1 + cell.y * P2 + cell.z * P3) % num_particles;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn hash_particles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= num_particles {
        return;
    }

    cell_offsets[index] = INF;
    let particle_index = particle_indicies[index];
    particle_cell_indicies[particle_index] = hash_cell(get_cell(particles[particle_index].predicted_position.xyz));
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_density(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= num_particles {
        return;
    }

    var offset_table = OFFSET_TABLE;

    let particle_index = particle_indicies[index];
    let origin = particles[particle_index].predicted_position;
    let cell_index = get_cell(origin.xyz);

    // Accumulate density
    var density: f32 = 0.;
    var near_density: f32 = 0.;

    // Iterate neighbour cells
    for (var i = 0; i < 27; i++) {
        let neighbour_cell_index = cell_index + offset_table[i];
        let hash_index = hash_cell(neighbour_cell_index);
        var neighbour_it = cell_offsets[hash_index];
        // Iterate neighbours in the cell
        while (neighbour_it < num_particles) {
            let neighbour_index = particle_indicies[neighbour_it];
            if particle_cell_indicies[neighbour_index] != hash_index {
                break;
            }
            neighbour_it++;

            let neighbour = particles[neighbour_index];

            let dst = distance(neighbour.predicted_position, origin);
            if dst > fluid_props.smoothing_radius {
                continue;
            }

            density += smoothing_kernel(dst);
            near_density += smoothing_kernel_near(dst);
        }
    }

    // Store density
    density = density + DENSITY_PADDING;
    near_density = near_density + DENSITY_PADDING;
    particles[particle_index].density = vec2(density, near_density);

    // Convert density to pressure
    let pressure = fluid_props.pressure_scalar * (density - fluid_props.target_density);
    let near_pressure = fluid_props.near_pressure_scalar * near_density;
    particles[particle_index].pressure = vec2(pressure, near_pressure);
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_pressure_force(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= num_particles {
        return;
    }

    var offset_table = OFFSET_TABLE;

    let particle_index = particle_indicies[index];
    let origin = particles[particle_index].predicted_position;
    let velocity = particles[particle_index].velocity;
    let pressure = particles[particle_index].pressure.x;
    let near_pressure = particles[particle_index].pressure.y;
    let cell_index = get_cell(origin.xyz);

    // Accumulate pressure force
    var pressure_force = vec3(0.);
    var viscosity_force = vec3(0.);

    // Iterate neighbour cells
    for (var i = 0; i < 27; i++) {
        let neighbour_cell_index = cell_index + offset_table[i];
        let hash_index = hash_cell(neighbour_cell_index);
        var neighbour_it = cell_offsets[hash_index];

        // Iterate neighbours in the cell
        while (neighbour_it < num_particles) {
            let neighbour_index = particle_indicies[neighbour_it];
            if particle_cell_indicies[neighbour_index] != hash_index {
                break;
            }
            neighbour_it++;

            if particle_index == neighbour_index {
                continue;
            }

            let neighbour = particles[neighbour_index];

            // Find direction of the force
            let dst = distance(neighbour.predicted_position, origin);
            if dst > fluid_props.smoothing_radius {
                continue;
            }
            var dir = (neighbour.predicted_position - origin).xyz;
            if dst > 0. {
                dir /= dst;
            } else {
                dir = vec3(0., 1., 0.);
            }

            // Calculate pressure contribution taking into account shared pressure
            let slope = smoothing_kernel_derivative(dst);
            let shared_pressure = (pressure + neighbour.pressure.x) / 2.;

            // Calculate near pressure contribution
            let slope_near = smoothing_kernel_derivative_near(dst);
            let shared_pressure_near = (near_pressure + neighbour.pressure.y) / 2.;

            pressure_force += dir * shared_pressure * slope / neighbour.density.x;
            pressure_force += dir * shared_pressure_near * slope_near / neighbour.density.y;

            let viscosity = smoothing_kernel_viscosity(dst);
            viscosity_force += (neighbour.velocity - velocity).xyz * viscosity;
        }
    }
    let pressure_contribution = pressure_force / particles[particle_index].density.x;
    let viscosity_contribution = viscosity_force * fluid_props.viscosity_strength;

    particles[particle_index].acceleration = vec4(pressure_contribution + viscosity_contribution, 0.);
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn integrate(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= num_particles {
        return;
    }

    // Integrate
    particles[index].velocity += (gravity.value + particles[index].acceleration) * fluid_props.delta_time;
    particles[index].position += particles[index].velocity * fluid_props.delta_time;

    // We're now in unit cube space
    var local_position = fluid_container.world_to_local * particles[index].position;
    var local_velocity = fluid_container.world_to_local * particles[index].velocity;

    // --- Handle collisions
    let edge_dst = HALF_SIZE - abs(local_position);
    if edge_dst.x <= 0. {
        local_position.x = HALF_SIZE * sign(local_position.x);
        local_velocity.x *= -1. * fluid_props.collision_damping;
    }
    if edge_dst.y <= 0. {
        local_position.y = HALF_SIZE * sign(local_position.y);
        local_velocity.y *= -1. * fluid_props.collision_damping;
    }
    if edge_dst.z <= 0. {
        local_position.z = HALF_SIZE * sign(local_position.z);
        local_velocity.z *= -1. * fluid_props.collision_damping;
    }

    particles[index].position = fluid_container.local_to_world * local_position;
    particles[index].velocity = fluid_container.local_to_world * local_velocity;

    // Calculate predicted postions
    particles[index].predicted_position = particles[index].position + particles[index].velocity * LOOKAHEAD_FACTOR;
}
