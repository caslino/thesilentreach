use bevy::prelude::*;
use bevy::picking::prelude::*;
use bevy::input::keyboard::{KeyboardInput, Key};
use crate::persistence::{Database, Discovery};
use crate::universe::StarClicked; // Direct import of Event

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        // StarClicked event is registered in UniversePlugin, but we consume it here
        app.init_resource::<RenameState>()
           .add_systems(Startup, setup_rename_ui)
           .add_systems(Update, (handle_star_clicked_event, rename_input_system));
    }
}

#[derive(Resource, Default)]
struct RenameState {
    active: bool,
    target_cell: Option<big_space::GridCell<i64>>,
    current_name: String,
}

#[derive(Component)]
struct RenameOverlayRoot;

#[derive(Component)]
struct RenameInputText;

fn setup_rename_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
        Visibility::Hidden,
        RenameOverlayRoot,
    )).with_children(|parent| {
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::WHITE),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        )).with_children(|panel| {
            panel.spawn((
                Text::new("RENAME SYSTEM"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
            ));
            
            panel.spawn((
                Node { height: Val::Px(20.0), ..default() }, 
            ));

            panel.spawn((
                Text::new(""),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
                RenameInputText,
            ));
            
             panel.spawn((
                Node { height: Val::Px(20.0), ..default() }, 
            ));
            
             panel.spawn((
                Text::new("Press ENTER to Save | ESC to Cancel"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.5, 0.5, 0.5)), 
            ));
        });
    });
}

fn handle_star_clicked_event(
    mut events: EventReader<StarClicked>,
    mut rename_state: ResMut<RenameState>,
    mut q_overlay: Query<&mut Visibility, With<RenameOverlayRoot>>,
    mut q_input_text: Query<&mut Text, With<RenameInputText>>,
    mut time: ResMut<Time<Virtual>>,
    db: Res<Database>,
) {
    if rename_state.active { return; }

    for ev in events.read() {
        rename_state.active = true;
        rename_state.target_cell = Some(ev.cell);
        
        // Fetch current name
        if let Ok(Some(disc)) = db.get_discovery(ev.cell) {
            rename_state.current_name = disc.name;
        } else {
                rename_state.current_name = format!("S {},{},{}", ev.cell.x, ev.cell.y, ev.cell.z);
        }

        // Show UI
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Visible;
        }
        if let Ok(mut txt) = q_input_text.get_single_mut() {
            txt.0 = rename_state.current_name.clone();
        }

        // PAUSE
        time.pause();
    }
}

fn rename_input_system(
    mut key_evr: EventReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut rename_state: ResMut<RenameState>,
    mut q_overlay: Query<&mut Visibility, With<RenameOverlayRoot>>,
    mut q_input_text: Query<&mut Text, With<RenameInputText>>,
    mut time: ResMut<Time<Virtual>>,
    db: Res<Database>,
) {
    if !rename_state.active { return; }

    // 1. Handle Special Keys
    if keys.just_pressed(KeyCode::Escape) {
        // Cancel
        rename_state.active = false;
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        time.unpause();
        return;
    }

    if keys.just_pressed(KeyCode::Enter) {
        // Submit
        let new_name = rename_state.current_name.trim();
        if !new_name.is_empty() && new_name.len() <= 100 {
             if let Some(cell) = rename_state.target_cell {
                 let discovery = Discovery {
                     cell_x: cell.x,
                     cell_y: cell.y,
                     cell_z: cell.z,
                     name: new_name.to_string(),
                     finder: "Player".to_string(),
                     note: "Renamed".to_string(),
                     date: "2026".to_string(), 
                     object_type: "Star System".to_string(),
                 };
                 
                 if let Err(e) = db.save_discovery(&discovery) {
                     error!("Failed to save rename: {}", e);
                 } else {
                     info!("Renamed system to: {}", new_name);
                 }
             }
        }
        
        rename_state.active = false;
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        time.unpause();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        rename_state.current_name.pop();
    }

    // 2. Handle Text Input via Logical Keys
    for ev in key_evr.read() {
        if ev.state.is_pressed() {
            if let Key::Character(ref s) = ev.logical_key {
                // Ensure not control char? s is SmolStr
                let c = s.chars().next().unwrap_or('\0');
                if !c.is_control() && rename_state.current_name.len() < 100 {
                     if c.is_alphanumeric() || "-+=# ".contains(c) {
                         rename_state.current_name.push(c);
                     }
                }
            } else if let Key::Space = ev.logical_key {
                 if rename_state.current_name.len() < 100 {
                     rename_state.current_name.push(' ');
                 }
            }
        }
    }
    
    // 3. Update UI
    if let Ok(mut txt) = q_input_text.get_single_mut() {
        txt.0 = rename_state.current_name.clone() + "_"; 
    }
}
