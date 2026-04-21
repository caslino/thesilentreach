use crate::player::camera::Velocity;
use crate::player::input::VehicleInput;
use crate::player::prediction::AntiCollisionState;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
            .add_systems(
                Update,
                (
                    update_hud.run_if(crate::player::interaction::console_is_inactive),
                    system_saved_toast_system,
                ),
            );
    }
}

use crate::player::navigation::NavigationClues;
use crate::universe::SystemSavedEvent;

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
struct WarpText;

#[derive(Component)]
struct SavedToastText {
    timer: Timer,
}

#[derive(Component)]
struct HudRoot;

fn setup_hud(mut commands: Commands) {
    // Main HUD Container (Bottom Left)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(20.0),
                left: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            HudRoot,
        ))
        .with_children(|parent| {
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

            // Warp Indicator
            parent.spawn((
                Text::new("WARP: OFF"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 1.0)), // Light Blue
                WarpText,
            ));

            // Anti-Collision Warning (Hidden by default via Display::None)
            parent.spawn((
                Text::new("ANTI-COLLISION ACTIVATED"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.0, 0.0)),
                AntiCollisionText,
                Node {
                    display: Display::None,
                    ..default()
                },
            ));

            // Navigation Display (Hidden by default via Display::None)
            parent.spawn((
                Text::new("SYSTEM BOOT...\nCalculating Position..."),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.5)), // Retro Green
                NavText,
                Node {
                    display: Display::None,
                    ..default()
                },
            ));
        });

    // FPS Counter (Bottom Right)
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
        HudRoot,
    ));

    // Saved Toast
    commands.spawn((
        Text::new("SYSTEM DATA SAVED"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.2, 1.0, 0.2)), // Bright Green
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(15.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-100.0)),
            ..default()
        },
        Visibility::Hidden,
        SavedToastText {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        },
        HudRoot,
    ));
}

fn update_hud(
    input: Res<VehicleInput>,
    q_velocity: Query<&Velocity>,
    acs: Res<AntiCollisionState>,
    nav_clues: Res<NavigationClues>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<ThrottleText>>,
        Query<&mut Text, With<SpeedText>>,
        Query<&mut Text, With<NavText>>,
        Query<&mut Text, With<FpsText>>,
        Query<&mut Text, With<WarpText>>,
    )>,
    mut q_warning_node: Query<&mut Node, (With<AntiCollisionText>, Without<NavText>)>,
    mut q_nav_node: Query<&mut Node, (With<NavText>, Without<AntiCollisionText>)>,
    mut q_hud_root: Query<
        &mut Visibility,
        (With<HudRoot>, Without<AntiCollisionText>, Without<NavText>),
    >,
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut nav_visible: Local<bool>,
) {
    // Cinematic Mode Toggle (L) - Uses Visibility to hide everything
    if keys.just_pressed(KeyCode::KeyL) {
        for mut vis in q_hud_root.iter_mut() {
            if *vis == Visibility::Hidden {
                *vis = Visibility::Visible;
            } else {
                *vis = Visibility::Hidden;
            }
        }
    }

    // Update Text Content
    if let Ok(mut text) = text_queries.p0().get_single_mut() {
        text.0 = format!("Throttle: {:.0}%", input.throttle * 100.0);
    }

    if let Ok(velocity) = q_velocity.get_single() {
        if let Ok(mut text) = text_queries.p1().get_single_mut() {
            let speed = velocity.0.length();
            text.0 = format!("Speed: {:.0} m/s", speed);
        }
    }

    if let Ok(mut text) = text_queries.p4().get_single_mut() {
        if input.warp_mode {
            text.0 = "WARP: ON".to_string();
        } else {
            text.0 = "WARP: OFF".to_string();
        }
    }

    // Toggle Warning Display
    if let Ok(mut node) = q_warning_node.get_single_mut() {
        if acs.is_active {
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }

    // Toggle Nav Display
    if keys.just_pressed(KeyCode::KeyH) {
        *nav_visible = !*nav_visible;
    }

    if let Ok(mut node) = q_nav_node.get_single_mut() {
        if *nav_visible {
            node.display = Display::Flex;
            if let Ok(mut text) = text_queries.p2().get_single_mut() {
                let age = time.elapsed_secs();
                if age < 5.0 {
                    text.0 = format!(
                        "CRYO-STASIS DISENGAGED.\nSYSTEM SCANNING... {:.0}%",
                        age * 20.0
                    );
                } else {
                    let vec = nav_clues.vector_to_origin;
                    let heading_str = format!(
                        "Heading to Core: [{:.2}, {:.2}, {:.2}]",
                        vec.x, vec.y, vec.z
                    );

                    let mut signal_str = String::from("\nPulsar Signals:\n");
                    for (i, (strength, _freq)) in nav_clues.pulsar_signals.iter().enumerate() {
                        let bars = (*strength * 10.0) as usize;
                        let bar_vis: String = std::iter::repeat('|').take(bars).collect();
                        signal_str.push_str(&format!(
                            "P{}: [{:<10}] {:.3}\n",
                            i + 1,
                            bar_vis,
                            strength
                        ));
                    }

                    text.0 = format!("{}\n{}", heading_str, signal_str);
                }
            }
        } else {
            node.display = Display::None;
        }
    }

    // Update FPS
    if let Ok(mut text) = text_queries.p3().get_single_mut() {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.0 = format!("FPS: {:.0}", value);
            }
        }
    }
}

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
