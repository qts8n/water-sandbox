const WORKGROUP_SIZE: u32 = 32;

const LOOKAHEAD_FACTOR: f32 = 1. / 60.;
const DENSITY_PADDING: f32 = 0.00001;

const PI: f32 = 3.141592653589793238;  // Math constants
const INF: u32 = 999999999;

const P1: u32 = 15823;  // Some large primes for hashing
const P2: u32 = 9737333;

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

struct WorldCursor {
    position: vec2<f32>,
    radius: f32,
    force: f32,
}

struct FluidContainer {
    position: vec2<f32>,
    size: vec2<f32>,
}

struct Gravity {
    value: vec2<f32>,
}

struct FluidParticle {
    position: vec2<f32>,
    density: vec2<f32>,
    pressure: vec2<f32>,
    velocity: vec2<f32>,
    acceleration: vec2<f32>,
    predicted_position: vec2<f32>,
}

// Shared between passes
@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage, read_write> particles: array<FluidParticle>;
// Only used in integrate
@group(0) @binding(2) var<uniform> world_cursor: WorldCursor;
@group(0) @binding(3) var<uniform> fluid_container: FluidContainer;
@group(0) @binding(4) var<uniform> gravity: Gravity;
// Used elsewhere
@group(0) @binding(2) var<storage> particle_indicies: array<u32>;
@group(0) @binding(3) var<storage, read_write> particle_cell_indicies: array<u32>;
@group(0) @binding(4) var<storage, read_write> cell_offsets: array<u32>;

// Smothing radius kernel functions

fn smoothing_kernel(dst: f32) -> f32 {
    let volume = 6. / (PI * pow(fluid_props.smoothing_radius, 4.));
    let v = fluid_props.smoothing_radius - dst;
    return v * v * volume;
}

fn smoothing_kernel_near(dst: f32) -> f32 {
    let volume = 10. / (PI * pow(fluid_props.smoothing_radius, 5.));
    let v = fluid_props.smoothing_radius - dst;
    return v * v * v * volume;
}

// Slope calculation

fn smoothing_kernel_derivative(dst: f32) -> f32 {
    let scale = 12. / (PI * pow(fluid_props.smoothing_radius, 4.));
    return (dst - fluid_props.smoothing_radius) * scale;
}

fn smoothing_kernel_derivative_near(dst: f32) -> f32 {
    let scale = 30. / (PI * pow(fluid_props.smoothing_radius, 5.));
    let v = dst - fluid_props.smoothing_radius;
    return v * v * scale;
}

fn smoothing_kernel_viscosity(dst: f32) -> f32 {
    let volume = 4. / (PI * pow(fluid_props.smoothing_radius, 8.));
    let v = fluid_props.smoothing_radius * fluid_props.smoothing_radius - dst * dst;
    return v * v * v * volume;
}

// Hashing cell indicies

fn get_cell(position: vec2<f32>) -> vec2<i32> {
    return vec2<i32>(floor(position / fluid_props.smoothing_radius));
}

fn hash_cell(cell_index: vec2<i32>) -> u32 {
    let cell = vec2<u32>(cell_index);
    return (cell.x * P1 + cell.y * P2) % fluid_props.num_particles;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn hash_particles(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    cell_offsets[index] = INF;
    let particle_index = particle_indicies[index];
    particle_cell_indicies[particle_index] = hash_cell(get_cell(particles[particle_index].predicted_position));
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_density(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
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

                density += smoothing_kernel(dst);
                near_density += smoothing_kernel_near(dst);

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

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_pressure_force(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    let particle_index = particle_indicies[index];
    let origin = particles[particle_index].predicted_position;
    let velocity = particles[particle_index].velocity;
    let pressure = particles[particle_index].pressure.x;
    let near_pressure = particles[particle_index].pressure.y;
    let cell_index = get_cell(origin);

    // Accumulate pressure force
    var pressure_force = vec2(0.);
    var viscosity_force = vec2(0.);

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

                if particle_index == neighbour_index {
                    neighbour_it++;
                    continue;
                }

                let neighbour = particles[neighbour_index];

                // Find direction of the force
                var dir = neighbour.predicted_position - origin;
                let dst = distance(neighbour.predicted_position, origin);
                if dst > fluid_props.smoothing_radius {
                    neighbour_it++;
                    continue;
                }
                if dst > 0. {
                    dir /= dst;
                } else {
                    dir = vec2(0., 1.);
                }
                dir *= fluid_props.mass;

                // Calculate pressure contribution taking into account shared pressure
                let slope = smoothing_kernel_derivative(dst);
                let shared_pressure = (pressure + neighbour.pressure.x) / 2.;

                // Calculate near pressure contribution
                let slope_near = smoothing_kernel_derivative_near(dst);
                let shared_pressure_near = (near_pressure + neighbour.pressure.y) / 2.;

                pressure_force += dir * shared_pressure * slope / neighbour.density.x;
                pressure_force += dir * shared_pressure_near * slope_near / neighbour.density.y;

                let viscosity = smoothing_kernel_viscosity(dst);
                viscosity_force += (neighbour.velocity - velocity) * viscosity;

                neighbour_it++;
            }
        }
    }

    particles[particle_index].acceleration = pressure_force / particles[particle_index].density.x + viscosity_force * fluid_props.viscosity_strength;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn integrate(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    // Calculate external force
    var external_acceleration = vec2(0.);
    if world_cursor.force != 0. {
        let dir_to_cursor = world_cursor.position - particles[index].position;

        let dst = distance(world_cursor.position, particles[index].position);
        if dst < world_cursor.radius && dst > 0. {
            external_acceleration = dir_to_cursor / dst * world_cursor.force;
        }
    }

    // Integrate
    particles[index].velocity += (gravity.value + particles[index].acceleration + external_acceleration) / fluid_props.mass * fluid_props.delta_time;
    particles[index].position += particles[index].velocity * fluid_props.delta_time;

    // Handle collisions
    let half_size = fluid_container.size / 2.;
    let ext_min = fluid_container.position - half_size;
    let ext_max = fluid_container.position + half_size;

    if particles[index].position.x < ext_min.x {
        particles[index].velocity.x *= -1. * fluid_props.collision_damping;
        particles[index].position.x = ext_min.x;
    } else if particles[index].position.x > ext_max.x {
        particles[index].velocity.x *= -1. * fluid_props.collision_damping;
        particles[index].position.x = ext_max.x;
    }

    if particles[index].position.y < ext_min.y {
        particles[index].velocity.y *= -1. * fluid_props.collision_damping;
        particles[index].position.y = ext_min.y;
    } else if particles[index].position.y > ext_max.y {
        particles[index].velocity.y *= -1. * fluid_props.collision_damping;
        particles[index].position.y = ext_max.y;
    }

    // Calculate predicted postions
    particles[index].predicted_position = particles[index].position + particles[index].velocity * LOOKAHEAD_FACTOR;
}
