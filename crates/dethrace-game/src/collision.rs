use bevy::prelude::Resource;
use dethrace_core::collision::StaticCollisionWorld;

#[derive(Resource)]
pub struct TrackCollisionWorld(pub StaticCollisionWorld);

#[derive(Resource, Default)]
pub struct CollisionDebugSettings {
    pub enabled: bool,
}
