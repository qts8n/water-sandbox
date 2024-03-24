const WORKGROUP_SIZE: u32 = 1024;

const LOOKAHEAD_FACTOR: f32 = 1. / 50.;
const DENSITY_PADDING: f32 = 0.00001;

const PI: f32 = 3.141592653589793238;  // Math constants
const INF: u32 = 999999999;

const P1: u32 = 15823;  // Some large primes for hashing
const P2: u32 = 9737333;
const P3: u32 = 440817757;

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

struct FluidContainer {
    position: vec3<f32>,
    size: vec3<f32>,
}

struct Gravity {
    value: vec3<f32>,
}

// Shared between passes
@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage, read_write> positions: array<vec3f>;
@group(0) @binding(2) var<storage, read_write> velocities: array<vec3f>;
@group(0) @binding(3) var<storage, read_write> accelerations: array<vec3f>;
@group(0) @binding(4) var<storage, read_write> predicted_positions: array<vec3f>;
// Only used in integrate
@group(0) @binding(5) var<uniform> fluid_container: FluidContainer;
@group(0) @binding(6) var<uniform> gravity: Gravity;
// Used elsewhere
@group(0) @binding(5) var<storage, read_write> densities: array<vec2f>;
@group(0) @binding(6) var<storage, read_write> pressures: array<vec2f>;
@group(0) @binding(7) var<storage> particle_indicies: array<u32>;
@group(0) @binding(8) var<storage, read_write> particle_cell_indicies: array<u32>;
@group(0) @binding(9) var<storage, read_write> cell_offsets: array<u32>;

// Smothing radius kernel functions

fn smoothing_kernel(dst: f32) -> f32 {
    let volume = 15. / (2. * PI * pow(fluid_props.smoothing_radius, 5.));
    let v = fluid_props.smoothing_radius - dst;
    return v * v * volume;
}

fn smoothing_kernel_near(dst: f32) -> f32 {
    let volume = 15. / (PI * pow(fluid_props.smoothing_radius, 6.));
    let v = fluid_props.smoothing_radius - dst;
    return v * v * v * volume;
}

// Slope calculation

fn smoothing_kernel_derivative(dst: f32) -> f32 {
    let scale = 15. / (PI * pow(fluid_props.smoothing_radius, 5.));
    return (dst - fluid_props.smoothing_radius) * scale;
}

fn smoothing_kernel_derivative_near(dst: f32) -> f32 {
    let scale = 45. / (PI * pow(fluid_props.smoothing_radius, 6.));
    let v = dst - fluid_props.smoothing_radius;
    return v * v * scale;
}

fn smoothing_kernel_viscosity(dst: f32) -> f32 {
    let volume = 315. / (64. * PI * pow(fluid_props.smoothing_radius, 9.));
    let v = fluid_props.smoothing_radius * fluid_props.smoothing_radius - dst * dst;
    return v * v * v * volume;
}

// Hashing cell indicies

fn get_cell(position: vec3<f32>) -> vec3<i32> {
    return vec3<i32>(floor(position / fluid_props.smoothing_radius));
}

