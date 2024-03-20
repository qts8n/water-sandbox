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

struct FluidParticle {
    position: vec2<f32>,
    density: vec2<f32>,
    pressure: vec2<f32>,
    velocity: vec2<f32>,
    acceleration: vec2<f32>,
    predicted_position: vec2<f32>,
};

const PI: f32 = 3.1415926;
const DENSITY_PADDING: f32 = 0.00001;

@group(0) @binding(0) var<uniform> fluid_props: FluidProps;
@group(0) @binding(1) var<storage, read_write> particles: array<FluidParticle>;


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


@compute @workgroup_size(1024, 1, 1)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // Check workgroup boundary
    let index = invocation_id.x;
    if (index >= fluid_props.num_particles) {
        return;
    }

    // Accumulate density
    var density = 0.;
    var near_density = 0.;
    for (var i: u32 = 0; i < fluid_props.num_particles; i++) {
        let neighbour = particles[i];

        let dst = distance(neighbour.predicted_position, particles[index].predicted_position);
        if (dst > fluid_props.smoothing_radius) {
            continue;
        }

        density += smoothing_kernel(fluid_props.smoothing_radius, dst);
        near_density += smoothing_kernel_near(fluid_props.smoothing_radius, dst);
    }

    // Store density
    density = fluid_props.mass * density + DENSITY_PADDING;
    near_density = fluid_props.mass * near_density + DENSITY_PADDING;
    particles[index].density = vec2(density, near_density);

    // Convert density to pressure
    let pressure = fluid_props.pressure_scalar * (density - fluid_props.target_density);
    let near_pressure = fluid_props.near_pressure_scalar * near_density;
    particles[index].pressure = vec2(pressure, near_pressure);
}
