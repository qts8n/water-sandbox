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
const INF: u32 = 99999999;
const P1: u32 = 15823;  // Some large primes
const P2: u32 = 9737333;

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage, read_write> particles: array<FluidParticle>;
@group(0) @binding(2) var<storage> particle_indicies: array<u32>;
@group(0) @binding(3) var<storage> particle_cell_indicies: array<u32>;
@group(0) @binding(4) var<storage> cell_offsets: array<u32>;

// Slope calculation

fn smoothing_kernel_derivative(radius: f32, dst: f32) -> f32 {
    let scale = 12. / (PI * pow(radius, 4.));
    return (dst - radius) * scale;
}

fn smoothing_kernel_derivative_near(radius: f32, dst: f32) -> f32 {
    let scale = 30. / (PI * pow(radius, 5.));
    let v = dst - radius;
    return v * v * scale;
}

fn smoothing_kernel_viscosity(radius: f32, dst: f32) -> f32 {
    let volume = 4. / (PI * pow(radius, 8.));
    let v = radius * radius - dst * dst;
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
                let slope = smoothing_kernel_derivative(fluid_props.smoothing_radius, dst);
                let shared_pressure = (pressure + neighbour.pressure.x) / 2.;

                // Calculate near pressure contribution
                let slope_near = smoothing_kernel_derivative_near(fluid_props.smoothing_radius, dst);
                let shared_pressure_near = (near_pressure + neighbour.pressure.y) / 2.;

                pressure_force += dir * shared_pressure * slope / neighbour.density.x;
                pressure_force += dir * shared_pressure_near * slope_near / neighbour.density.y;

                let viscosity = smoothing_kernel_viscosity(fluid_props.smoothing_radius, dst);
                viscosity_force += (neighbour.velocity - velocity) * viscosity;

                neighbour_it++;
            }
        }
    }

    particles[particle_index].acceleration = pressure_force / particles[particle_index].density.x + viscosity_force * fluid_props.viscosity_strength;
}
