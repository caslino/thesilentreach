use bevy::prelude::*;
use thesilentreach::universe::UniversePlugin;
use thesilentreach::player::PlayerPlugin;
use thesilentreach::persistence::PersistencePlugin;

use bevy::render::renderer::RenderAdapter;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (450.0, 850.0).into(),
                title: "The Silent Reach".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(UniversePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(PersistencePlugin)
        .add_systems(Startup, check_gpu)
        .run();
}

fn check_gpu(adapter: Res<RenderAdapter>) {
    let info = adapter.get_info();
    info!("---------------------SYSTEM INFO-----------------------------");
    info!("GPU ADAPTER: {}", info.name);
    info!("BACKEND: {:?}", info.backend);
    info!("DRIVER: {}", info.driver_info);
    info!("TYPE: {:?}", info.device_type);
    info!("---------------------SYSTEM INFO-----------------------------");
}
