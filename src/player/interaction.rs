use bevy::prelude::*;

use bevy::input::keyboard::{KeyboardInput, Key};
use crate::persistence::{Database, Discovery};
// use crate::universe::materials::{StarMaterial, PlanetMaterial};
use crate::universe::{StarClicked, SystemSavedEvent, StarDetails, PlanetDetails};
use crate::universe::spawner::SpawnTracker; // Needed to find entity from cell
use crate::player::camera::ZenCamera;
use big_space::GridCell;

pub struct SystemConsolePlugin;

impl Plugin for SystemConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConsoleState>()
           .add_systems(Startup, setup_console_ui)
           .add_systems(Update, (console_input_system, handle_star_clicked_event).chain()); // Chained to prevent input conflict
    }
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum ConsoleFocus {
    #[default]
    Name,
    Note,
}

#[derive(Resource, Default)]
pub struct ConsoleState {
    pub active: bool,
    target_cell: Option<big_space::GridCell<i64>>,
    target_entity: Option<Entity>, // Specific entity (Star or Planet)
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

#[derive(Component)]
struct CoordinatesText;

#[derive(Component)]
struct NamedByText;

#[derive(Component)]
struct CompositionText;

#[derive(Component)]
struct TargetLabelText;

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
                Text::new("System Registry:"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node { align_self: AlignSelf::FlexStart, ..default() },
            ));

            // Target Label (Dynamic)
            panel.spawn((
                Text::new("Target: Scanning..."),
                TextFont { font_size: 16.0, ..default() }, // Bold/Larger
                TextColor(Color::srgb(0.0, 1.0, 1.0)), // Cyan
                TargetLabelText,
                Node { margin: UiRect::bottom(Val::Px(10.0)), align_self: AlignSelf::FlexStart, ..default() },
            ));

            // Name Input
            panel.spawn((
                Text::new(""),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
                NameInputText,
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));

            // Coordinates Display
            panel.spawn((
                Text::new(""),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                CoordinatesText,
                Node { margin: UiRect::bottom(Val::Px(10.0)), align_self: AlignSelf::FlexStart, ..default() },
            ));
            
            // Named By Label (Dynamic)
            panel.spawn((
                Text::new(""),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.9, 0.7, 0.2)), // Goldish
                NamedByText,
                Node { margin: UiRect::bottom(Val::Px(5.0)), align_self: AlignSelf::FlexStart, ..default() },
            ));

             // Composition Label (Dynamic)
            panel.spawn((
                Text::new(""),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgb(0.6, 0.8, 1.0)), // Cyanish
                CompositionText,
                Node { margin: UiRect::bottom(Val::Px(20.0)), align_self: AlignSelf::FlexStart, ..default() },
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
    keys: Res<ButtonInput<KeyCode>>,
    q_player: Query<(&GridCell<i64>, &Transform), With<ZenCamera>>,
    tracker: Res<SpawnTracker>,
    q_children: Query<&Children>,
    q_transform: Query<&Transform>,
) {
    if state.active { return; }

    // Check for Event OR Enter Key
    let mut target_cell = None;
    let mut target_entity = None;

    // 1. Event (Click) - Specific Entity
    for ev in events.read() {
        target_cell = Some(ev.cell);
        target_entity = Some(ev.entity);
    }
    
    // 2. Enter Key (Smart Context)
    if target_cell.is_none() && keys.just_pressed(KeyCode::Enter) {
         if let Ok((cell, player_tf)) = q_player.get_single() {
             target_cell = Some(*cell);
             
             // Find Nearest Entity in this cell
             if let Some(root_entity) = tracker.spawned_cells.get(cell) {
                 if let Ok(children) = q_children.get(*root_entity) {
                     // Need root transform to get correct relative position
                     // (Children are relative to System Root + System Root is relative to Camera/Origin)
                     let root_pos = if let Ok(tf) = q_transform.get(*root_entity) {
                         tf.translation
                     } else {
                         Vec3::ZERO
                     };

                     let mut min_dist = f32::MAX;
                     let mut closest = None;
                     
                     for child in children {
                         if let Ok(child_tf) = q_transform.get(*child) {
                             let child_global = root_pos + child_tf.translation;
                             let dist = child_global.distance(player_tf.translation);
                             
                             if dist < min_dist {
                                 min_dist = dist;
                                 closest = Some(*child);
                             }
                         }
                     }
                     target_entity = closest;
                 }
             }
         }
    }

    if let Some(cell) = target_cell {
        state.active = true;
        state.target_cell = Some(cell);
        state.target_entity = target_entity;
        state.focus = ConsoleFocus::Name; // Default focus
        
        // Fetch current name and note
        if let Ok(Some(disc)) = db.get_discovery(cell) {
            state.current_name = disc.name;
            state.current_note = disc.note;
        } else {
            state.current_name = String::new(); // Empty by default to show placeholder
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
    mut q_text_set: ParamSet<(
        Query<(&mut Text, &mut TextColor), With<NameInputText>>,
        Query<&mut Text, With<NoteInputText>>,
        Query<&mut Text, With<CoordinatesText>>,
        Query<&mut Text, With<NamedByText>>,
        Query<&mut Text, With<CompositionText>>,
        Query<&mut Text, With<TargetLabelText>>,
    )>,
    q_star_details: Query<&StarDetails>,
    q_planet_details: Query<&PlanetDetails>,
    q_system_label: Query<&crate::universe::spawner::SystemLabel>,
    tracker: Res<SpawnTracker>,
    q_children: Query<&Children>,
    mut time: ResMut<Time<Virtual>>,
    db: Res<Database>,
    mut save_events: EventWriter<SystemSavedEvent>,
) {
    if !state.active { return; }

    // 1. Handle Control Keys
    if keys.just_pressed(KeyCode::Escape) {
        state.active = false;
        state.target_entity = None; // Clear target on close to prevent stale data
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

                     // Trigger Toast
                     save_events.send(SystemSavedEvent { name: name.clone() });
                 }
             }
        }
        
        state.active = false;
        state.target_entity = None; // Clear target on save too
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
    
    // Name Logic
    let mut is_named = false;
    
    if let Ok((mut txt, mut color)) = q_text_set.p0().get_single_mut() {
        let cursor = if state.focus == ConsoleFocus::Name { cursor_char } else { "" };
        
        let is_default = state.current_name.starts_with("S ") && state.current_name.contains(",");
        
        if state.current_name.is_empty() {
             txt.0 = format!("[can be named]{}", cursor);
             color.0 = Color::srgb(0.5, 0.5, 0.5); // Grey
             is_named = false;
        } else if is_default && state.focus != ConsoleFocus::Name {
             txt.0 = format!("[can be named]{}", cursor); 
             color.0 = Color::srgb(0.5, 0.5, 0.5); // Grey
             is_named = false;
        } else {
             txt.0 = format!("{}{}", state.current_name, cursor);
             color.0 = Color::WHITE;
             is_named = true;
        }
    }
    
    // Named By Logic
    if let Ok(mut txt) = q_text_set.p3().get_single_mut() {
        if is_named {
            txt.0 = "Named by: BigDaddy".to_string();
        } else {
            txt.0 = "".to_string();
        }
    }

    // Composition Logic
    if let Ok(mut txt) = q_text_set.p4().get_single_mut() {
        let mut desc = "Scanning...".to_string();
        
        let mut entity_to_check = state.target_entity;

        // If no entity (Enter key), try to find Star in current cell
        if entity_to_check.is_none() {
            if let Some(cell) = state.target_cell {
                if let Some(root_entity) = tracker.spawned_cells.get(&cell) {
                    // Find child with StarDetails
                    if let Ok(children) = q_children.get(*root_entity) {
                        for child in children {
                            if q_star_details.get(*child).is_ok() {
                                entity_to_check = Some(*child);
                                break;
                            }
                        }
                    }
                }
            }
        }

        if let Some(e) = entity_to_check {
            if let Ok(star) = q_star_details.get(e) {
                // Approximate class
                let c = LinearRgba::from(star.color);
                let class = if c.red > 0.9 && c.green < 0.5 { "M (Red Dwarf)" }
                            else if c.blue > 0.8 { "O (Blue Giant)" }
                            else if c.green > 0.8 { "G (Yellow Dwarf)" }
                            else { "K (Orange Dwarf)" };
                            
                desc = format!("Class: {} | Radius: {:.1} units", class, star.size);
            } else if let Ok(planet) = q_planet_details.get(e) {
                desc = planet.0.description();
            }
        }
        

        
        // info!("Composition Text set to: {}", desc); // DEBUG
        txt.0 = desc;
    }

    // Target Label Logic
    if let Ok(mut txt) = q_text_set.p5().get_single_mut() {
        if let Some(e) = state.target_entity {
            // Try to find the Text2d child which has the label? 
            // Actually, SystemLabel is on the text child of the entity usually? 
            // Wait, my spawner puts SystemLabel on the Text2d child OF the star/planet.
            // So if `e` is the Star/Planet, we need to find its children with SystemLabel.
            
            let mut found_name = "Unknown Object".to_string();
            if let Ok(children) = q_children.get(e) {
                for child in children {
                    if let Ok(label) = q_system_label.get(*child) {
                         // We found the label component. But the text is on this child too?
                         // SystemLogic ref: Text2d is the component.
                         // But we can't query Text2d easily here without adding to system param.
                         // Let's just assume identity for now or use Description?
                         // Actually, let's use the spawner's generated text if we can access it using Text2d query
                         // But `bevy::text::Text2d` might be hard to read here without `Text` 
                         // Wait, in Bevy 0.15 Text2d uses `Text` component? Or `TextLayout`? 
                         // Looking at spawner code: 
                         // star.spawn((Text2d::new(star_name) ... SystemLabel));
                         // So it has a Text2d component. 
                         // Bevy 0.15: Text2d is a component? Yes. 
                         // Actually, let's just re-derive the name from type for now to be safe and simple, 
                         // OR, just say "Target: Star/Planet".
                         // Better: We have star/planet details.
                    }
                }
            }
            
            // Simplified Name Derivation since Text2d access is complex to add cleanly right now
             if let Ok(_) = q_star_details.get(e) {
                 found_name = "Star".to_string(); 
             } else if let Ok(_) = q_planet_details.get(e) {
                 found_name = "Planet".to_string();
             } else {
                 found_name = "Unidentified Body".to_string(); // Renamed from Unknown Object
             }
             txt.0 = format!("Target: {}", found_name);
        } else {
             txt.0 = "Target: System (General)".to_string();
        }
    }
    
    // Coords Logic
    if let Ok(mut txt) = q_text_set.p2().get_single_mut() {
        if let Some(cell) = state.target_cell {
            txt.0 = format!("Coordinates: [{}, {}, {}]", cell.x, cell.y, cell.z);
        }
    }
    
    if let Ok(mut txt) = q_text_set.p1().get_single_mut() {
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
