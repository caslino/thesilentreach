use bevy::prelude::*;
use big_space::{FloatingOrigin, GridCell, ReferenceFrame};
use crate::player::input::TiltInput;

pub struct ZenCameraPlugin;

impl Plugin for ZenCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera) // Startup runs after PreStartup
           .add_systems(Update, ship_controls);
    }
}

#[derive(Component)]
pub struct ZenCamera {
    pub current_speed: f32,
    pub target_speed: f32,
    pub max_speed: f32,
    pub rotation_velocity: f32,
}

impl Default for ZenCamera {
    fn default() -> Self {
        Self {
            current_speed: 0.0,
            target_speed: 0.0,
            max_speed: 5000.0, // High speed for space travel
            rotation_velocity: 0.0,
        }
    }
}

fn setup_camera(mut commands: Commands, q_big_space: Query<Entity, With<ReferenceFrame<i64>>>) {
    let big_space_id = q_big_space.single();
    
    commands.entity(big_space_id).with_children(|parent| {
        parent.spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 0.0, 1000.0).looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            },
            ZenCamera::default(),
            GridCell::<i64>::default(),
            FloatingOrigin,
        ));
    });
}

fn ship_controls(
    mut query: Query<(&mut Transform, &mut ZenCamera)>,
    tilt: Res<TiltInput>,
    time: Res<Time>,
) {
    let (mut transform, mut ship) = query.single_mut();
    let dt = time.delta_secs();
    
    // 1. Handle Speed (Pitch determines target speed)
    // Pitch > 0 -> Accelerate towards max_speed
    // Pitch < 0 -> Decelerate / Reverse
    if tilt.pitch > 0.05 {
        ship.target_speed = tilt.pitch * ship.max_speed;
    } else if tilt.pitch < -0.05 {
        ship.target_speed = tilt.pitch * (ship.max_speed * 0.2); // Slower reverse
    } else {
        ship.target_speed = 0.0; // Decay to stop if no input
    }
    
    // Smooth speed transition (inertia)
    ship.current_speed = ship.current_speed.lerp(ship.target_speed, dt * 0.5);

    // 2. Handle Turning (Roll determines rotation speed)
    let turn_sharpness = 1.0;
    ship.rotation_velocity = -tilt.roll * turn_sharpness;
    
    // Apply Rotation
    transform.rotate_y(ship.rotation_velocity * dt);
    
    // Apply Banking (Visual effect only, rotate Z slightly based on turn)
    let current_bank = transform.rotation.to_euler(EulerRot::YXZ).2;
    let target_bank = tilt.roll * 0.3; // Max bank angle in radians
    let new_bank = current_bank.lerp(target_bank, dt * 2.0);
    
    // Reconstruct rotation with new Y (turn) and Z (bank), keeping X (pitch) stable for now
    let (y, x, _) = transform.rotation.to_euler(EulerRot::YXZ);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, y, x, new_bank);

    // 3. Move forward
    let forward = transform.forward();
    transform.translation += forward * ship.current_speed * dt;
}
