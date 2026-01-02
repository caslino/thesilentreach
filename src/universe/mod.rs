use bevy::prelude::*;
use big_space::{BigSpacePlugin, ReferenceFrame, BigSpaceCommands};

pub struct UniversePlugin;

pub mod spawner;
pub mod physics;
pub mod sky;

#[derive(Resource)]
pub struct GameWorld(pub Entity);

#[derive(Resource)]
pub struct UniverseSeed(pub u64);

#[derive(Component)]
pub struct Mass(pub f32);

#[derive(Component)]
pub struct Radius(pub f32);

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(UniverseSeed(12345)) // Fixed seed for now
            .add_plugins(BigSpacePlugin::<i64>::default())
            .add_systems(PreStartup, setup_universe)
            .add_plugins(spawner::StarSystemSpawnerPlugin)
            .add_plugins(sky::SkyPlugin);
    }
}

pub fn setup_universe(mut commands: Commands) {
    commands.spawn_big_space(ReferenceFrame::<i64>::default(), |_| {});
}
