use bevy::prelude::*;
use big_space::GridCell;
use crate::player::camera::{Velocity, ZenCamera};
use crate::universe::{Mass, Radius};
use crate::universe::physics::{GRAVITY_CONSTANT, GRID_SIZE};

pub struct TrajectoryPlugin;

impl Plugin for TrajectoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PredictionTimer>()
           .init_resource::<AntiCollisionState>()
           .add_systems(Update, update_trajectory_prediction);
    }
}


#[derive(Resource, Default, Debug)]
pub struct AntiCollisionState {
    pub is_active: bool,
    pub avoidance_vector: Vec3,
}

#[derive(Resource)]
struct PredictionTimer(Timer);

impl Default for PredictionTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.1, TimerMode::Repeating))
    }
}

fn update_trajectory_prediction(
    mut timer: ResMut<PredictionTimer>,
    time: Res<Time>,
    q_ship: Query<(&GridCell<i64>, &Transform, &Velocity), With<ZenCamera>>,
    q_mass: Query<(&GridCell<i64>, &Transform, &Mass, &Radius)>,
    mut acs_state: ResMut<AntiCollisionState>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let Ok((start_cell, start_transform, start_vel)) = q_ship.get_single() else { return; };

    let mut current_pos = start_transform.translation;
    let mut current_vel = start_vel.0;
    
    let sim_dt = 0.05;
    let steps = 100;

    acs_state.is_active = false;
    acs_state.avoidance_vector = Vec3::ZERO;

    for _ in 0..steps {
        let mut total_acc = Vec3::ZERO;
        
        for (body_cell, body_tf, mass, radius) in q_mass.iter() {
            let cell_diff = *body_cell - *start_cell;
            let large_diff = Vec3::new(
                cell_diff.x as f32 * GRID_SIZE, 
                cell_diff.y as f32 * GRID_SIZE,
                cell_diff.z as f32 * GRID_SIZE,
            );
            
            let body_rel_pos = body_tf.translation + large_diff;
            let relative_pos = body_rel_pos - current_pos;
            let distance_sq = relative_pos.length_squared();
            
            if distance_sq > 0.1 {
                let distance = distance_sq.sqrt();
                let dir = relative_pos / distance;
                let force = GRAVITY_CONSTANT * mass.0 / distance_sq;
                total_acc += dir * force;
                
                // ACS Risk Detection: Dynamic Safety Margin (2.0x Radius)
                // This ensures we detect large stars from further away.
                let safety_margin = radius.0 * 3.0; 
                
                if distance < safety_margin {
                    acs_state.is_active = true;
                    
                    // Improved Evasion Logic
                    // If we are flying roughly towards it?
                    // Check dot product of velocity and direction to body.
                    let vel_dir = current_vel.normalize_or_zero();
                    let dot = vel_dir.dot(dir);
                    
                    if dot > 0.9 {
                        // Head-on collision!
                        // We need to sidestep. Use cross product to find a perpendicular vector.
                        // Cross with UP (Y). If we are aligned with Y, cross with RIGHT (X).
                        let mut side = vel_dir.cross(Vec3::Y);
                        if side.length_squared() < 0.01 {
                            side = vel_dir.cross(Vec3::X);
                        }
                        acs_state.avoidance_vector += side.normalize_or_zero() * 2.0; // Stronger sidestep
                    } else {
                        // Standard repulsion (push away from body)
                        acs_state.avoidance_vector -= dir;
                    }
                }
            }
        }

        current_vel += total_acc * sim_dt;
        current_pos += current_vel * sim_dt;

        current_vel += total_acc * sim_dt;
        current_pos += current_vel * sim_dt;
    }
    
    if acs_state.is_active {
        acs_state.avoidance_vector = acs_state.avoidance_vector.normalize_or_zero();
    }
}
