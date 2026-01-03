use bevy::prelude::*;
use thesilentreach::universe::UniversePlugin;
use thesilentreach::player::PlayerPlugin;
use thesilentreach::persistence::PersistencePlugin;

use bevy::render::renderer::RenderAdapter;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    // 1. Setup non-blocking writer
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    // 2. Configure our own subscriber
    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,bevy_hierarchy=error"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (450.0, 850.0).into(),
                title: "The Silent Reach".to_string(),
                ..default()
            }),
            ..default()
        })
        .disable::<bevy::log::LogPlugin>())
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
