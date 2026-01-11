use crate::player::input::VehicleInput;
use crate::player::prediction::AntiCollisionState;
use crate::universe::physics::{GRAVITY_CONSTANT, GRID_SIZE, REPULSION_STRENGTH};
use crate::universe::{Mass, Radius};
use bevy::prelude::*;
use big_space::{FloatingOrigin, GridCell, ReferenceFrame};

const SHIP_MASS: f32 = 1000.0; // kg
const THRUST_FORCE: f32 = 200_000.0; // (Increased from 100k)
const _DRAG_COEFFICIENT: f32 = 2.0; // "Space Friction" / Inertial Dampeners
const TURN_POWER: f32 = 0.8; // Lower torque for heavier start
const ROTATIONAL_DRAG: f32 = 0.5; // Lower drag to preserve momentum

pub struct ZenCameraPlugin;

impl Plugin for ZenCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_hz(60.0)) // Target 60Hz Physics
            .add_systems(Startup, setup_camera)
            .add_systems(
                FixedUpdate,
                (ship_controls, apply_gravity, physics_step).chain(),
            );
    }
}

#[derive(Component, Default)]
pub struct AngularVelocity(pub Vec3); // Local angular velocity (Pitch, Yaw, Roll)

#[derive(Component, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component)]
pub struct ZenCamera {
    pub max_speed: f32,
}

impl Default for ZenCamera {
    fn default() -> Self {
        Self {
            max_speed: 200000.0,
        }
    }
}

use crate::persistence::SpawnLocation;
use rand::Rng;

#[derive(Component)]
pub struct HeadCamera;

fn setup_camera(
    mut commands: Commands,
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    mut spawn_loc: ResMut<SpawnLocation>,
    mut vehicle_input: ResMut<VehicleInput>,
) {
    let big_space_id = q_big_space.single();

    // Generate Random Spawn if not set
    if !spawn_loc.has_spawned {
        let mut rng = rand::thread_rng();
        let range = 20_000..40_000;

        let x = if rng.gen_bool(0.5) {
            rng.gen_range(range.clone())
        } else {
            -rng.gen_range(range.clone())
        };
        let y = if rng.gen_bool(0.5) {
            rng.gen_range(range.clone())
        } else {
            -rng.gen_range(range.clone())
        };
        let z = if rng.gen_bool(0.5) {
            rng.gen_range(range.clone())
        } else {
            -rng.gen_range(range.clone())
        };

        spawn_loc.cell = GridCell::new(x, y, z);
        spawn_loc.local_pos = Vec3::new(0.0, 0.0, 2500.0);
        spawn_loc.has_spawned = true;

        info!(
            "CRYO-AWAKENING: Spawned at {:?} (Safe Distance)",
            spawn_loc.cell
        );
    } else {
        // Restore Throttle if loaded from Save
        vehicle_input.throttle = spawn_loc.throttle;
        info!(
            "CRYO-AWAKENING: Restored Throttle: {:.0}%",
            vehicle_input.throttle * 100.0
        );
    }

    commands.entity(big_space_id).with_children(|parent| {
        // --- 1. SHIP ROOT (Movement/Collision) ---
        parent
            .spawn((
                Transform::from_translation(spawn_loc.local_pos).looking_at(Vec3::ZERO, Vec3::Y),
                GlobalTransform::default(),
                ZenCamera::default(),
                Velocity(spawn_loc.velocity.unwrap_or(Vec3::ZERO)),
                AngularVelocity(Vec3::ZERO), // Added Angular Physics
                Mass(SHIP_MASS),
                spawn_loc.cell,
                FloatingOrigin,
                Visibility::default(),
            ))
            .with_children(|ship| {
                // ... Head Camera ...
                ship.spawn((
                    Camera3d::default(),
                    HeadCamera,
                    bevy::core_pipeline::bloom::Bloom::NATURAL, // Enable Bloom (Disabled for FPS)
                    bevy::core_pipeline::tonemapping::Tonemapping::TonyMcMapface, // Better exposure
                    Projection::from(PerspectiveProjection {
                        far: 10_000_000.0,
                        ..default()
                    }),
                    Transform::default(),
                ));
            });
    });
}

