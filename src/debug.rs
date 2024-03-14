use bevy::prelude::*;

pub struct DebugPlugin;


impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, log_debug_presence);
    }
}


fn log_debug_presence() {
    println!("[DEBUG] INFO log: Debugger is active for this session!");
}
