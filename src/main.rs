use bevy::prelude::*;

use thesilentreach::persistence::PersistencePlugin;
use thesilentreach::player::PlayerPlugin;

use bevy::render::renderer::RenderAdapter;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use thesilentreach::universe::{RenderConfig, RenderMode, UniversePlugin};

mod effects;

fn main() {
    // CLI Args Parsing
    let args: Vec<String> = std::env::args().collect();
    let mut render_mode = RenderMode::Baked;
    if args.contains(&"--procedural".to_string()) {
        render_mode = RenderMode::Procedural;
        println!("RENDER MODE: PROCEDURAL (High Detail) 💎");
    } else {
        println!("RENDER MODE: BAKED (High Performance) 🌍");
    }

    let scenario = if let Some(pos) = args.iter().position(|x| x == "--scenario") {
        args.get(pos + 1)
            .cloned()
            .unwrap_or_else(|| "default".to_string())
    } else {
        "default".to_string()
    };
    if scenario != "default" {
        println!("SCENARIO: {} 🌌", scenario.to_uppercase());
    }

    let force_origin = args.contains(&"--origin".to_string());
    if force_origin {
        println!("COMMAND: TELEPORT TO ORIGIN 🚀");
    }

    // 1. Setup non-blocking writer
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    // 2. Configure our own subscriber
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,wgpu_core=warn,wgpu_hal=warn,bevy_hierarchy=error")
    });

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    App::new()
        .insert_resource(RenderConfig { mode: render_mode })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (450.0, 850.0).into(),
                        title: "The Silent Reach".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<bevy::log::LogPlugin>(),
        )
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(UniversePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(effects::warp::WarpPlugin)
        .add_plugins(PersistencePlugin {
            scenario,
            force_origin,
        })
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
