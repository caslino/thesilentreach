use bevy::prelude::*;
use big_space::{BigSpacePlugin, ReferenceFrame, BigSpaceCommands};

pub struct UniversePlugin;

pub mod spawner;

#[derive(Resource)]
pub struct GameWorld(pub Entity);

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(BigSpacePlugin::<i64>::default())
            .add_systems(PreStartup, setup_universe)
            .add_plugins(spawner::StarSystemSpawnerPlugin);
    }
}

pub fn setup_universe(mut commands: Commands) {
    commands.spawn_big_space(ReferenceFrame::<i64>::default(), |_| {});
}
