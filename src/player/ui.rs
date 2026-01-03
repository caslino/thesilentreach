use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::player::input::VehicleInput;
use crate::player::camera::Velocity;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {

        app.add_systems(Startup, setup_hud)
           .add_systems(Update, (update_hud, system_saved_toast_system));
    }
}

use crate::player::prediction::AntiCollisionState;
use crate::player::navigation::NavigationClues;
use crate::universe::SystemSavedEvent; // Import Event

#[derive(Component)]
struct ThrottleText;

#[derive(Component)]
struct SpeedText;

#[derive(Component)]
struct AntiCollisionText;

#[derive(Component)]
struct NavText;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct SavedToastText {
    timer: Timer,
}

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
        
        // Anti-Collision Warning
        parent.spawn((
            Text::new("ANTI-COLLISION ACTIVATED"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.0, 0.0)),
            AntiCollisionText,
            Visibility::Hidden,
        ));
        
        // Navigation Display
        parent.spawn((
            Text::new("SYSTEM BOOT...\nCalculating Position..."),
            TextFont {
                font_size: 16.0,
                // font: monopsaced if possible, check assets?
                ..default()
            },
            TextColor(Color::srgb(0.5, 1.0, 0.5)), // Retro Green
            NavText,
        ));
    });

    // FPS Counter (Top Right)
    commands.spawn((
        Text::new("FPS: 0"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)), // Bright Green
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        },
        FpsText,
    ));

    
    // Saved Toast (Hidden by default)
    commands.spawn((
        Text::new("SYSTEM DATA SAVED"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::srgb(0.2, 1.0, 0.2)), // Bright Green
        Node {
             position_type: PositionType::Absolute,
             top: Val::Percent(15.0),
             left: Val::Percent(50.0), // Centered horizontally
             margin: UiRect::left(Val::Px(-100.0)), // Offset by half width approx
             ..default()
        },
        Visibility::Hidden,
        SavedToastText { timer: Timer::from_seconds(3.0, TimerMode::Once) },
    ));
}

fn update_hud(
    input: Res<VehicleInput>,
    q_velocity: Query<&Velocity>,
    acs: Res<AntiCollisionState>,
    nav_clues: Res<NavigationClues>,
    mut q_throttle: Query<&mut Text, (With<ThrottleText>, Without<SpeedText>, Without<AntiCollisionText>, Without<NavText>, Without<FpsText>)>,
    mut q_speed: Query<&mut Text, (With<SpeedText>, Without<ThrottleText>, Without<AntiCollisionText>, Without<NavText>, Without<FpsText>)>,
    mut q_warning: Query<&mut Visibility, With<AntiCollisionText>>,
    mut q_nav: Query<(&mut Text, &mut Visibility), (With<NavText>, Without<FpsText>, Without<AntiCollisionText>)>, // Added Visibility + Without
    mut q_fps: Query<&mut Text, With<FpsText>>,
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>, // For Toggle
    mut nav_visible: Local<bool>, // Local State
) {
    // ... Existing Throttle/Speed/ACS updates ...
    if let Ok(mut text) = q_throttle.get_single_mut() {
        text.0 = format!("Throttle: {:.0}%", input.throttle * 100.0);
    }

    if let Ok(velocity) = q_velocity.get_single() {
        if let Ok(mut text) = q_speed.get_single_mut() {
            let speed = velocity.0.length();
            text.0 = format!("Speed: {:.0} m/s", speed);
        }
    }
    
    if let Ok(mut vis) = q_warning.get_single_mut() {
        if acs.is_active {
            *vis = Visibility::Visible;
        } else {
            *vis = Visibility::Hidden;
        }
    }
    
    // Toggle Nav
    if keys.just_pressed(KeyCode::KeyH) {
        *nav_visible = !*nav_visible;
    }

    // Update Navigation UI
    if let Ok((mut text, mut vis)) = q_nav.get_single_mut() {
        if *nav_visible {
            *vis = Visibility::Visible;
            let age = time.elapsed_secs();
            if age < 5.0 {
                 text.0 = format!("CRYO-STASIS DISENGAGED.\nSYSTEM SCANNING... {:.0}%", age * 20.0);
            } else {
                 let vec = nav_clues.vector_to_origin;
                 let heading_str = format!("Heading to Core: [{:.2}, {:.2}, {:.2}]", vec.x, vec.y, vec.z);
                 
                 let mut signal_str = String::from("\nPulsar Signals:\n");
                 for (i, (strength, _freq)) in nav_clues.pulsar_signals.iter().enumerate() {
                     let bars = (*strength * 10.0) as usize;
                     let bar_vis: String = std::iter::repeat('|').take(bars).collect();
                     signal_str.push_str(&format!("P{}: [{:<10}] {:.3}\n", i+1, bar_vis, strength));
                 }
                 
                 text.0 = format!("{}\n{}", heading_str, signal_str);
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }

    // Update FPS
    if let Ok(mut text) = q_fps.get_single_mut() {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.0 = format!("FPS: {:.0}", value);
            }
        }
    }
}

// --- DISCOVERY UI REMOVED ---
// use crate::persistence::{CurrentSystemData, Database};

// --- TOAST SYSTEM ---
fn system_saved_toast_system(
    mut events: EventReader<SystemSavedEvent>,
    mut q_toast: Query<(&mut Visibility, &mut SavedToastText, &mut Text)>,
    time: Res<Time>,
) {
    if let Ok((mut vis, mut toast, mut text)) = q_toast.get_single_mut() {
        // Trigger
        for ev in events.read() {
            *vis = Visibility::Visible;
            toast.timer.reset();
            text.0 = format!("SYSTEM DATA SAVED: {}", ev.name);
        }
        
        // Update Timer
        if *vis == Visibility::Visible {
            toast.timer.tick(time.delta());
            if toast.timer.finished() {
                *vis = Visibility::Hidden;
            }
        }
    }
}
