use bevy::prelude::*;
use big_space::{FloatingOrigin, GridCell, ReferenceFrame};
use crate::player::input::VehicleInput;
use crate::universe::Mass;

const GRAVITY_CONSTANT: f32 = 50.0;
const THRUST_POWER: f32 = 2500.0;
const DAMPING_FACTOR: f32 = 0.99; // Less drag for deep space

pub struct ZenCameraPlugin;

impl Plugin for ZenCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
           .add_systems(Update, (ship_controls, apply_gravity, physics_step).chain());
    }
}

#[derive(Component, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct ZenCamera {
    pub max_speed: f32,
}

impl Default for ZenCamera {
    fn default() -> Self {
        Self {
            max_speed: 10000.0, // Higher max speed
        }
    }
}

fn setup_camera(mut commands: Commands, q_big_space: Query<Entity, With<ReferenceFrame<i64>>>) {
    let big_space_id = q_big_space.single();
    
    commands.entity(big_space_id).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 1000.0).looking_at(Vec3::ZERO, Vec3::Y),
            ZenCamera::default(),
            Velocity(Vec3::ZERO),
            GridCell::<i64>::default(),
            FloatingOrigin,
        ));
    });
}

fn ship_controls(
    mut query: Query<(&mut Transform, &mut Velocity, &mut ZenCamera)>,
    input: Res<VehicleInput>,
    time: Res<Time>,
) {
    let (mut transform, mut velocity, _ship) = query.single_mut();
    let dt = time.delta_secs();
    
    // 1. Steering (Pitch, Yaw, Roll)
    let pitch_speed = 1.0;
    let yaw_speed = 1.0;
    let roll_speed = 2.0;

    // Apply rotation relative to local axes
    let rotation_delta = Quat::from_euler(
        EulerRot::XYZ,
        input.pitch * pitch_speed * dt, // Pitch (Local X)
        input.yaw * yaw_speed * dt,     // Yaw (Local Y)
        input.roll * roll_speed * dt    // Roll (Local Z)
    );
    transform.rotation = transform.rotation * rotation_delta;
    transform.rotation = transform.rotation.normalize();

    // 2. Thrust (Throttle)
    if input.throttle > 0.0 {
        let forward = transform.forward();
        let thrust = forward * input.throttle * THRUST_POWER * dt;
        velocity.0 += thrust;
    }
}

fn apply_gravity(
    mut ship_query: Query<(&GridCell<i64>, &Transform, &mut Velocity), With<ZenCamera>>,
    mass_query: Query<(&GridCell<i64>, &Transform, &Mass)>,
    time: Res<Time>,
) {
    let Ok((ship_cell, ship_pos, mut ship_vel)) = ship_query.get_single_mut() else { return; };
    let dt = time.delta_secs();

    for (body_cell, body_pos, mass) in mass_query.iter() {
        // Calculate offset manually assuming 1M grid size
        let cell_diff = *body_cell - *ship_cell;
        let large_diff = Vec3::new(
             cell_diff.x as f32 * 1_000_000.0, 
             cell_diff.y as f32 * 1_000_000.0,
             cell_diff.z as f32 * 1_000_000.0,
        );
        
        let relative_pos = body_pos.translation - ship_pos.translation + large_diff;
        
        let distance_sq = relative_pos.length_squared();
        if distance_sq < 100.0 { continue; } // Avoid singularities

        let distance = distance_sq.sqrt();
        let direction = relative_pos / distance;

        let force_mag = GRAVITY_CONSTANT * mass.0 / distance_sq;
        let acceleration = direction * force_mag;

        ship_vel.0 += acceleration * dt;
    }
}

fn physics_step(
    mut query: Query<(&mut Transform, &mut Velocity)>,
    time: Res<Time>,
) {
    let (mut transform, mut velocity) = query.single_mut();
    let dt = time.delta_secs();

    // Damping / Drag
    velocity.0 *= DAMPING_FACTOR;

    // Apply Velocity
    transform.translation += velocity.0 * dt;
}
