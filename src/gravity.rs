use bevy::prelude::*;
use bevy::core::Pod;
use bevy_app_compute::prelude::*;
use bytemuck::Zeroable;

const GRAVITY_FORCE: f32 = 9.8;


#[derive(Resource, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
pub struct Gravity {
    pub value: Vec2,
}


impl Gravity {
    pub fn new(value: Vec2) -> Self { Self { value } }

    pub fn set_zero(&mut self) {
        self.value = Vec2::ZERO;
    }

    pub fn set_default(&mut self) {
        self.value = Vec2::new(0., -GRAVITY_FORCE);
    }
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
