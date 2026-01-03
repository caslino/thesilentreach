use bevy::prelude::*;

use bevy::input::keyboard::{KeyboardInput, Key};
use crate::persistence::{Database, Discovery};
use crate::universe::StarClicked;

pub struct SystemConsolePlugin;

impl Plugin for SystemConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConsoleState>()
           .add_systems(Startup, setup_console_ui)
           .add_systems(Update, (handle_star_clicked_event, console_input_system));
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum ConsoleFocus {
    #[default]
    Name,
    Note,
}

#[derive(Resource, Default)]
struct ConsoleState {
    active: bool,
    target_cell: Option<big_space::GridCell<i64>>,
    current_name: String,
    current_note: String,
    focus: ConsoleFocus,
}

#[derive(Component)]
struct ConsoleOverlayRoot;

#[derive(Component)]
struct NameInputText;

#[derive(Component)]
struct NoteInputText;

fn setup_console_ui(mut commands: Commands) {
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
        ConsoleOverlayRoot,
    )).with_children(|parent| {
        parent.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(20.0)),
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(600.0),
                ..default()
            },
            BorderColor(Color::WHITE),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        )).with_children(|panel| {
            // Header
            panel.spawn((
                Text::new("SYSTEM CONSOLE"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(1.0, 0.8, 0.2)),
            ));
            
            panel.spawn((Node { height: Val::Px(20.0), ..default() },));

            // Name Label
            panel.spawn((
                Text::new("System Name:"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node { align_self: AlignSelf::FlexStart, ..default() },
            ));

            // Name Input
            panel.spawn((
                Text::new(""),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
                NameInputText,
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));
            
            // Note Label
            panel.spawn((
                Text::new("Zen Note:"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                 Node { align_self: AlignSelf::FlexStart, ..default() },
            ));

            // Note Input
             panel.spawn((
                Text::new(""),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                NoteInputText,
                 Node { 
                    width: Val::Percent(100.0),
                    min_height: Val::Px(60.0),
                    ..default() 
                },
            ));
            
            panel.spawn((Node { height: Val::Px(20.0), ..default() },));
            
            // Footer
             panel.spawn((
                Text::new("[TAB] Switch Field | [ENTER] Save | [ESC] Cancel"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.5, 0.5, 0.5)), 
            ));
        });
    });
}

fn handle_star_clicked_event(
    mut events: EventReader<StarClicked>,
    mut state: ResMut<ConsoleState>,
    mut q_overlay: Query<&mut Visibility, With<ConsoleOverlayRoot>>,
    mut time: ResMut<Time<Virtual>>,
    db: Res<Database>,
) {
    if state.active { return; }

    for ev in events.read() {
        state.active = true;
        state.target_cell = Some(ev.cell);
        state.focus = ConsoleFocus::Name; // Default focus
        
        // Fetch current name and note
        if let Ok(Some(disc)) = db.get_discovery(ev.cell) {
            state.current_name = disc.name;
            state.current_note = disc.note;
        } else {
            state.current_name = format!("S {},{},{}", ev.cell.x, ev.cell.y, ev.cell.z);
            state.current_note = String::new();
        }

        // Show UI
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Visible;
        }

        // PAUSE
        time.pause();
    }
}

fn console_input_system(
    mut key_evr: EventReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ConsoleState>,
    mut q_overlay: Query<&mut Visibility, With<ConsoleOverlayRoot>>,
    mut q_name_text: Query<&mut Text, (With<NameInputText>, Without<NoteInputText>)>,
    mut q_note_text: Query<&mut Text, (With<NoteInputText>, Without<NameInputText>)>,
    mut time: ResMut<Time<Virtual>>,
    db: Res<Database>,
) {
    if !state.active { return; }

    // 1. Handle Control Keys
    if keys.just_pressed(KeyCode::Escape) {
        state.active = false;
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        time.unpause();
        return;
    }

    if keys.just_pressed(KeyCode::Tab) {
        state.focus = match state.focus {
            ConsoleFocus::Name => ConsoleFocus::Note,
            ConsoleFocus::Note => ConsoleFocus::Name,
        };
    }

    if keys.just_pressed(KeyCode::Enter) {
        // Save
        if let Some(cell) = state.target_cell {
             let name = state.current_name.trim().to_string();
             let note = state.current_note.trim().to_string();
             
             // Basic Validation
             if !name.is_empty() && name.len() <= 100 {
                 let discovery = Discovery {
                     cell_x: cell.x,
                     cell_y: cell.y,
                     cell_z: cell.z,
                     name: name.clone(),
                     finder: "Player".to_string(),
                     note: note.clone(), // Save user note
                     date: "2026".to_string(), 
                     object_type: "Star System".to_string(), // Keep simple for now
                 };
                 
                 if let Err(e) = db.save_discovery(&discovery) {
                     error!("Failed to save console data: {}", e);
                 } else {
                     info!("Saved system system: {} | Note: {}", name, note);
                 }
             }
        }
        
        state.active = false;
        if let Ok(mut vis) = q_overlay.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        time.unpause();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        match state.focus {
            ConsoleFocus::Name => { state.current_name.pop(); },
            ConsoleFocus::Note => { state.current_note.pop(); },
        }
    }

    // 2. Handle Text Input
    for ev in key_evr.read() {
        if ev.state.is_pressed() {
            if let Key::Character(ref s) = ev.logical_key {
                let c = s.chars().next().unwrap_or('\0');
                if !c.is_control() {
                    match state.focus {
                        ConsoleFocus::Name => {
                            if state.current_name.len() < 100 && (c.is_alphanumeric() || "-+=# ".contains(c)) {
                                state.current_name.push(c);
                            }
                        },
                        ConsoleFocus::Note => {
                            if state.current_note.len() < 500 { // Max note length
                                state.current_note.push(c);
                            }
                        }
                    }
                }
            } else if let Key::Space = ev.logical_key {
                 match state.focus {
                    ConsoleFocus::Name => { if state.current_name.len() < 100 { state.current_name.push(' '); } },
                    ConsoleFocus::Note => { if state.current_note.len() < 500 { state.current_note.push(' '); } },
                }
            }
        }
    }
    
    // 3. Update UI
    let cursor_char = "_";
    
    if let Ok(mut txt) = q_name_text.get_single_mut() {
        let cursor = if state.focus == ConsoleFocus::Name { cursor_char } else { "" };
        txt.0 = format!("{}{}", state.current_name, cursor);
    }
    
    if let Ok(mut txt) = q_note_text.get_single_mut() {
        let cursor = if state.focus == ConsoleFocus::Note { cursor_char } else { "" };
        // Simple word wrap simulation (visual only, for now just raw string)
        if state.current_note.is_empty() && state.focus != ConsoleFocus::Note {
            txt.0 = "[No Note]".to_string(); // Placeholder
             // Set color grey? Can't easily change color here without query.
        } else {
            txt.0 = format!("{}{}", state.current_note, cursor);
        }
    }
}
