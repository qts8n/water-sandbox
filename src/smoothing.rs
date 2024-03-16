use std::f32::consts::PI;


pub const DENSITY_PADDING: f32 = 0.00001;


pub fn smoothing_kernel(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }
    let volume = 6. / (PI * radius.powi(4));
    let v = radius - distance;
    v * v * volume
}


pub fn smoothing_kernel_derivative(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }

    // Slope calculation
    let scale = 12. / (PI * radius.powi(4));
    (distance - radius) * scale
}


pub fn smoothing_kernel_near(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }
    let volume = 10. / (PI * radius.powi(5));
    let v = radius - distance;
    v * v * v * volume
}


pub fn smoothing_kernel_derivative_near(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }

    // Slope calculation
    let scale = 30. / (PI * radius.powi(5));
    let v = distance - radius;
    v * v * scale
}

pub fn smoothing_kernel_viscosity(radius: f32, distance: f32) -> f32 {
    if distance > radius {
        return 0.;
    }
    let volume = 4. / (PI * radius.powi(8));
    let v = radius * radius - distance * distance;
    v * v * v * volume
}
