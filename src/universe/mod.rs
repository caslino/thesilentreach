use bevy::prelude::*;
use big_space::{BigSpaceCommands, BigSpacePlugin, ReferenceFrame};
use serde::{Deserialize, Serialize};

pub struct UniversePlugin;

pub mod gpu_star_renderer;
pub mod materials;
pub mod nebula;
pub mod physics;
pub mod planet_baker;
pub mod pulsar;
pub mod sky;
pub mod spawner;
pub mod star_common;
pub mod terrain;

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
    pub star_type: StarType,
    pub color: Color,
    pub size: f32,
    pub planets: Option<Vec<DetailedPlanet>>,
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize, Reflect, PartialEq, Eq)]
pub enum StarType {
    BlueGiant,
    YellowDwarf,
    RedDwarf,
    // NeutronStar, // Future expansion?
}

impl StarType {
    pub fn get_base_color(&self) -> Color {
        match self {
            StarType::BlueGiant => Color::srgb(0.2, 0.4, 1.0), // Deep Hot Blue
            StarType::YellowDwarf => Color::srgb(1.0, 0.9, 0.6), // Sun-like
            StarType::RedDwarf => Color::srgb(1.0, 0.3, 0.1),  // Dim Red
        }
    }

    pub fn get_size_range(&self) -> (f32, f32) {
        match self {
            StarType::BlueGiant => (80.0, 150.0),
            StarType::YellowDwarf => (40.0, 70.0),
            StarType::RedDwarf => (15.0, 35.0),
        }
    }

    pub fn get_light_intensity(&self) -> f32 {
        match self {
            StarType::BlueGiant => 50_000_000_000.0,
            StarType::YellowDwarf => 10_000_000_000.0,
            StarType::RedDwarf => 2_000_000_000.0,
        }
    }

