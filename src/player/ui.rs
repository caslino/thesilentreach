use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::player::input::VehicleInput;
use crate::player::camera::Velocity;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hud)
           .add_systems(Update, update_hud);
    }
}

use crate::player::prediction::AntiCollisionState;
use crate::player::navigation::NavigationClues;

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
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        FpsText,
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
    mut q_nav: Query<&mut Text, (With<NavText>, Without<FpsText>)>,
    mut q_fps: Query<&mut Text, With<FpsText>>,
    diagnostics: Res<DiagnosticsStore>,
    time: Res<Time>,
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
    
    // Update Navigation UI
    if let Ok(mut text) = q_nav.get_single_mut() {
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

// --- DISCOVERY UI ---
use crate::persistence::{CurrentSystemData, DiscoveryRepository};

#[derive(Component)]
struct DiscoveryPanel;

#[derive(Component)]
struct SystemNameText;

#[derive(Component)]
struct ZenNoteText;

#[derive(Component)]
struct RegisterButton;

// Simple UI State for Input
// Note: handling text input in Bevy without egui is complex. 
// We will simulate it with a simple "Press Enter to Claim" for now,
// or use a very basic state machine.
#[derive(Resource, Default)]
struct DiscoveryInputState {
    step: DiscoveryStep, 
    temp_name: String,
    temp_note: String,
}

#[derive(Default, PartialEq, Eq)]
enum DiscoveryStep {
    #[default]
    Idle,
    Naming,
    Noting,
}

pub struct DiscoveryUiPlugin;
impl Plugin for DiscoveryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiscoveryInputState>()
           .add_systems(Startup, setup_discovery_ui)
           .add_systems(Update, (update_discovery_panel, handle_discovery_interactions));
    }
}

fn setup_discovery_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(50.0),
            left: Val::Percent(50.0), // Center
            // translate: Transform::from_translation(Vec3::new(-50.0, 0.0, 0.0)), // CSS translate not in Bevy UI Node yet? Use margins.
            margin: UiRect { left: Val::Auto, right: Val::Auto, ..default() },
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        DiscoveryPanel,
    )).with_children(|parent| {
        // System Name
        parent.spawn((
            Text::new("Unknown Sector"),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::srgb(0.9, 0.8, 0.2)), // Gold
            SystemNameText,
        ));
        
        // Zen Note / Status
        parent.spawn((
            Text::new("Scanning..."),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ZenNoteText,
        ));
        
        // Register Button (Visual only, interactions handled via Key for now)
        parent.spawn((
            Text::new("[ PRESS 'ENTER' TO REGISTER ]"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.0, 1.0, 1.0)), 
            RegisterButton,
            Visibility::Hidden,
        ));
    });
}

fn update_discovery_panel(
    mut data: ResMut<CurrentSystemData>,
    mut q_name: Query<&mut Text, (With<SystemNameText>, Without<ZenNoteText>, Without<RegisterButton>)>,
    mut q_note: Query<&mut Text, (With<ZenNoteText>, Without<SystemNameText>, Without<RegisterButton>)>,
    mut q_btn: Query<&mut Visibility, With<RegisterButton>>,
    mut input_state: ResMut<DiscoveryInputState>,
) {
    if !data.is_dirty { return; }
    
    // reset input state on system change
    *input_state = DiscoveryInputState::default();

    if let Ok(mut name_txt) = q_name.get_single_mut() {
        if let Ok(mut note_txt) = q_note.get_single_mut() {
            if let Ok(mut btn_vis) = q_btn.get_single_mut() {
                if let Some(d) = &data.discovery {
                    name_txt.0 = d.name.clone();
                    note_txt.0 = format!("\"{}\" - {}", d.note, d.finder);
                    *btn_vis = Visibility::Hidden;
                } else {
                    name_txt.0 = format!("Sector {:?} (Uncharted)", data.cell);
                    note_txt.0 = "No Data Available.".to_string();
                    *btn_vis = Visibility::Visible; // Available to claim
                }
            }
        }
    }
    
    data.is_dirty = false;
}

fn handle_discovery_interactions(
    keys: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<DiscoveryInputState>,
    mut data: ResMut<CurrentSystemData>,
    mut repo: ResMut<DiscoveryRepository>,
    mut q_name: Query<&mut Text, With<SystemNameText>>,
    mut q_note: Query<&mut Text, With<ZenNoteText>>,
) {
    // Very basic state machine for "typing"
    // In a real game we'd capture CharInput
    
    if let Ok(mut name_txt) = q_name.get_single_mut() {
        if let Ok(mut note_txt) = q_note.get_single_mut() {
            
            match input_state.step {
                DiscoveryStep::Idle => {
                    if keys.just_pressed(KeyCode::Enter) && data.discovery.is_none() {
                        input_state.step = DiscoveryStep::Naming;
                        name_txt.0 = "NAME: _".to_string();
                        note_txt.0 = "Type Name (Simulated: 'Zenith Prime') -> Press ENTER".to_string();
                    }
                },
                DiscoveryStep::Naming => {
                    if keys.just_pressed(KeyCode::Enter) {
                        input_state.temp_name = "Zenith Prime".to_string(); // Simulate typing
                        input_state.step = DiscoveryStep::Noting;
                        name_txt.0 = input_state.temp_name.clone();
                        note_txt.0 = "NOTE: _ (Simulated: 'Quiet voids sing.') -> Press ENTER".to_string();
                    }
                },
                DiscoveryStep::Noting => {
                    if keys.just_pressed(KeyCode::Enter) {
                        input_state.temp_note = "Quiet voids sing.".to_string();
                        
                        // Save!
                        repo.save(data.cell, input_state.temp_name.clone(), "Traveler".to_string(), input_state.temp_note.clone());
                        
                        // Update Data locally
                        data.discovery = Some(crate::persistence::Discovery {
                            name: input_state.temp_name.clone(),
                            finder: "Traveler".to_string(),
                            note: input_state.temp_note.clone(),
                            date: "Now".to_string()
                        });
                        data.is_dirty = true; // Trigger refresh
                        
                        input_state.step = DiscoveryStep::Idle;
                    }
                }
            }
        }
    }
}
