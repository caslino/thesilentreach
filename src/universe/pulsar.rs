use crate::universe::UniverseSeed;
use bevy::prelude::*;
use big_space::GridCell;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, Debug)]
pub struct Pulsar {
    pub position: GridCell<i64>,
    pub frequency: f32,
}

#[derive(Resource, Default)]
pub struct PulsarMap {
    pub pulsars: Vec<Pulsar>,
}

pub struct PulsarPlugin;

impl Plugin for PulsarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PulsarMap>()
            .add_systems(Startup, generate_pulsars);
    }
}

fn generate_pulsars(seed: Res<UniverseSeed>, mut pulsar_map: ResMut<PulsarMap>) {
    let mut rng = StdRng::seed_from_u64(seed.0);

    // Generate a fixed number of pulsars scattered in a large volume
    // The previous hardcoded values were around +/- 100,000 range.
    // Let's generate 20 pulsars in a wider range to simulate a larger universe sector.

    let count = 20;
    let range = 500_000;

    pulsar_map.pulsars.clear();

    for _ in 0..count {
        let x = rng.gen_range(-range..=range);
        let y = rng.gen_range(-range..=range);
        let z = rng.gen_range(-range..=range);

        let frequency = rng.gen_range(0.1..2.0); // 0.1Hz to 2.0Hz

        pulsar_map.pulsars.push(Pulsar {
            position: GridCell::new(x, y, z),
            frequency,
        });
    }

    info!("Generated {} pulsars", pulsar_map.pulsars.len());
}
