// Misc.
mod smoothing;
mod helpers;

// App
mod state;
mod schedule;
mod debug;
mod camera;
mod menu;
mod hud;
mod fluid_container;
mod field;
mod gravity;
mod fluid;

use bevy::prelude::*;

use menu::MenuPlugin;
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
            MenuPlugin,
            HudPlugin,
            GizmoPlugin,
            FieldPlugin,
            GravityPlugin,
            // Game logic
            FluidPlugin,
        ))
        .run();
}