fn ship_controls(
    mut q_ship: Query<(
        &mut Transform,
        &mut Velocity,
        &mut AngularVelocity,
        &mut ZenCamera,
        &Children,
    )>,
    mut q_head: Query<(&mut Transform, &HeadCamera), Without<ZenCamera>>,
    input: Res<VehicleInput>,
    acs: Res<AntiCollisionState>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut ship_transform, mut ship_velocity, mut ship_ang_vel, _ship, children)) =
        q_ship.get_single_mut()
    else {
        return;
    };
    let dt = time.delta_secs();

    // Find Head Camera Child Entity
    let mut head_entity = None;
    for child in children.iter() {
        if q_head.get(*child).is_ok() {
            head_entity = Some(*child);
            break;
        }
    }

    // --- CONTROL LOGIC ---
    let is_look_mode = keys.pressed(KeyCode::KeyP);

    if acs.is_active {
        // ... (ACS Logic - simplified override for now)
        // ACS needs to manipulate rotation directly or via torque?
        // For safety, ACS overrides physics for instant correction typically,
        // but we can make it torque based later. keeping direct slerp for safety.
        if acs.avoidance_vector != Vec3::ZERO {
            let target_rot = ship_transform
                .looking_at(ship_transform.translation + acs.avoidance_vector, Vec3::Y)
                .rotation;
            ship_transform.rotation = ship_transform.rotation.slerp(target_rot, dt * 2.0);
            ship_ang_vel.0 = Vec3::ZERO; // cancel spin
        }
        let forward = ship_transform.forward();
        let thrust = forward * 1.0 * (THRUST_FORCE / SHIP_MASS) * dt;
        ship_velocity.0 += thrust;
    } else {
        // --- 1. ROTATION (Torque Based) ---
        if is_look_mode {
            // ROTATE HEAD (Independent)
            if let Some(entity) = head_entity {
                if let Ok((mut head, _)) = q_head.get_mut(entity) {
                    let rotation_delta = Quat::from_euler(
                        EulerRot::XYZ,
                        input.pitch * 2.0 * dt,
                        input.yaw * 2.0 * dt,
                        input.roll * 3.0 * dt,
                    );
                    head.rotation = head.rotation * rotation_delta;
                    head.rotation = head.rotation.normalize();
                }
            }
        } else {
            // ROTATE SHIP (Physics)
            // Input adds Angular Acceleration (Torque/Inertia)
            // PITCH (X), YAW (Y), ROLL (Z)

            let steer_torque = Vec3::new(input.pitch, input.yaw, input.roll) * TURN_POWER;

            // F = ma -> a = F/m
            // Here TURN_POWER is effectively Peak Torque / Moment of Inertia
            ship_ang_vel.0 += steer_torque * dt;

            // Damping (Physics Step will also handle, but we can do input shaping here if needed)

            // Re-center Head
            if let Some(entity) = head_entity {
                if let Ok((mut head, _)) = q_head.get_mut(entity) {
                    head.rotation = head.rotation.slerp(Quat::IDENTITY, dt * 5.0);
                }
            }
        }

        // --- 2. THRUST (F = ma -> a = F/m) ---
        // --- 2. THRUST (F = ma -> a = F/m) ---
        if input.throttle.abs() > 0.0 {
            let forward = ship_transform.forward();

            // Reverse Thrusters are weaker (1/3 power)
            let mut current_thrust_power = if input.throttle > 0.0 {
                THRUST_FORCE
            } else {
                THRUST_FORCE / 3.0
            };

            if input.warp_mode {
                current_thrust_power *= 10.0;
            }

            let accel = (input.throttle * current_thrust_power) / SHIP_MASS;
            ship_velocity.0 += forward * accel * dt;
        }
    }
}

