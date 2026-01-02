use bevy::prelude::*;
use crate::player::input::VehicleInput;
use crate::player::camera::Velocity;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
           .add_systems(Update, update_hud);
    }
}

#[derive(Component)]
struct ThrottleText;

#[derive(Component)]
struct SpeedText;

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    )).with_children(|parent| {
        // Throttle Indicator
        parent.spawn((
            Text::new("Throttle: 0%"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ThrottleText,
        ));

        // Speed Indicator
        parent.spawn((
            Text::new("Speed: 0 m/s"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            SpeedText,
        ));
    });
}

fn update_hud(
    input: Res<VehicleInput>,
    q_velocity: Query<&Velocity>,
    mut q_throttle: Query<&mut Text, (With<ThrottleText>, Without<SpeedText>)>,
    mut q_speed: Query<&mut Text, (With<SpeedText>, Without<ThrottleText>)>,
) {
    if let Ok(mut text) = q_throttle.get_single_mut() {
        text.0 = format!("Throttle: {:.0}%", input.throttle * 100.0);
    }

    if let Ok(velocity) = q_velocity.get_single() {
        if let Ok(mut text) = q_speed.get_single_mut() {
            let speed = velocity.0.length();
            text.0 = format!("Speed: {:.0} m/s", speed);
        }
    }
}
