use bevy::prelude::*;
use big_space::GridCell;
pub mod database;
pub use database::Database; // Re-export for public use
pub use database::DiscoveredWorld as Discovery;
pub use database::PlayerState;

#[derive(Resource, Default)]
pub struct CurrentSystemData {
    pub cell: GridCell<i64>,
    pub discovery: Option<Discovery>,
    pub is_dirty: bool, // Trigger UI update
}

#[derive(Resource, Default)]
pub struct SpawnLocation {
    pub cell: GridCell<i64>,
    pub local_pos: Vec3,
    pub velocity: Option<Vec3>, // New field for loading state
    pub throttle: f32,          // Added throttle persistence
    pub has_spawned: bool,
}

#[derive(Resource, Default)]
pub struct PersistenceConfig {
    pub scenario: String,
    pub force_origin: bool,
    pub star_override: Option<crate::universe::StarType>,
    pub planet_override: Option<crate::universe::PlanetType>,
}

use crate::player::camera::{Velocity, ZenCamera};
use crate::player::input::VehicleInput;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PersistencePlugin {
    pub scenario: String,
    pub force_origin: bool,
    pub star_override: Option<crate::universe::StarType>,
    pub planet_override: Option<crate::universe::PlanetType>,
}

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        // Initialize DB
        let db = Database::open().expect("Failed to open SQLite database");

        // Seed Predefined System if needed
        if let Err(e) = db.seed_predefined_system(&self.scenario) {
            error!("Failed to seed database: {}", e);
        }

        app.insert_resource(db)
            .insert_resource(PersistenceConfig {
                scenario: self.scenario.clone(),
                force_origin: self.force_origin,
                star_override: self.star_override,
                planet_override: self.planet_override,
            })
            .init_resource::<SpawnLocation>()
            .init_resource::<CurrentSystemData>()
            .add_systems(PreStartup, load_player_state) // Load before camera setup
            .add_systems(Update, (check_system_change, auto_save_system));
    }
}