fn apply_gravity(
    mut ship_query: Query<(&GridCell<i64>, &Transform, &mut Velocity), With<ZenCamera>>,
    mass_query: Query<(&GridCell<i64>, &Transform, &Mass, &Radius)>,
    time: Res<Time>,
) {
    let Ok((ship_cell, ship_pos, mut ship_vel)) = ship_query.get_single_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // Optimization: Max gravity range squared (50,000 units)
    // Gravity at 50k is neglible.
    let max_range = 50_000.0;
    let max_range_sq = max_range * max_range;

    // Find nearest 3 bodies (Single Pass, No Allocation)
    // We store (dist_sq, vec, mass, radius)
    let mut nearest = [(f32::MAX, Vec3::ZERO, 0.0, 0.0); 3]; // Fixed size array

    for (body_cell, body_pos, mass, radius) in mass_query.iter() {
        // Coarse Grid Check: If cell distance is too large, skip expensive float Math
        if (body_cell.x - ship_cell.x).abs() > 1
            || (body_cell.y - ship_cell.y).abs() > 1
            || (body_cell.z - ship_cell.z).abs() > 1
        {
            continue;
        }

        let cell_diff = *body_cell - *ship_cell;
        let large_diff = Vec3::new(
            cell_diff.x as f32 * GRID_SIZE,
            cell_diff.y as f32 * GRID_SIZE,
            cell_diff.z as f32 * GRID_SIZE,
        );

        let relative_pos = body_pos.translation - ship_pos.translation + large_diff;
        let distance_sq = relative_pos.length_squared();

        if distance_sq > 0.1 && distance_sq < max_range_sq {
            // Insertion Sort into fixed array
            let entry = (distance_sq, relative_pos, mass.0, radius.0);
            if entry.0 < nearest[0].0 {
                nearest[2] = nearest[1];
                nearest[1] = nearest[0];
                nearest[0] = entry;
            } else if entry.0 < nearest[1].0 {
                nearest[2] = nearest[1];
                nearest[1] = entry;
            } else if entry.0 < nearest[2].0 {
                nearest[2] = entry;
            }
        }
    }

    // Apply forces
    for (dist_sq, vec, mass, radius) in nearest.iter() {
        if *dist_sq == f32::MAX {
            continue;
        }

        let distance = dist_sq.sqrt();
        let direction = *vec / distance;

        // 1. Gravity (Pull)
        let gravity_force = GRAVITY_CONSTANT * mass / dist_sq;
        ship_vel.0 += direction * gravity_force * dt;

        // 2. Surface Repulsion (Push)
        let safe_radius = radius * 1.5;
        if distance < safe_radius {
            let overlap = safe_radius - distance;
            let repulsion = -direction * overlap * REPULSION_STRENGTH * dt;
            ship_vel.0 += repulsion;
        }
    }
}

fn physics_step(
    mut query: Query<(&mut Transform, &mut Velocity, &mut AngularVelocity, &Mass)>,
    time: Res<Time>,
) {
    let (mut transform, mut velocity, mut ang_vel, _mass) = query.single_mut();
    let dt = time.delta_secs();

    // --- Linear Physics ---
    // Mass-based Drag similar to air resistance / inertial dampers
    // F_drag = -param * v
    // We tune param to be "Game Feel Good"
    let linear_drag = 1.2;
    let damping = (-linear_drag * dt).exp();
    velocity.0 *= damping;
    transform.translation += velocity.0 * dt;

    // --- Angular Physics ---
    // Apply Rotation
    // Angular Velocity is in LOCAL SPACE (Pitch, Yaw, Roll)
    // So we multiply rotation by the delta quaternion constructed from local axis

    if ang_vel.0.length_squared() > 0.000001 {
        let delta_rot = Quat::from_scaled_axis(ang_vel.0 * dt);
        transform.rotation = transform.rotation * delta_rot; // Right-multiply for local rotation
        transform.rotation = transform.rotation.normalize();

        // Angular Damping
        // Torque needs to fight this.
        let ang_damping = (-ROTATIONAL_DRAG * dt).exp();
        ang_vel.0 *= ang_damping;
    }
}

// Tethering system removed per user request
