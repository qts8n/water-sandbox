use bevy::{
    prelude::*,
    transform::TransformSystem,
    ecs::schedule::ScheduleBuildSettings,
};

use crate::state::GameState;


#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum InGameSet {
    UserInput,
    EntityUpdates,
    DespawnEntities,
}


#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum PhysicsSet {
    PropertyUpdates,
    PositionUpdates,
    CollisionDetection,
}


pub struct SchedulePlugin;


impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app
            .configure_schedules(ScheduleBuildSettings {
                auto_insert_apply_deferred: false, // Manually configure flush points
                ..default()
            })
            .configure_sets(Update, (
                InGameSet::DespawnEntities,
                // Flush point [#1] goes here
                InGameSet::UserInput,
                InGameSet::EntityUpdates,
            ).chain().run_if(in_state(GameState::InGame)))
            .configure_sets(PostUpdate, (
                PhysicsSet::PropertyUpdates,
                PhysicsSet::PositionUpdates,
                // Flush point [#2] goes here
                PhysicsSet::CollisionDetection,
            ).chain().before(TransformSystem::TransformPropagate).run_if(in_state(GameState::InGame)))
            // Insert a flush point [#1]
            .add_systems(Update, apply_deferred.after(InGameSet::DespawnEntities).before(InGameSet::UserInput))
            // Insert a flush point [#2]
            .add_systems(PostUpdate, apply_deferred.after(PhysicsSet::PositionUpdates).before(PhysicsSet::CollisionDetection));
    }
}
