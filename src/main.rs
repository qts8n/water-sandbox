mod state;
mod schedule;
mod debug;
mod camera;
mod hud;
mod fluid_container;
mod field;
mod gravity;
mod fluid;

use bevy::prelude::*;

use state::StatePlugin;
use schedule::SchedulePlugin;
use debug::DebugPlugin;
use camera::CameraPlugin;
use hud::HudPlugin;
use fluid_container::GizmoPlugin;
use field::FieldPlugin;
use gravity::GravityPlugin;
use fluid::FluidPlugin;


fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Misc.
            StatePlugin,
            SchedulePlugin,
            DebugPlugin,
            // World defaults
            CameraPlugin,
            HudPlugin,
            GizmoPlugin,
            FieldPlugin,
            GravityPlugin,
            // Game logic
            FluidPlugin,
        ))
        .run();
}
