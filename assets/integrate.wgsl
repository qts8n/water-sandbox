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
};

struct WorldCursor {
    position: vec2<f32>,
    radius: f32,
    force: f32,
};

struct FluidContainer {
    position: vec2<f32>,
    size: vec2<f32>,
};

struct Gravity {
    value: vec2<f32>,
};

struct FluidParticle {
    position: vec2<f32>,
    density: vec2<f32>,
    pressure: vec2<f32>,
    velocity: vec2<f32>,
    acceleration: vec2<f32>,
    predicted_position: vec2<f32>,
};

const LOOKAHEAD_FACTOR: f32 = 1. / 120.;

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<uniform> world_cursor: WorldCursor;
@group(0) @binding(2) var<uniform> fluid_container: FluidContainer;
@group(0) @binding(3) var<uniform> gravity: Gravity;
@group(0) @binding(4) var<storage, read_write> particles: array<FluidParticle>;

@compute @workgroup_size(1024, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if (index >= fluid_props.num_particles) {
        return;
    }

    // Calculate external force
    var external_acceleration = vec2(0.);
    if (world_cursor.force != 0.) {
        let dir_to_cursor = world_cursor.position - particles[index].position;

        let dst = distance(world_cursor.position, particles[index].position);
        if (dst < world_cursor.radius && dst > 0.) {
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

    if (particles[index].position.x < ext_min.x) {
        particles[index].velocity.x *= -1. * fluid_props.collision_damping;
        particles[index].position.x = ext_min.x;
    } else if (particles[index].position.x > ext_max.x) {
        particles[index].velocity.x *= -1. * fluid_props.collision_damping;
        particles[index].position.x = ext_max.x;
    }

    if (particles[index].position.y < ext_min.y) {
        particles[index].velocity.y *= -1. * fluid_props.collision_damping;
        particles[index].position.y = ext_min.y;
    } else if (particles[index].position.y > ext_max.y) {
        particles[index].velocity.y *= -1. * fluid_props.collision_damping;
        particles[index].position.y = ext_max.y;
    }

    // Calculate predicted postions
    particles[index].predicted_position = particles[index].position + particles[index].velocity * LOOKAHEAD_FACTOR;
}
