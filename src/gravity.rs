use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

const GRAVITY_FORCE: f32 = 9.8;


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Gravity {
    pub value: Vec3,
}


impl Gravity {
    pub fn new(value: Vec3) -> Self { Self { value } }

    pub fn set_zero(&mut self) {
        self.value = Vec3::ZERO;
    }

    pub fn set_default(&mut self) {
        self.value = Vec3::new(0., -GRAVITY_FORCE, 0.);
    }
}


impl Default for Gravity {
    fn default() -> Self {
        Self::new(Vec3::new(0., -GRAVITY_FORCE, 0.))
    }
}


pub struct GravityPlugin;


impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gravity>();
    }
}
