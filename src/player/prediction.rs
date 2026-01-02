use bevy::prelude::*;
use big_space::{GridCell, FloatingOrigin};
use crate::player::camera::{Velocity, ZenCamera};
use crate::universe::Mass;

pub struct TrajectoryPlugin;

impl Plugin for TrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectedPath>()
           .init_resource::<PredictionTimer>()
           .add_systems(Update, (update_trajectory_prediction, draw_trajectory));
    }
}

#[derive(Resource, Default)]
pub struct ProjectedPath(pub Vec<Vec3>);

#[derive(Resource)]
struct PredictionTimer(Timer);

impl Default for PredictionTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.1, TimerMode::Repeating)) // Run 10 times a second
    }
}

fn update_trajectory_prediction(
    mut timer: ResMut<PredictionTimer>,
    time: Res<Time>,
    q_ship: Query<(&GridCell<i64>, &Transform, &Velocity), With<ZenCamera>>,
    q_mass: Query<(&GridCell<i64>, &Transform, &Mass)>,
    mut path: ResMut<ProjectedPath>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((start_cell, start_transform, start_vel)) = q_ship.get_single() else { return; };

    // Ghost Simulation
    let mut current_pos = start_transform.translation;
    let mut current_vel = start_vel.0;
    
    // We simulate entirely in the specialized "Local Frame" of the ship's CURRENT GridCell.
    // This works fine as long as the simulation doesn't go extremely far (millions of units) in 2 seconds.
    // 100 steps * 0.05s = 5 seconds prediction.
    
    let sim_dt = 0.05;
    let steps = 100;
    let gravity_constant = 50.0; // Must match camera.rs

    path.0.clear();
    path.0.push(current_pos);

    for _ in 0..steps {
        // Calculate Gravity
        let mut total_acc = Vec3::ZERO;
        
        for (body_cell, body_tf, mass) in q_mass.iter() {
             // Calculate offset manually assuming 1M grid size
            let cell_diff = *body_cell - *start_cell;
            let large_diff = Vec3::new(
                cell_diff.x as f32 * 1_000_000.0, 
                cell_diff.y as f32 * 1_000_000.0,
                cell_diff.z as f32 * 1_000_000.0,
            );
            
            let body_rel_pos = body_tf.translation + large_diff;
            let relative_pos = body_rel_pos - current_pos;
            let distance_sq = relative_pos.length_squared();
            
            if distance_sq > 100.0 {
                let distance = distance_sq.sqrt();
                let dir = relative_pos / distance;
                let force = gravity_constant * mass.0 / distance_sq;
                total_acc += dir * force;
            }
        }

        // Integrate
        current_vel += total_acc * sim_dt;
        current_pos += current_vel * sim_dt;

        path.0.push(current_pos);
    }
}

fn draw_trajectory(
    path: Res<ProjectedPath>,
    mut gizmos: Gizmos,
) {
    if path.0.len() < 2 { return; }

    // gizmos.linestrip(path.0.clone(), Color::srgb(0.0, 1.0, 1.0)); // Cyan line (Disabled per user request)
}
