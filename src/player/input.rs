use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct TiltInput {
    /// Range -1.0 to 1.0. Negative = Left turn (Roll left), Positive = Right turn.
    pub roll: f32,
    /// Range -1.0 to 1.0. Negative = Back tilt (Brake/Reverse), Positive = Forward tilt (Accelerate)
    pub pitch: f32,
}

pub struct MobileInputPlugin;

impl Plugin for MobileInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TiltInput>()
           .add_systems(Update, mock_tilt_input);
    }
}

// Simulates phone tilt using keyboard keys
fn mock_tilt_input(
    mut tilt: ResMut<TiltInput>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let response_speed = 3.0 * time.delta_secs();
    
    // Roll (Turning) - Left/Right
    let target_roll = if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        -1.0
    } else if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        1.0
    } else {
        0.0
    };

    // Pitch (Throttle) - Up/Down
    let target_pitch = if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        1.0
    } else if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        -1.0
    } else {
        0.0
    };

    // Smooth interpolation towards target (simulating physical hand movement delay)
    tilt.roll = tilt.roll.lerp(target_roll, response_speed);
    tilt.pitch = tilt.pitch.lerp(target_pitch, response_speed);
}
