use bevy::prelude::*;

const GRAVITY_FORCE: f32 = 10.;


#[derive(Resource, Debug)]
pub struct Gravity {
    pub value: Vec2,
}


impl Gravity {
    fn new(value: Vec2) -> Self { Self { value } }
}


impl Default for Gravity {
    fn default() -> Self {
        Self::new(Vec2::new(0., -GRAVITY_FORCE))
    }
}


pub struct GravityPlugin;


impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gravity>();
    }
}
