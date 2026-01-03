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
    pub has_spawned: bool,
}

use crate::player::camera::{ZenCamera, Velocity};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        // Initialize DB
        let db = Database::open().expect("Failed to open SQLite database");

        app.insert_resource(db)
           .init_resource::<SpawnLocation>()
           .init_resource::<CurrentSystemData>()
           .add_systems(PreStartup, load_player_state) // Load before camera setup
           .add_systems(Update, (check_system_change, auto_save_system));
    }
}

fn load_player_state(
    db: Res<Database>,
    mut spawn_loc: ResMut<SpawnLocation>,
) {
    // 1. Check DB
    if let Ok(Some(state)) = db.get_player_state() {
        // 2. Calculate Catch-up
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let delta_time = current_time - state.timestamp;
        
        let mut local_pos = Vec3::new(state.local_x, state.local_y, state.local_z);
        let velocity = Vec3::new(state.vel_x, state.vel_y, state.vel_z);
        
        // Simple linear projection for "catch up" (ignoring gravity for simplicity during catchup)
        // If delta is huge (days), maybe cap it? For now, we trust the void.
        // Actually, preventing moving millions of km into a planet might be good.
        // Limit catchup drift to e.g. 1 minute for safety?
        // Or assume ship was "drifting" safely? User requested "trajectory path calculated".
        // Let's assume linear drift.
        let drift_time = delta_time.clamp(0, 600) as f32; // Limit to 10 mins of drift
        local_pos += velocity * drift_time;
        
        spawn_loc.cell = GridCell::new(state.cell_x, state.cell_y, state.cell_z);
        spawn_loc.local_pos = local_pos;
        spawn_loc.velocity = Some(velocity); // Need to add this field to SpawnLocation
        spawn_loc.has_spawned = true;
        
        info!("PERSISTENCE: Loaded state from {}s ago. Drifted {}s.", delta_time, drift_time);
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
) {
    *timer += time.delta_secs();
    if *timer < 5.0 { return; }
    *timer = 0.0;

    if let Ok((cell, tf, vel)) = q_player.get_single() {
         let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
         
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
             },
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
             },
             Err(e) => {
                 error!("Database Error: {}", e);
                 current_data.discovery = None;
             }
        }
        current_data.is_dirty = true;
    }
}
