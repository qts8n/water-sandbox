use bevy::math::Vec3;

pub fn cube_fluid(ni: usize, nj: usize, nk: usize, particle_rad: f32) -> Vec<Vec3> {
    let mut points = Vec::new();
    let half_extents = Vec3::new(ni as f32, nj as f32, nk as f32) * particle_rad;
    let offset = Vec3::new(particle_rad, particle_rad, particle_rad) - half_extents;
    let diam = particle_rad * 2.;
    for i in 0..ni {
        let x = (i as f32) * diam;
        for j in 0..nj {
            let y = (j as f32) * diam;
            for k in 0..nk {
                let z = (k as f32) * diam;
                points.push(Vec3::new(x, y, z) + offset);
            }
        }
    }

    points
}
