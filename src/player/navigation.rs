use bevy::prelude::*;
use big_space::{GridCell, FloatingOrigin};

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavigationClues>()
           .add_systems(Update, update_navigation_clues);
    }
}

#[derive(Resource, Default)]
pub struct NavigationClues {
    pub vector_to_origin: Vec3,
    pub pulsar_signals: Vec<(f32, f32)>, // (Strength, Frequency)
}

fn update_navigation_clues(
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
    mut clues: ResMut<NavigationClues>,
    _time: Res<Time>,
) {
    let Ok(current_cell) = q_camera.get_single() else { return; };

    // 1. Core Heading (Vector to 0,0,0)
    // Since we are far away, the direction is roughly -current_cell
    // We treat the grid as the primary coordinate system for long-range navigation.
    let dir_to_origin = Vec3::new(
        -current_cell.x as f32,
        -current_cell.y as f32,
        -current_cell.z as f32,
    ).normalize_or_zero();
    
    clues.vector_to_origin = dir_to_origin;

    // 2. Pulsar Signals (Simulation)
    // Arbitrary fixed coordinates for pulsars (in Grid Cells)
    let pulsars = [
        (GridCell::new(100_000, 100_000, 100_000), 1.0), // Fast 
        (GridCell::new(-50_000, 20_000, 0), 0.5),        // Medium
        (GridCell::new(0, -80_000, 40_000), 0.2),        // Slow
    ];

    clues.pulsar_signals.clear();
    for (p_cell, freq) in pulsars.iter() {
        let dx = (current_cell.x - p_cell.x) as f32;
        let dy = (current_cell.y - p_cell.y) as f32;
        let dz = (current_cell.z - p_cell.z) as f32;
        let dist_sq = dx*dx + dy*dy + dz*dz;
        
        let strength = 1.0 / (1.0 + dist_sq.sqrt() * 0.0001); // Fake signal dropoff
        clues.pulsar_signals.push((strength, *freq));
    }
}
