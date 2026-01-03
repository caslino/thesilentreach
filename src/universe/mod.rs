use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use big_space::{BigSpacePlugin, ReferenceFrame, BigSpaceCommands};

pub struct UniversePlugin;

pub mod spawner;
pub mod physics;
pub mod sky;
pub mod materials;

use self::materials::{StarMaterial, PlanetMaterial};

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

#[derive(Component, Clone, Debug, Serialize, Deserialize)]
pub struct StarDetails {
    pub color: Color,
    pub size: f32,
}

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PlanetType {
    Terran,
    Ice,
    Magma,
    GasGiant,
}

impl PlanetType {
    pub fn from_seed(seed: f32) -> Self {
        let n = seed.fract();
        if n < 0.3 { PlanetType::Ice }
        else if n < 0.6 { PlanetType::GasGiant }
        else if n < 0.8 { PlanetType::Terran }
        else { PlanetType::Magma }
    }

    pub fn get_palette(&self) -> (LinearRgba, LinearRgba) {
        match self {
            PlanetType::Terran => (LinearRgba::from(Color::srgb(0.2, 0.8, 0.2)), LinearRgba::from(Color::srgb(0.1, 0.5, 0.1))), // Green/Forest
            PlanetType::Ice => (LinearRgba::from(Color::srgb(0.8, 0.9, 1.0)), LinearRgba::from(Color::srgb(0.5, 0.7, 0.9))), // White/Cyan
            PlanetType::Magma => (LinearRgba::from(Color::srgb(1.0, 0.2, 0.0)), LinearRgba::from(Color::srgb(0.5, 0.0, 0.0))), // Red/DarkRed
            PlanetType::GasGiant => (LinearRgba::from(Color::srgb(0.8, 0.6, 0.4)), LinearRgba::from(Color::srgb(0.5, 0.3, 0.2))), // Beige/Brown
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

impl Plugin for UniversePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(UniverseSeed(12345)) // Fixed seed for now
            .add_plugins(BigSpacePlugin::<i64>::default())
            .add_systems(PreStartup, setup_universe)
            .add_plugins(spawner::StarSystemSpawnerPlugin)
            .add_plugins(sky::SkyPlugin)
            .add_plugins(MaterialPlugin::<StarMaterial>::default())
            .add_plugins(MaterialPlugin::<PlanetMaterial>::default())
            .add_event::<StarClicked>()
            .add_event::<SystemSavedEvent>();
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
