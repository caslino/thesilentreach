use bevy::prelude::*;

pub struct MobileInputPlugin;

impl Plugin for MobileInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleInput>()
           .add_systems(Update, update_vehicle_input);
    }
}

#[derive(Resource, Default, Debug)]
pub struct VehicleInput {
    pub throttle: f32, // -1.0 (Full Reverse) to 1.0 (Full Forward)
    pub pitch: f32,    // -1.0 to 1.0 (Down/Up)
    pub yaw: f32,      // -1.0 to 1.0 (Left/Right)
    pub roll: f32,     // -1.0 to 1.0 (Roll Left/Right)
}

fn update_vehicle_input(
    mut input: ResMut<VehicleInput>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // 1. Throttle (Incremental, Sticky)
    // Up Arrow: Increase
    // Down Arrow: Decrease
    let throttle_sensitivity = 0.5; // Takes 2 seconds to go 0->1
    if keys.pressed(KeyCode::ArrowUp) {
        input.throttle = (input.throttle + throttle_sensitivity * dt).min(1.0);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        input.throttle = (input.throttle - throttle_sensitivity * dt).max(-1.0);
    }

    // 2. Pitch (W/S) - Spring Back
    let target_pitch = if keys.pressed(KeyCode::KeyW) {
        1.0 
    } else if keys.pressed(KeyCode::KeyS) {
        -1.0
    } else {
        0.0
    };
    input.pitch = lerp(input.pitch, target_pitch, dt * 10.0); // Fast response

    // 3. Yaw (A/D) - Spring Back
    let target_yaw = if keys.pressed(KeyCode::KeyA) {
        1.0 // Turn Left
    } else if keys.pressed(KeyCode::KeyD) {
        -1.0 // Turn Right
    } else {
        0.0
    };
    input.yaw = lerp(input.yaw, target_yaw, dt * 10.0);

    // 4. Roll (Q/E) - Spring Back
    let target_roll = if keys.pressed(KeyCode::KeyQ) {
        1.0 // Roll Left
    } else if keys.pressed(KeyCode::KeyE) {
        -1.0 // Roll Right
    } else {
        0.0
    };
    input.roll = lerp(input.roll, target_roll, dt * 10.0);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