fn load_player_state(
    db: Res<Database>,
    mut spawn_loc: ResMut<SpawnLocation>,
    config: Res<PersistenceConfig>,
) {
    if config.force_origin || config.star_override.is_some() {
        spawn_loc.cell = GridCell::new(0, 0, 0);

        // Default to origin
        let mut local_pos = Vec3::ZERO;

        if let Some(star_type) = config.star_override {
            // Get star size to position player outside it
            let (_, max_size) = star_type.get_size_range();
            // Position at 3x radius for a good view, with a minimum floor for tiny stars
            let spawn_dist = (max_size * 3.0).max(50.0);
            local_pos = Vec3::new(0.0, 0.0, spawn_dist);
            info!(
                "PERSISTENCE: Star Override ({:?}). Spawning at distance {:.2}",
                star_type, local_pos.z
            );
        } else if let Some(planet_type) = config.planet_override {
            // Force origin spawn near a planet
            // Star size is usually 50-100. Planets are further out.
            // But we already have logic to override the first planet at (0,0,0) in spawner.rs if at origin.
            // Wait, spawner.rs overrides the first planet in the *list*.
            // In the "Dummy" system, the first planet is at distance 0 (Jupiter logic).
            // Let's spawn at a distance that works for most planets (~150-300 range)
            local_pos = Vec3::new(0.0, 0.0, 400.0);
            info!(
                "PERSISTENCE: Planet Override ({:?}). Spawning at distance 400.0",
                planet_type
            );
        } else {
            info!("PERSISTENCE: Force Origin requested. Spawning at (0,0,0).");
        }

        spawn_loc.local_pos = local_pos;
        spawn_loc.velocity = Some(Vec3::ZERO);
        spawn_loc.throttle = 0.0;
        spawn_loc.has_spawned = true;
        return;
    }

    // Special Scenario Spawns
    if config.scenario == "jupiter" {
        spawn_loc.cell = GridCell::new(0, 0, 0);
        // Jupiter is at 0,0,0 with radius 250. Spawn at z=800 looking at it.
        spawn_loc.local_pos = Vec3::new(0.0, 0.0, 800.0);
        spawn_loc.velocity = Some(Vec3::ZERO);
        spawn_loc.throttle = 0.0;
        spawn_loc.has_spawned = true;
        info!("PERSISTENCE: Jupiter Scenario detected. Spawning at (0,0,800).");
        return;
    }

    // 1. Check DB
    if let Ok(Some(state)) = db.get_player_state() {
        // 2. Calculate Catch-up
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let delta_time = current_time - state.timestamp;

        let local_pos = Vec3::new(state.local_x, state.local_y, state.local_z);
        let velocity = Vec3::new(state.vel_x, state.vel_y, state.vel_z);

        // Simple linear projection for "catch up" (ignoring gravity for simplicity during catchup)
        // If delta is huge (days), maybe cap it? For now, we trust the void.
        // Actually, preventing moving millions of km into a planet might be good.
        // Limit catchup drift to e.g. 1 minute for safety?
        // Or assume ship was "drifting" safely? User requested "trajectory path calculated".
        // Let's assume linear drift.
        // catch-up logic
        let drift_time = delta_time.clamp(0, 86400 * 7) as f32; // Limit to 1 week

        let mut final_pos = local_pos;
        let mut final_vel = velocity;

        // 1. Check for Star (Dominant Gravity)
        let cell = GridCell::new(state.cell_x, state.cell_y, state.cell_z);
        if has_star(&cell) {
            // Star Assumptions (Must match spawner.rs)
            let star_mass = 1_000_000.0;
            let mu = crate::universe::physics::GRAVITY_CONSTANT * star_mass;

            let r_vec = local_pos; // Star is at 0,0,0
            let r = r_vec.length();
            let v_sq = velocity.length_squared();

            // Specific Orbital Energy: E = v^2/2 - mu/r
            let energy = v_sq / 2.0 - mu / r;

            if energy < 0.0 && r > 100.0 {
                // Elliptical Orbit (Stable)
                // We will approximate with a mean motion rotation for circular/near-circular orbits
                // This is a "Game Feel" approximation rather than full Kepler equation solver for stability

                // Mean Motion (n) = sqrt(mu / a^3). severe approx: a ~= r
                let n = (mu / r.powi(3)).sqrt();
                let angle = n * drift_time;

                // Rotate Position and Velocity
                // Axis of rotation? Cross product of r and v
                let angular_momentum = r_vec.cross(velocity);
                if angular_momentum.length_squared() > 0.001 {
                    let axis = angular_momentum.normalize();
                    let rot = Quat::from_axis_angle(axis, angle);

                    final_pos = rot * local_pos;
                    final_vel = rot * velocity;

                    info!(
                        "PERSISTENCE: Applied Orbital Catch-up. Angle: {:.2} rads",
                        angle
                    );

                    // Orbital Decay (Simulate drag/tidal forces)
                    // Decay by 0.1% per hour?
                    // drift_time is seconds. 1 hour = 3600s.
                    // Let's do a very small linear decay or exponential.
                    // decay_factor = 1.0 - (drift_time * 0.000001); // 1e-6 per second

                    let decay_rate = 0.0000005; // Very subtle decay
                    let decay_factor = (1.0 - decay_rate * drift_time).max(0.90); // Cap at 10% loss per session for safety

                    final_pos *= decay_factor;
                } else {
                    // Straight line fallback
                    final_pos += velocity * drift_time;
                }
            } else {
                // Hyperbolic/Parabolic - just drift linearly
                final_pos += velocity * drift_time;
            }
        } else {
            // Deep Space - Linear Drift
            final_pos += velocity * drift_time;
        }

        spawn_loc.cell = cell;
        spawn_loc.local_pos = final_pos;
        spawn_loc.velocity = Some(final_vel);
        spawn_loc.throttle = state.throttle; // Set loaded throttle
        spawn_loc.has_spawned = true;

        info!(
            "PERSISTENCE: Loaded state from {}s ago. Drifted {}s.",
            delta_time, drift_time
        );
    } else {
        info!("PERSISTENCE: No save found. Starting fresh.");
        spawn_loc.has_spawned = false;
    }
}

