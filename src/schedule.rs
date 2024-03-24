use bevy::prelude::*;

use crate::state::GameState;


#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum InGameSet {
    UserInput,
    EntityUpdates,
    DespawnEntities,
}


#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum ShaderPhysicsSet {
    Prepare,
    Pass,
}


pub struct SchedulePlugin;


impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_sets(Update, (
                InGameSet::DespawnEntities,
                InGameSet::UserInput,
                InGameSet::EntityUpdates,
            ).chain().run_if(in_state(GameState::InGame)))
            .configure_sets(PostUpdate, (
                ShaderPhysicsSet::Prepare,
                ShaderPhysicsSet::Pass,
            ).chain().run_if(in_state(GameState::InGame)));
    }
}
