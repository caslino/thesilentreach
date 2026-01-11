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
    pub warp_mode: bool,
}

fn update_vehicle_input(
    mut input: ResMut<VehicleInput>,
    keys: Res<ButtonInput<KeyCode>>,
    touches: Res<Touches>,
    windows: Query<&Window>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // --- 0. TOUCH CONTROLS (iOS) ---
    // If any touch is active, we use touch logic.
    // Left Half: Throttle (Y) + Roll (X) [Sticky]
    // Right Half: Pitch (Y) + Yaw (X) [Steering]

    // We only process touches if they exist, otherwise fallback to Keyboard
    let mut touch_active = false;

    // Safely get window for touch calculation
    if let Ok(window) = windows.get_single() {
        let width = window.width();
        let center_x = width / 2.0;

        for touch in touches.iter() {
            touch_active = true;
            let pos = touch.position();

            // SENSITIVITY
            let touch_sensitivity = 0.005;

            if pos.x < center_x {
                // LEFT ZONE: Throttle & Roll
                if let Some(delta) = touches.get_pressed(touch.id()).map(|t| t.delta()) {
                    input.throttle =
                        (input.throttle - delta.y * touch_sensitivity).clamp(-1.0, 1.0);
                    input.roll = (input.roll + delta.x * touch_sensitivity).clamp(-1.0, 1.0);
                }
            } else {
                // RIGHT ZONE: Pitch & Yaw (Stick)
                if let Some(delta) = touches.get_pressed(touch.id()).map(|t| t.delta()) {
                    input.pitch =
                        (input.pitch + delta.y * touch_sensitivity * 10.0).clamp(-1.0, 1.0);
                    input.yaw = (input.yaw - delta.x * touch_sensitivity * 10.0).clamp(-1.0, 1.0);
                }
            }
        }

        // Spring Back (Touch)
        let mut left_active = false;
        let mut right_active = false;
        for touch in touches.iter() {
            if touch.position().x < center_x {
                left_active = true;
            } else {
                right_active = true;
            }
        }

        if touch_active {
            if !left_active {
                input.roll = lerp(input.roll, 0.0, dt * 5.0);
            }
            if !right_active {
                input.pitch = lerp(input.pitch, 0.0, dt * 5.0);
                input.yaw = lerp(input.yaw, 0.0, dt * 5.0);
            }
            return; // Skip Keyboard if using Touch (prevents conflict)
        }
    }

    // --- KEYBOARD CONTROLS (Fallback) ---
    // 1. Throttle (Incremental, Sticky)
    // Up Arrow: Increase
    // Down Arrow: Decrease
    // Space: Brake (Cut Throttle)
    let throttle_sensitivity = 0.5; // Takes 2 seconds to go 0->1

    if keys.just_pressed(KeyCode::Space) {
        input.throttle = 0.0;
    } else {
        if keys.pressed(KeyCode::ArrowUp) {
            input.throttle = (input.throttle + throttle_sensitivity * dt).min(1.0);
        }
        if keys.pressed(KeyCode::ArrowDown) {
            input.throttle = (input.throttle - throttle_sensitivity * dt).max(-1.0);
        }
    }

    // Toggle Warp Mode (0 Key)
    if keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::KeyO) {
        // Wait, user said "0". KeyO is for Origin (Teleport).
        // Let's stick to Digit0 as requested.
        if keys.just_pressed(KeyCode::Digit0) {
            input.warp_mode = !input.warp_mode;
        }
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

    // 3. Yaw (A/D OR Left/Right Arrows) - Spring Back
    let target_yaw = if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        1.0 // Turn Left
    } else if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
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