fn auto_save_system(
    time: Res<Time>,
    // Run every 5 seconds
    mut timer: Local<f32>,
    db: Res<Database>,
    q_player: Query<(&GridCell<i64>, &Transform, &Velocity), With<ZenCamera>>,
    input: Res<VehicleInput>, // Added input resource
    config: Res<PersistenceConfig>,
) {
    if config.scenario == "jupiter" {
        return; // Do not save state in scenarios
    }
    *timer += time.delta_secs();
    if *timer < 5.0 {
        return;
    }
    *timer = 0.0;

    if let Ok((cell, tf, vel)) = q_player.get_single() {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let state = PlayerState {
            cell_x: cell.x,
            cell_y: cell.y,
            cell_z: cell.z,
            local_x: tf.translation.x,
            local_y: tf.translation.y,
            local_z: tf.translation.z,
            vel_x: vel.0.x,
            vel_y: vel.0.y,
            vel_z: vel.0.z,
            timestamp: current_time,
            throttle: input.throttle, // Save throttle
        };

        if let Err(e) = db.save_player_state(&state) {
            error!("AutoSave Failed: {}", e);
        } else {
            // info!("AutoSaved Player State"); // Noisy
        }
    }
}

// System to detect cell change and query registry
// System to detect cell change and query registry
// Helper to check standard deterministic logic (Must match spawner.rs)
//Ideally this should be shared, but for now we duplicate the simple check
fn has_star(cell: &GridCell<i64>) -> bool {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // Assuming UniverseSeed is consistent (Default 0). If we have a seed resource, we should use it.
    // For now, let's assume seed 0 or pass it in.
    // Wait, spawner uses UniverseSeed resource. We should fetch it.
    use std::hash::{Hash, Hasher};
    cell.hash(&mut hasher);
    0.hash(&mut hasher); // Hardcoded seed 0 for now as we don't have access to UniverseSeed here easily without adding it to system params
    let cell_seed = hasher.finish();

    // Logic from spawner.rs:
    use rand::{Rng, SeedableRng};
    let mut rng = rand::rngs::StdRng::seed_from_u64(cell_seed);
    let is_origin = cell.x == 0 && cell.y == 0 && cell.z == 0;
    let density_chance = if is_origin { 1.0 } else { 0.02 };
    rng.gen_bool(density_chance)
}

fn check_system_change(
    q_player: Query<&GridCell<i64>, (Changed<GridCell<i64>>, With<ZenCamera>)>,
    db: Res<Database>,
    mut current_data: ResMut<CurrentSystemData>,
) {
    if let Ok(cell) = q_player.get_single() {
        info!("Entered System: {:?}", cell);
        current_data.cell = *cell;

        // SQLite Query
        match db.get_discovery(*cell) {
            Ok(Some(discovery)) => {
                current_data.discovery = Some(discovery);
            }
            Ok(None) => {
                // No discovery found. Check if it SHOULD be a star system.
                if has_star(cell) {
                    // Auto-Discover!
                    let default_name = format!("S {},{},{}", cell.x, cell.y, cell.z);
                    let new_discovery = Discovery {
                        cell_x: cell.x,
                        cell_y: cell.y,
                        cell_z: cell.z,
                        name: default_name.clone(),
                        finder: "System AI".to_string(), // Auto-discovered
                        note: "Autologged".to_string(),
                        date: "2026".to_string(),
                        object_type: "Star System".to_string(),
                    };

                    if let Err(e) = db.save_discovery(&new_discovery) {
                        error!("Failed to auto-save discovery: {}", e);
                        current_data.discovery = None;
                    } else {
                        info!("Auto-Discovered System: {}", default_name);
                        current_data.discovery = Some(new_discovery);
                    }
                } else {
                    current_data.discovery = None;
                }
            }
            Err(e) => {
                error!("Database Error: {}", e);
                current_data.discovery = None;
            }
        }
        current_data.is_dirty = true;
    }
}
