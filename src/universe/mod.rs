use bevy::prelude::*;
use big_space::{BigSpaceCommands, BigSpacePlugin, ReferenceFrame};
use serde::{Deserialize, Serialize};

pub struct UniversePlugin;

pub mod gpu_star_renderer;
pub mod materials;
pub mod physics;
pub mod pulsar;
pub mod sky;
pub mod spawner;
pub mod star_common;

use self::materials::{PlanetMaterial, StarMaterial};

#[derive(Resource)]
pub struct GameWorld(pub Entity);

#[derive(Resource, Clone, Copy, Debug)]
pub struct UniverseSeed(pub u64);

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Procedural,
    #[default]
    Baked,
}

#[derive(Resource, Debug, Clone)]
pub struct UniverseConfig {
    pub scenario_name: String,
    pub active_seed: u64,
}

#[derive(Resource, Default, Debug)]
pub struct RenderConfig {
    pub mode: RenderMode,
}

#[derive(Component)]
pub struct Mass(pub f32);

#[derive(Component)]
pub struct Radius(pub f32);

#[derive(Component)]
pub struct Star;

#[derive(Component)]
pub struct Planet;

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Orbit {
    pub radius: f32,
    pub speed: f32,
    pub angle: f32,
}

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct StarDetails {
    pub color: Color,
    pub size: f32,
    pub planets: Option<Vec<DetailedPlanet>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct DetailedPlanet {
    pub name: String,
    pub planet_type: PlanetType,
    pub distance: f32,
    pub size: f32,
    pub color: Color,
    pub second_color: Option<Color>,
    pub atmosphere_color: Option<Color>,
    pub atmosphere_density: Option<f32>,
    pub orbit_speed: f32,
}

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize, Reflect, Default)]
#[reflect(Component, Default)]
pub enum PlanetType {
    #[default]
    Terran,
    Ice,
    Magma,
    GasGiant,
}

impl PlanetType {
    pub fn from_seed(seed: f32) -> Self {
        let n = seed.fract();
        if n < 0.3 {
            PlanetType::Ice
        } else if n < 0.6 {
            PlanetType::GasGiant
        } else if n < 0.8 {
            PlanetType::Terran
        } else {
            PlanetType::Magma
        }
    }

    pub fn get_palette(&self) -> (LinearRgba, LinearRgba) {
        match self {
            PlanetType::Terran => (
                LinearRgba::from(Color::srgb(0.2, 0.8, 0.2)),
                LinearRgba::from(Color::srgb(0.1, 0.5, 0.1)),
            ), // Green/Forest
            PlanetType::Ice => (
                LinearRgba::from(Color::srgb(0.8, 0.9, 1.0)),
                LinearRgba::from(Color::srgb(0.5, 0.7, 0.9)),
            ), // White/Cyan
            PlanetType::Magma => (
                LinearRgba::from(Color::srgb(1.0, 0.2, 0.0)),
                LinearRgba::from(Color::srgb(0.5, 0.0, 0.0)),
            ), // Red/DarkRed
            PlanetType::GasGiant => (
                LinearRgba::from(Color::srgb(0.8, 0.6, 0.4)),
                LinearRgba::from(Color::srgb(0.5, 0.3, 0.2)),
            ), // Beige/Brown
        }
    }

    pub fn description(&self) -> String {
        match self {
            PlanetType::Terran => "Class: Terran | Atmos: N2/O2 | Gravity: 1.0g".to_string(),
            PlanetType::Ice => "Class: Frozen | Atmos: Thin CO2 | Gravity: 0.6g".to_string(),
            PlanetType::Magma => "Class: Volcanic | Atmos: S02 | Gravity: 0.9g".to_string(),
            PlanetType::GasGiant => "Class: Gas Giant | Atmos: H2/He | Gravity: 4.5g".to_string(),
        }
    }

    pub fn get_atmosphere_color(&self) -> (LinearRgba, f32) {
        match self {
            PlanetType::Terran => (LinearRgba::from(Color::srgb(0.0, 0.4, 0.8)), 1.0), // Blue Atmosphere
            PlanetType::Ice => (LinearRgba::from(Color::srgb(0.6, 0.9, 1.0)), 0.6),    // Cyan mist
            PlanetType::Magma => (LinearRgba::from(Color::srgb(1.0, 0.3, 0.0)), 1.2), // Thick Orange/Red
            PlanetType::GasGiant => (LinearRgba::from(Color::srgb(0.7, 0.5, 0.3)), 1.5), // Heavy Beige
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct PlanetDetails(pub PlanetType);

#[derive(Event)]
pub struct StarClicked {
    pub entity: Entity,
    pub cell: big_space::GridCell<i64>,
}

#[derive(Event)]
pub struct SystemSavedEvent {
    pub name: String,
}

#[cfg(test)]
mod seed_test;

fn get_universe_seed() -> u64 {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|x| x == "--scenario") {
        if let Some(scenario) = args.get(pos + 1) {
            match scenario.as_str() {
                "milky_way" => return 987654321,
                _ => {}
            }
        }
    }

    if let Ok(s) = std::env::var("UNIVERSE_SEED") {
        s.parse::<u64>().unwrap_or_else(|_| {
            warn!("Invalid UNIVERSE_SEED environment variable, using random seed.");
            rand::random()
        })
    } else {
        rand::random()
    }
}

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        let seed = get_universe_seed();
        let args: Vec<String> = std::env::args().collect();
        let scenario_name = if let Some(pos) = args.iter().position(|x| x == "--scenario") {
            args.get(pos + 1)
                .cloned()
                .unwrap_or_else(|| "default".to_string())
        } else {
            "default".to_string()
        };

        info!("Universe Scenario: {}, Seed: {}", scenario_name, seed);

        app.insert_resource(UniverseSeed(seed))
            .insert_resource(UniverseConfig {
                scenario_name,
                active_seed: seed,
            })
            .add_plugins(BigSpacePlugin::<i64>::default())
            .add_systems(PreStartup, setup_universe)
            .add_plugins(spawner::StarSystemSpawnerPlugin)
            .add_plugins(sky::SkyPlugin)
            .add_plugins(gpu_star_renderer::GPUStarPlugin)
            .add_plugins(pulsar::PulsarPlugin)
            .add_plugins(MaterialPlugin::<StarMaterial>::default())
            .add_plugins(MaterialPlugin::<PlanetMaterial>::default())
            .add_event::<StarClicked>()
            .add_event::<SystemSavedEvent>()
            // Global Ambient Light so things aren't pitch black
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 80.0,
            });
    }
}

pub fn setup_universe(mut commands: Commands) {
    commands.spawn_big_space(ReferenceFrame::<i64>::default(), |_| {});
}

pub const SECTOR_SIZE: i64 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SectorIndex {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl SectorIndex {
    pub fn from_cell(cell: big_space::GridCell<i64>) -> Self {
        Self {
            x: cell.x.div_euclid(SECTOR_SIZE),
            y: cell.y.div_euclid(SECTOR_SIZE),
            z: cell.z.div_euclid(SECTOR_SIZE),
        }
    }
}
