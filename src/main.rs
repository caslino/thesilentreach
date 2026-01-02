use bevy::prelude::*;
use thesilentreach::universe::UniversePlugin;
use thesilentreach::player::PlayerPlugin;
use thesilentreach::persistence::PersistencePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(UniversePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(PersistencePlugin)
        .run();
}
