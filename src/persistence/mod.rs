use bevy::prelude::*;
use big_space::GridCell;
use std::collections::HashMap;

// --- Data Models ---
#[derive(Debug, Clone)]
pub struct Discovery {
    pub name: String,
    pub finder: String,
    pub note: String, // Zen Note (max 140 chars)
    pub date: String,
}

#[derive(Resource, Default)]
pub struct CurrentSystemData {
    pub cell: GridCell<i64>,
    pub discovery: Option<Discovery>,
    pub is_dirty: bool, // Trigger UI update
}

#[derive(Resource, Default)]
pub struct DiscoveryRepository {
    // Key: (x, y, z) of GridCell
    store: HashMap<(i64, i64, i64), Discovery>,
}

impl DiscoveryRepository {
    pub fn save(&mut self, cell: GridCell<i64>, name: String, finder: String, note: String) {
        let key = (cell.x, cell.y, cell.z);
        let discovery = Discovery {
            name,
            finder,
            note,
            date: "2026-01-02".to_string(), // In real app, use chrono
        };
        self.store.insert(key, discovery);
        info!("Saved Discovery: {:?}", self.store.get(&key));
    }

    pub fn get(&self, cell: GridCell<i64>) -> Option<&Discovery> {
        let key = (cell.x, cell.y, cell.z);
        self.store.get(&key)
    }
}

#[derive(Resource, Default)]
pub struct SpawnLocation {
    pub cell: GridCell<i64>,
    pub local_pos: Vec3,
    pub has_spawned: bool,
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnLocation>()
           .init_resource::<DiscoveryRepository>()
           .init_resource::<CurrentSystemData>()
           .add_systems(Update, check_system_change);
    }
}

use crate::player::camera::ZenCamera;

// System to detect cell change and query registry
fn check_system_change(
    q_player: Query<&GridCell<i64>, (Changed<GridCell<i64>>, With<ZenCamera>)>,
    repo: Res<DiscoveryRepository>,
    mut current_data: ResMut<CurrentSystemData>,
) {
    if let Ok(cell) = q_player.get_single() {
        info!("Entered System: {:?}", cell);
        current_data.cell = *cell;
        
        if let Some(discovery) = repo.get(*cell) {
            current_data.discovery = Some(discovery.clone());
        } else {
            current_data.discovery = None;
        }
        current_data.is_dirty = true;
    }
}