    pub fn get_light_range(&self) -> f32 {
        match self {
            StarType::BlueGiant => 5_000_000.0,
            StarType::YellowDwarf => 2_000_000.0,
            StarType::RedDwarf => 800_000.0,
        }
    }
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
            .add_plugins(planet_baker::PlanetBakerPlugin)
            .add_plugins(terrain::TerrainPlugin)
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

pub fn get_solar_system_data() -> Vec<(big_space::GridCell<i64>, StarDetails)> {
    let mut systems = Vec::new();
    let center = big_space::GridCell::new(0, 0, 0);

    let planets = vec![
        DetailedPlanet {
            name: "Mercury".to_string(),
            planet_type: PlanetType::Magma, // Hot, no atmos
            distance: 4000.0,
            size: 10.0,
            color: Color::srgb(0.6, 0.6, 0.6), // Greyish
            second_color: Some(Color::srgb(0.4, 0.4, 0.4)),
            atmosphere_color: None,
            atmosphere_density: None,
            orbit_speed: 0.047,
        },
        DetailedPlanet {
            name: "Venus".to_string(),
            planet_type: PlanetType::Magma, // Hot, thick atmos
            distance: 7000.0,
            size: 25.0,
            color: Color::srgb(0.9, 0.8, 0.5), // Yellowish
            second_color: Some(Color::srgb(0.7, 0.6, 0.3)),
            atmosphere_color: Some(Color::srgb(1.0, 0.9, 0.4)),
            atmosphere_density: Some(2.0),
            orbit_speed: 0.035,
        },
        DetailedPlanet {
            name: "Earth".to_string(),
            planet_type: PlanetType::Terran,
            distance: 10000.0,
            size: 26.0,
            color: Color::srgb(0.1, 0.4, 0.8), // Blue Marble
            second_color: Some(Color::srgb(0.2, 0.6, 0.2)), // Green
            atmosphere_color: Some(Color::srgb(0.4, 0.6, 1.0)),
            atmosphere_density: Some(1.0),
            orbit_speed: 0.029,
        },
        DetailedPlanet {
            name: "Mars".to_string(),
            planet_type: PlanetType::Terran, // Using Terran pattern for visuals but Red
            distance: 15000.0,
            size: 14.0,
            color: Color::srgb(0.8, 0.3, 0.1), // Rusty Red
            second_color: Some(Color::srgb(0.6, 0.2, 0.0)),
            atmosphere_color: Some(Color::srgb(0.9, 0.6, 0.4)),
            atmosphere_density: Some(0.2), // Thin
            orbit_speed: 0.024,
        },
        DetailedPlanet {
            name: "Jupiter".to_string(),
            planet_type: PlanetType::GasGiant,
            distance: 25000.0,
            size: 80.0,
            color: Color::srgb(0.8, 0.7, 0.6),              // Beige
            second_color: Some(Color::srgb(0.6, 0.5, 0.4)), // Bands
            atmosphere_color: Some(Color::srgb(0.7, 0.6, 0.5)),
            atmosphere_density: Some(1.5),
            orbit_speed: 0.013,
        },
        DetailedPlanet {
            name: "Saturn".to_string(),
            planet_type: PlanetType::GasGiant,
            distance: 35000.0,
            size: 70.0,
            color: Color::srgb(0.9, 0.8, 0.6), // Yellowish Beige
            second_color: Some(Color::srgb(0.7, 0.6, 0.4)),
            atmosphere_color: Some(Color::srgb(0.8, 0.7, 0.5)),
            atmosphere_density: Some(1.3),
            orbit_speed: 0.009,
        },
        DetailedPlanet {
            name: "Uranus".to_string(),
            planet_type: PlanetType::Ice,
            distance: 42000.0,
            size: 40.0,
            color: Color::srgb(0.6, 0.8, 0.9), // Pale Cyan
            second_color: Some(Color::srgb(0.5, 0.7, 0.9)),
            atmosphere_color: Some(Color::srgb(0.6, 0.9, 1.0)),
            atmosphere_density: Some(0.8),
            orbit_speed: 0.006,
        },
        DetailedPlanet {
            name: "Neptune".to_string(),
            planet_type: PlanetType::Ice,
            distance: 50000.0,
            size: 38.0,
            color: Color::srgb(0.2, 0.3, 0.9), // Deep Blue
            second_color: Some(Color::srgb(0.1, 0.2, 0.8)),
            atmosphere_color: Some(Color::srgb(0.2, 0.4, 0.9)),
            atmosphere_density: Some(0.9),
            orbit_speed: 0.005,
        },
    ];

    systems.push((
        center,
        StarDetails {
            star_type: StarType::YellowDwarf,
            color: Color::srgb(1.0, 1.0, 0.9), // White/Yellow Sun
            size: 150.0,
            planets: Some(planets),
        },
    ));

    systems
}

pub fn get_jupiter_system_data() -> Vec<(big_space::GridCell<i64>, StarDetails)> {
    let mut systems = Vec::new();
    let center = big_space::GridCell::new(0, 0, 0);

    // 1. Jupiter (The Primary)
    // Modeled as a PlanetType::GasGiant but we place it as the "Star" of this system logic for simplicity,
    // OR we spawn a dummy Star (Sun far away?) and Jupiter as a planet?
    // The prompt says: "The Primary (Jupiter): Location: GridCell(0,0,0). Visuals: Use PlanetType::GasGiant. Size 250.0"
    // Usually (0,0,0) is a StarDetails. StarDetails has `star_type`.
    // But we want it to look like a Gas Giant.
    // If we use StarDetails, it renders with `StarMaterial` (emissive).
    // The prompt implies we should see Jupiter.
    // However, the spawner uses `StarDetails` to create the root.
    // IF we want Jupiter to be the central object at (0,0,0) and render as a planet, we might need a hack or
    // simply define it as a Planet in the list, and have a dummy central star (or no star?).
    //
    // Re-reading Prompt: "The Primary (Jupiter): Location GridCell(0,0,0)... Visuals Use PlanetType::GasGiant"
    // And "Populate the system with the Galilean moons for scale".
    //
    // The `Spawner` spawns `StarDetails` at the center using `StarMaterial` (Unit Sphere High).
    // Planets are children.
    // If I want Jupiter to be the big thing in the middle, I might need to make it a "Planet" at distance 0?
    //
    // Let's create a "Star" that is invisible/small or just the Sun far away?
    // Actually, `StarDetails` struct doesn't have `PlanetType`.
    // It has `StarType`.
    //
    // PROMPT SPECIFIC INSTRUCTION: "The Primary (Jupiter): Location GridCell(0,0,0)..."
    //
    // If I put it in `planets` list with distance 0, it will spawn inside the star.
    //
    // Let's look at `StarDetails`. It has `planets: Option<Vec<DetailedPlanet>>`.
    // I will define a "Dummy" Star at (0,0,0) that is tiny/invisible?
    // OR I will just spawn Jupiter as a "Planet" at distance 0.
    //
    // Let's try spawning a Dummy Star (Sun, distant light source?) and Jupiter as the main Planet.
    // But the prompt says "Spawn directly in front of a massive... gas giant".
    // If I spawn at (0,0,0), I am inside it.
    //
    // Start with a 'Sun' (Light source) effectively acting as the Star, and Jupiter orbit it?
    // "Jupiter Scenario" usually implies we are AT Jupiter.
    //
    // Let's define the System:
    // Star: The Sun (Far away? or just central light?)
    // Actually, the Light comes from the Star.
    // If Jupiter is the "Primary", it shouldn't emit light like a star.
    //
    // I will define the Central Object as a Dummy Star (maybe "Sun (Distant)")
    // And Jupiter as a Planet at a small distance or 0.
    //
    // Let's define:
    // Star: name="Sun (Distant)", type=YellowDwarf, size=1.0 (tiny), distance=0.
    // Planets:
    //  - Jupiter (Dist 0 or small offset)
    //  - Moons (Orbiting Jupiter? Our system only supports Planets orbiting Star)
    //
    // Ah, our system `DetailedPlanet` does not support moons of planets.
    // The prompt says "The Galilean Moons (For Scale): Io: Distance ~1,200..."
    //
    // It seems the intent is to simulate the "Jupiter System" as if Jupiter is the "Star" (Gravity well)
    // but rendered as a Gas Giant.
    //
    // Issue: `spawner.rs` spawns the central object as `Star` with `StarMaterial`.
    // `StarMaterial` is unlit emissive.
    // `PlanetMaterial` is lit.
    //
    // If I make Jupiter the "Star" entry, it will use Star shader (glowing). Gas Giants shouldn't glow.
    //
    // SOLUTION:
    // I will make the "Star" entry invisible/dummy.
    // I will add Jupiter as a "Planet" at Distance ~0 (or very close).
    // I will add Moons as "Planets" at their respective distances.
    // Effectively, the "Star" entity is just the coordinate anchor and light source (The Sun).
    //
    // Star: "Sun (Light Source)", Size 100, but maybe placed far away?
    // Wait, `StarDetails` defines the generic system.
    //
    // Let's assume the user wants Jupiter at (0,0,0).
    // I will create:
    // Star: "Jupiter System Center" (Invisible/Small)
    // Planet 1: Jupiter (Dist 0, Size 250)
    // Planet 2: Io (Dist 1200)
    // Planet 3: Europa (Dist 1800)
    // Planet 4: Ganymede (Dist 2800)
    // Planet 5: Callisto (Dist 3800)
    //
    // This allows Moons to orbit constraints (which are relative to (0,0,0)).
    // Jupiter will be at (0,0,0).
    //
    // One catch: `spawn_star_with_data` spawns a PointLight at the center.
    // If Jupiter is there, the light will be inside it.
    // Jupiter is not a star, so it shouldn't emit light. The light should come from the Sun.
    //
    // I might need to hack the "Star" to be the Sun, but effectively place Jupiter at the center?
    // But if (0,0,0) is the Sun...
    //
    // Let's just follow the data structure.
    // I will define a "Jupiter" Planet at distance 0.
    // And the moons at their distances.
    // The "Star" will be "Sun Intensity" but maybe I need to accept the light source is at (0,0,0)...
    //
    // If the light is at (0,0,0), Jupiter (at 0,0,0) will be lit from inside?
    // And moons will be lit from the center (Jupiter).
    //
    // Actually, in the real Jupiter system, the light comes from the Sun (far away).
    // But in this "Jupiter Scenario", maybe we just want to see Jupiter and moons.
    // The Prompt says: "The Primary (Jupiter): Location GridCell(0,0,0)".
    //
    // If I put Jupiter at 0,0,0 and the Light at 0,0,0, it will look weird (Moons lit from Jupiter).
    //
    // Maybe I should offset the Light?
    // `StarDetails` has `color` and `size`. Light intensity is derived from `StarType`.
    //
    // I will trick it:
    // I'll set the "Star" to be the Sun, but place it far away?
    // No, `spawner.rs` spawns Star at `cell` (0,0,0).
    //
    // If I modify `spawner.rs` to support `PlanetType` for the Central Body, that would be ideal.
    // BUT `spawner.rs` logic is "Spawn Star Sphere".
    //
    // Let's stick to the "Planets List" approach.
    // I will make the Star "Dark/Invisible"?
    // And put a "Sun" as a distant planet? No, planets don't emit light.
    //
    // Let's Look at `spawner.rs`:
    // Line 165: `let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0)); // Fake Sun` in `planet.wgsl`!!!
    //
    // OH! logic in `planet.wgsl` Line 165 uses a **FAKE SUN DIR**.
    // `let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0)); // Fake Sun`
    //
    // SO, the PointLight in `spawner.rs` does NOT affect the planets (which use `planet.wgsl`).
    // `planet.wgsl` ignores real lights and uses a hardcoded direction!
    //
    // This simplifies everything! I don't need to worry about the PointLight position for the planets' shading.
    //
    // Valid Plan:
    // Star: "Jupiter Gravity Well". Size: 1.0 (Tiny/Invisible). Color: Black?
    // Planets:
    //  - Jupiter: Dist 0. Size 250.
    //  - Moons: Distances as requested.
    //
    // This works.

    let mut planets = Vec::new();

    // Jupiter
    planets.push(DetailedPlanet {
        name: "Jupiter".to_string(),
        planet_type: PlanetType::GasGiant,
        distance: 0.1, // Near zero to be at center
        size: 250.0,
        color: Color::srgb_u8(227, 220, 203), // E3DCCB
        second_color: Some(Color::srgb_u8(200, 139, 58)), // C88B3A
        atmosphere_color: Some(Color::srgb_u8(200, 150, 100)),
        atmosphere_density: Some(1.0),
        orbit_speed: 0.0,
    });

    // Io
    planets.push(DetailedPlanet {
        name: "Io".to_string(),
        planet_type: PlanetType::Magma, // Volcanic
        distance: 1200.0,
        size: 12.0,
        color: Color::srgb_u8(216, 211, 85), // D8D355
        second_color: Some(Color::srgb_u8(200, 100, 0)),
        atmosphere_color: Some(Color::srgb(0.8, 0.8, 0.2)),
        atmosphere_density: Some(0.5),
        orbit_speed: 0.05,
    });

    // Europa
    planets.push(DetailedPlanet {
        name: "Europa".to_string(),
        planet_type: PlanetType::Ice,
        distance: 1800.0,
        size: 10.0,
        color: Color::srgb_u8(240, 248, 255), // F0F8FF
        second_color: Some(Color::srgb_u8(200, 220, 255)),
        atmosphere_color: Some(Color::srgb(0.8, 0.9, 1.0)),
        atmosphere_density: Some(0.2),
        orbit_speed: 0.15,
    });

    // Ganymede
    planets.push(DetailedPlanet {
        name: "Ganymede".to_string(),
        planet_type: PlanetType::Terran, // Rocky Grey -> Use Terran with Grey colors? Or Ice? Prompt says "Rocky Grey". Terran is usually Green/Blue. I'll use Ice or just override colors.
        // Prompt says "Rocky Grey (#7C756F)".
        // PlanetType affects Palette. But `spawn_star_with_data` uses `planet_data.color` primarily.
        distance: 2800.0,
        size: 18.0,
        color: Color::srgb_u8(124, 117, 111), // 7C756F
        second_color: Some(Color::srgb_u8(100, 90, 80)),
        atmosphere_color: None,
        atmosphere_density: None,
        orbit_speed: 0.1,
    });

    // Callisto
    planets.push(DetailedPlanet {
        name: "Callisto".to_string(),
        planet_type: PlanetType::Ice, // Dark Pockmarked.
        distance: 3800.0,
        size: 16.0,
        color: Color::srgb_u8(75, 75, 75), // 4B4B4B
        second_color: Some(Color::srgb_u8(50, 50, 50)),
        atmosphere_color: None,
        atmosphere_density: None,
        orbit_speed: 0.3,
    });

    systems.push((
        center,
        StarDetails {
            star_type: StarType::RedDwarf, // Dim star as placeholder
            color: Color::NONE,            // Invisible? Text2d might still show.
            size: 1.0,
            planets: Some(planets),
        },
    ));

    systems
}
