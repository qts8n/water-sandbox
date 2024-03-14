use bevy::prelude::*;

use crate::schedule::InGameSet;

const FLUID_CONTAINER_SIZE: Vec2 = Vec2::new(800., 640.);
const FLUID_CONTAINER_POSITION: Vec2 = Vec2::ZERO;


#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct FluidContainerGizmo;


#[derive(Resource, Debug)]
pub struct FluidContainer {
    pub position: Vec2,
    pub size: Vec2,
}


impl FluidContainer {
    fn new(position: Vec2, size: Vec2) -> Self {
        Self {
            position,
            size,
        }
    }

    pub fn get_extents(&self) -> (Vec2, Vec2) {
        let half_size = self.size / 2.;
        (
            Vec2::new(self.position.x - half_size.x, self.position.y - half_size.y),
            Vec2::new(self.position.x + half_size.x, self.position.y + half_size.y),
        )
    }
}


impl Default for FluidContainer {
    fn default() -> Self {
        Self::new(FLUID_CONTAINER_POSITION, FLUID_CONTAINER_SIZE)
    }
}


pub struct GizmoPlugin;


impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_gizmo_group::<FluidContainerGizmo>()
            .init_resource::<FluidContainer>()
            .add_systems(Update, draw_gizmos.in_set(InGameSet::EntityUpdates));
    }
}


fn draw_gizmos(mut border_gizmos: Gizmos<FluidContainerGizmo>, container: Res<FluidContainer>) {
    border_gizmos.rect_2d(container.position, 0., container.size, Color::WHITE);
}
