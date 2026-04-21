use std::path::PathBuf;

pub mod effects;
pub mod universe;
pub mod persistence;
pub mod player;
pub mod recorder;

/// Detection for macOS bundle asset root
pub fn get_asset_root() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        use std::env;
        if let Ok(exe_path) = env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                // Check if we are inside a .app bundle (Contents/MacOS/...)
                if parent.ends_with("MacOS") {
                    if let Some(contents) = parent.parent() {
                        let resources = contents.join("Resources").join("assets");
                        if resources.exists() {
                            return resources;
                        }
                    }
                }
            }
        }
    }
    "assets".into()
}

/// Main app entry point — called by both desktop (gen/bin/desktop.rs) and mobile targets.
pub fn main() {
    use bevy::prelude::*;
    use bevy::render::renderer::RenderAdapter;
    use universe::{RenderConfig, RenderMode, UniversePlugin};
    use persistence::PersistencePlugin;
    use player::PlayerPlugin;

    eprintln!("--- THE SILENT REACH STARTUP (FIX V1.3) ---");

    // macOS Bundle Path Detection
    let asset_path = get_asset_root().to_string_lossy().to_string();
    if asset_path != "assets" {
        eprintln!("DEBUG: Resolved Bundle Asset Root: {}", asset_path);
    }

    // CLI Args Parsing
    let args: Vec<String> = std::env::args().collect();
    let mut render_mode = RenderMode::Procedural;
    if args.contains(&"--baked".to_string()) {
        render_mode = RenderMode::Baked;
        println!("RENDER MODE: BAKED (High Performance) 🌍");
    } else {
        println!("RENDER MODE: PROCEDURAL (High Detail) 💎");
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

    // Resolution, Directory, and Blink Duration parsing
    let (width, height, output_dir, blink_duration) = if args.contains(&"shorts".to_string()) {
        println!("MODE: SHORTS (1080x1920) 📱");
        (1080.0, 1920.0, "recordings/shorts".to_string(), 1.0)
    } else if args.contains(&"video".to_string()) {
        println!("MODE: VIDEO (1920x1080) 🎥");
        (1920.0, 1080.0, "recordings/videos".to_string(), 2.0)
    } else if args.contains(&"linkedin".to_string()) {
        println!("MODE: LINKEDIN (1080x1350) 👔");
        (1080.0, 1350.0, "recordings/linkedin".to_string(), 1.5)
    } else {
        (450.0, 850.0, ".".to_string(), 0.5)
    };

    let force_origin = args.contains(&"--origin".to_string());
    if force_origin {
        println!("COMMAND: TELEPORT TO ORIGIN 🚀");
    }

    let star_override = if let Some(pos) = args.iter().position(|x| x == "--star") {
        let t = args.get(pos + 1).cloned().unwrap_or_default();
        let result = universe::StarType::from_str(&t);
        if let Some(st) = result {
            println!("COMMAND: STAR OVERRIDE -> {:?} 🌟", st);
        } else {
            println!("WARNING: Unknown star type '{}'", t);
        }
        result
    } else {
        None
    };

    let planet_override = if let Some(pos) = args.iter().position(|x| x == "--planet") {
        let t = args.get(pos + 1).cloned().unwrap_or_default();
        let result = universe::PlanetType::from_str(&t);
        if let Some(pt) = result {
            println!("COMMAND: PLANET OVERRIDE -> {:?} 🌍", pt);
        } else {
            println!("WARNING: Unknown planet type '{}'", t);
        }
        result
    } else {
        None
    };

    App::new()
        .insert_resource(RenderConfig { mode: render_mode })
        .insert_resource(recorder::RecordingDirectory(output_dir))
        .insert_resource(recorder::RecordingBlinkDuration(blink_duration))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (width, height).into(),
                        title: "The Silent Reach".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_path,
                    watch_for_changes_override: Some(cfg!(debug_assertions)),
                    ..default()
                })
                .disable::<bevy::transform::TransformPlugin>()
        )
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(UniversePlugin)
        .add_plugins(PlayerPlugin)
        .add_plugins(effects::warp::WarpPlugin)
        .add_plugins(PersistencePlugin {
            scenario,
            force_origin,
            star_override,
            planet_override,
        })
        .add_plugins(
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            recorder::RecorderPlugin,
            #[cfg(any(target_os = "android", target_os = "ios"))]
            bevy::app::PanicHandlerPlugin,
        )
        .add_systems(Startup, check_gpu)
        .run();
}

fn check_gpu(adapter: bevy::prelude::Res<bevy::render::renderer::RenderAdapter>) {
    let info = adapter.get_info();
    bevy::prelude::info!("---------------------SYSTEM INFO-----------------------------");
    bevy::prelude::info!("GPU ADAPTER: {}", info.name);
    bevy::prelude::info!("BACKEND: {:?}", info.backend);
    bevy::prelude::info!("DRIVER: {}", info.driver_info);
    bevy::prelude::info!("TYPE: {:?}", info.device_type);
    bevy::prelude::info!("---------------------SYSTEM INFO-----------------------------");
}

// Mobile Entry Points
#[cfg(any(target_os = "android", target_os = "ios"))]
#[unsafe(no_mangle)]
pub extern "C" fn start_app() {
    main();
}