fn hash_cell(cell_index: vec3<i32>) -> u32 {
    let cell = vec3<u32>(cell_index);
    return (cell.x * P1 + cell.y * P2 + cell.z * P3) % fluid_props.num_particles;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_density(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    let particle_index = particle_indicies[index];
    let origin = predicted_positions[particle_index];
    let cell_index = get_cell(origin);

    // Accumulate density
    var density: f32 = 0.;
    var near_density: f32 = 0.;

    // Iterate neighbour cells
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            for (var k = -1; k <= 1; k++) {
                let neighbour_cell_index = cell_index + vec3(i, j, k);
                let hash_index = hash_cell(neighbour_cell_index);
                var neighbour_it = cell_offsets[hash_index];

                // Iterate neighbours in the cell
                while (neighbour_it != INF && neighbour_it < fluid_props.num_particles) {
                    let neighbour_index = particle_indicies[neighbour_it];
                    if particle_cell_indicies[neighbour_index] != hash_index {
                        break;
                    }

                    let dst = distance(predicted_positions[neighbour_index], origin);
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
    }

    // Store density
    density = fluid_props.mass * density + DENSITY_PADDING;
    near_density = fluid_props.mass * near_density + DENSITY_PADDING;
    densities[particle_index] = vec2(density, near_density);

    // Convert density to pressure
    let pressure = fluid_props.pressure_scalar * (density - fluid_props.target_density);
    let near_pressure = fluid_props.near_pressure_scalar * near_density;
    pressures[particle_index] = vec2(pressure, near_pressure);
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn update_pressure_force(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    let particle_index = particle_indicies[index];
    let origin = predicted_positions[particle_index];
    let velocity = velocities[particle_index];
    let pressure = pressures[particle_index].x;
    let near_pressure = pressures[particle_index].y;
    let cell_index = get_cell(origin);

    // Accumulate pressure force
    var pressure_force = vec3(0.);
    var viscosity_force = vec3(0.);

    // Iterate neighbour cells
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            for (var k = -1; k <= 1; k++) {
                let neighbour_cell_index = cell_index + vec3(i, j, k);
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

                    // Find direction of the force
                    var dir = predicted_positions[neighbour_index] - origin;
                    let dst = distance(predicted_positions[neighbour_index], origin);
                    if dst > fluid_props.smoothing_radius {
                        neighbour_it++;
                        continue;
                    }
                    if dst > 0. {
                        dir /= dst;
                    } else {
                        dir = vec3(0., 1., 0.);
                    }
                    dir *= fluid_props.mass;

                    // Calculate pressure contribution taking into account shared pressure
                    let slope = smoothing_kernel_derivative(dst);
                    let shared_pressure = (pressure + pressures[neighbour_index].x) / 2.;

                    // Calculate near pressure contribution
                    let slope_near = smoothing_kernel_derivative_near(dst);
                    let shared_pressure_near = (near_pressure + pressures[neighbour_index].y) / 2.;

                    pressure_force += dir * shared_pressure * slope / densities[neighbour_index].x;
                    pressure_force += dir * shared_pressure_near * slope_near / densities[neighbour_index].y;

                    let viscosity = smoothing_kernel_viscosity(dst);
                    viscosity_force += (velocities[neighbour_index] - velocity) * viscosity;

                    neighbour_it++;
                }
            }
        }
    }

    accelerations[particle_index] = pressure_force / densities[particle_index].x + viscosity_force * fluid_props.viscosity_strength;
}

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn integrate(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if index >= fluid_props.num_particles {
        return;
    }

    // Integrate
    velocities[index] += (gravity.value + accelerations[index]) / fluid_props.mass * fluid_props.delta_time;
    positions[index] += velocities[index] * fluid_props.delta_time;

    // Handle collisions
    let half_size = fluid_container.size / 2.;
    let ext_min = fluid_container.position - half_size + fluid_props.radius;
    let ext_max = fluid_container.position + half_size - fluid_props.radius;

    if positions[index].x < ext_min.x {
        velocities[index].x *= -1. * fluid_props.collision_damping;
        positions[index].x = ext_min.x;
    } else if positions[index].x > ext_max.x {
        velocities[index].x *= -1. * fluid_props.collision_damping;
        positions[index].x = ext_max.x;
    }

    if positions[index].y < ext_min.y {
        velocities[index].y *= -1. * fluid_props.collision_damping;
        positions[index].y = ext_min.y;
    } else if positions[index].y > ext_max.y {
        velocities[index].y *= -1. * fluid_props.collision_damping;
        positions[index].y = ext_max.y;
    }

    if positions[index].z < ext_min.z {
        velocities[index].z *= -1. * fluid_props.collision_damping;
        positions[index].z = ext_min.z;
    } else if positions[index].z > ext_max.z {
        velocities[index].z *= -1. * fluid_props.collision_damping;
        positions[index].z = ext_max.z;
    }

    // Calculate predicted postions
    predicted_positions[index] = positions[index] + velocities[index] * LOOKAHEAD_FACTOR;
}
