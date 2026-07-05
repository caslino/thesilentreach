use bevy::prelude::*;
use crate::persistence::{Database, PlayerName};
use bevy::input::keyboard::{Key, KeyboardInput};

pub struct PlayerSetupPlugin;

impl Plugin for PlayerSetupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SetupState>()
            .add_systems(Startup, setup_welcome_ui)
            .add_systems(Update, welcome_input_system);
    }
}

#[derive(Resource, Default)]
struct SetupState {
    pub input_name: String,
}

#[derive(Component)]
struct WelcomeOverlayRoot;

#[derive(Component)]
struct WelcomeInputText;

fn setup_welcome_ui(mut commands: Commands, _player_name: Res<PlayerName>, db: Res<Database>) {
    // Only show if player name is not set in DB
    let is_new_player = db.get_setting("player_name").unwrap_or(None).is_none();
    
    let visibility = if is_new_player {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.95)),
            visibility,
            WelcomeOverlayRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("WELCOME TO THE SILENT REACH"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("IDENTIFY YOURSELF, EXPLORER"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                WelcomeInputText,
                Node {
                    margin: UiRect::vertical(Val::Px(30.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("[ENTER] Confirm Identity"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
            ));
        });

    if is_new_player {
        info!("PLAYER SETUP: Awaiting player identification...");
    }
}

fn welcome_input_system(
    mut key_evr: EventReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut setup_state: ResMut<SetupState>,
    mut player_name: ResMut<PlayerName>,
    db: Res<Database>,
    mut q_overlay: Query<&mut Visibility, With<WelcomeOverlayRoot>>,
    mut q_text: Query<&mut Text, With<WelcomeInputText>>,
    mut time: ResMut<Time<Virtual>>,
    touches: Res<Touches>,
) {
    let Ok(mut overlay_vis) = q_overlay.single_mut() else { return; };
    if *overlay_vis == Visibility::Hidden {
        return;
    }

    // Pause physics while naming
    time.pause();

    // Trigger keyboard show on Android
    super::soft_keyboard::show_keyboard();

    // If tapped/touched, show keyboard again in case it got closed/dismissed by user
    if touches.any_just_pressed() {
        super::soft_keyboard::show_keyboard();
    }

    if keys.just_pressed(KeyCode::Enter) && !setup_state.input_name.trim().is_empty() {
        let final_name = setup_state.input_name.trim().to_string();
        
        // Save to DB
        if let Err(e) = db.save_setting("player_name", &final_name) {
            error!("Failed to save player name: {}", e);
        } else {
            player_name.0 = final_name;
            info!("PLAYER SETUP: Identity confirmed as {}.", player_name.0);
            *overlay_vis = Visibility::Hidden;
            time.unpause();
            super::soft_keyboard::hide_keyboard();
        }
    }

    if keys.just_pressed(KeyCode::Backspace) {
        setup_state.input_name.pop();
    }

    for ev in key_evr.read() {
        if ev.state.is_pressed() {
            if let Key::Character(ref s) = ev.logical_key {
                let c = s.chars().next().unwrap_or('\0');
                if !c.is_control() && setup_state.input_name.len() < 24 {
                    setup_state.input_name.push(c);
                }
            } else if let Key::Space = ev.logical_key {
                if setup_state.input_name.len() < 24 {
                    setup_state.input_name.push(' ');
                }
            }
        }
    }

    if let Ok(mut txt) = q_text.single_mut() {
        txt.0 = format!("{}_", setup_state.input_name);
    }
}
