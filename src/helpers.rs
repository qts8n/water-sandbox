use bevy::math::Vec2;

pub fn cube_fluid(ni: usize, nj: usize, particle_rad: f32) -> Vec<Vec2> {
    let mut points = Vec::new();
    let half_extents = Vec2::new(ni as f32, nj as f32) * particle_rad;
    let offset = Vec2::new(particle_rad, particle_rad) - half_extents;
    let diam = particle_rad * 2.;
    for i in 0..ni {
        let x = (i as f32) * diam;
        for j in 0..nj {
            let y = (j as f32) * diam;
            points.push(Vec2::new(x, y) + offset);
        }
    }

    points
}
