use bevy::prelude::*;

use crate::persistence::{Database, Discovery, PlayerName};
use bevy::input::keyboard::{Key, KeyboardInput};
// use crate::universe::materials::{StarMaterial, PlanetMaterial};
use crate::player::camera::{Velocity, ZenCamera};
use crate::universe::spawner::{GalaxyMap, SpawnTracker}; // Needed to find entity from cell
use crate::universe::{PlanetDetails, SectorIndex, StarClicked, StarDetails, SystemSavedEvent};
use big_space::prelude::*; 

pub struct SystemConsolePlugin;

impl Plugin for SystemConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConsoleState>()
            .add_systems(Startup, setup_console_ui)
            .add_systems(
                Update,
                (
                    console_input_system.run_if(console_is_active),
                    handle_star_clicked_event.run_if(console_is_inactive),
                    teleport_to_origin_system.run_if(console_is_inactive),
                    handle_spawn_command_event,
                )
                    .chain(),
            )
            .add_event::<SpawnCommandEvent>(); // Chained to prevent input conflict
    }
}

#[derive(Event)]
pub struct SpawnCommandEvent(pub crate::universe::StarType);

#[derive(Default, PartialEq, Eq, Clone, Copy)]
pub enum ConsoleFocus {
    #[default]
    Name,
    Note,
}

#[derive(Resource, Default)]
pub struct ConsoleState {
    pub active: bool,
    pub spawn_mode: bool, // If true, we are in the spawn/jump menu
    pub target_cell: Option<big_space::prelude::GridCell>,
    pub target_entity: Option<Entity>, // Specific entity (Star or Planet)
    pub current_name: String,
    pub current_note: String,
    pub focus: ConsoleFocus,
}

pub fn console_is_active(state: Res<ConsoleState>) -> bool {
    state.active
}

pub fn console_is_inactive(state: Res<ConsoleState>) -> bool {
    !state.active
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

#[derive(Component)]
struct SpawnTypeButton(crate::universe::StarType);

#[derive(Component)]
struct RegistryPanel;

fn setup_console_ui(mut commands: Commands) {
    commands
        .spawn((
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
        ))
        .with_children(|parent| {
            // Main Container (Horizontal)
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    ..default()
                }, Visibility::default()))
                .with_children(|main_row| {
                    // LEFT PANEL: REGISTRY
                    main_row
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(20.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                width: Val::Px(500.0),
                                margin: UiRect::right(Val::Px(20.0)),
                                ..default()
                            },
                            BorderColor(Color::WHITE),
                            BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                            RegistryPanel,
                            Visibility::default(),
                        ))
                        .with_children(|panel| {
                            // Header
                            panel.spawn((
                                Text::new("SYSTEM REGISTRY"),
                                TextFont {
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.8, 0.2)),
                            ));

                            panel.spawn((Node {
                                height: Val::Px(20.0),
                                ..default()
                            }, Visibility::default()));

                            // Name Label
                            panel.spawn((
                                Text::new("Registry Identity:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                Node {
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Target Label (Dynamic)
                            panel.spawn((
                                Text::new("Target: Scanning..."),
                                TextFont {
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.0, 1.0, 1.0)), // Cyan
                                TargetLabelText,
                                Node {
                                    margin: UiRect::bottom(Val::Px(10.0)),
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Name Input
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 32.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                NameInputText,
                                Node {
                                    margin: UiRect::bottom(Val::Px(20.0)),
                                    ..default()
                                },
                            ));

                            // Coordinates Display
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                                CoordinatesText,
                                Node {
                                    margin: UiRect::bottom(Val::Px(10.0)),
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Named By Label (Dynamic)
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.7, 0.2)), // Goldish
                                NamedByText,
                                Node {
                                    margin: UiRect::bottom(Val::Px(5.0)),
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Composition Label (Dynamic)
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.6, 0.8, 1.0)), // Cyanish
                                CompositionText,
                                Node {
                                    margin: UiRect::bottom(Val::Px(20.0)),
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Note Label
                            panel.spawn((
                                Text::new("Zen Note:"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                                Node {
                                    align_self: AlignSelf::FlexStart,
                                    ..default()
                                },
                            ));

                            // Note Input
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                                NoteInputText,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(60.0),
                                    ..default()
                                },
                            ));

                            panel.spawn((Node {
                                height: Val::Px(20.0),
                                ..default()
                            }, Visibility::default()));

                            // Footer
                            panel.spawn((
                                Text::new("[TAB] Switch Field | [ENTER] Save | [ESC] Cancel"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                            ));
                        });

                    // RIGHT PANEL: INTERSTELLAR TERMINAL (JUMP)
                    main_row
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(20.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                width: Val::Px(250.0),
                                ..default()
                            },
                            BorderColor(Color::srgb(0.0, 0.6, 1.0)), // Blue Border
                            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
                            Visibility::default(),
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new("JUMP TERMINAL"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.0, 1.0, 1.0)),
                            ));

                            panel.spawn((Node {
                                height: Val::Px(15.0),
                                ..default()
                            }, Visibility::default()));

                            panel.spawn((
                                Text::new("Select Destination:"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.5, 0.5, 0.8)),
                                Node {
                                    margin: UiRect::bottom(Val::Px(10.0)),
                                    ..default()
                                },
                            ));

                            let stars = vec![
                                ("O_BlueGiant", crate::universe::StarType::O_BlueGiant),
                                ("B_BlueWhite", crate::universe::StarType::B_BlueWhite),
                                ("A_White", crate::universe::StarType::A_White),
                                ("F_YellowWhite", crate::universe::StarType::F_YellowWhite),
                                ("G_YellowDwarf", crate::universe::StarType::G_YellowDwarf),
                                ("K_OrangeDwarf", crate::universe::StarType::K_OrangeDwarf),
                                ("M_RedDwarf", crate::universe::StarType::M_RedDwarf),
                                ("NeutronStar", crate::universe::StarType::NeutronStar),
                                ("BlackHole", crate::universe::StarType::BlackHole),
                            ];

                            for (label, star_type) in stars {
                                panel
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(30.0),
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            margin: UiRect::bottom(Val::Px(5.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.1, 0.1, 0.2)),
                                        SpawnTypeButton(star_type),
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(label),
                                            TextFont {
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(Color::WHITE),
                                        ));
                                    });
                            }

                            panel.spawn((Node {
                                height: Val::Px(20.0),
                                ..default()
                            }, Visibility::default()));

                            panel.spawn((
                                Text::new("Type '/spawn <type>'\nin registry name field"),
                                TextFont {
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.4, 0.4, 0.6)),
                                TextLayout::new_with_justify(JustifyText::Center),
                            ));
                        });
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
    q_player: Query<(&GridCell, &Transform), With<ZenCamera>>,
    tracker: Res<SpawnTracker>,
    q_children: Query<&Children>,
    q_transform: Query<&Transform>,
) {
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
        if let Ok((cell, player_tf)) = q_player.single() {
            target_cell = Some(*cell);

            // Find Nearest Entity in this cell
            if let Some(root_entities) = tracker.spawned_cells.get(cell) {
                for root_entity in root_entities {
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
        if let Ok(mut vis) = q_overlay.single_mut() {
            *vis = Visibility::Visible;
        }

        // PAUSE
        time.pause();
    } else if keys.just_pressed(KeyCode::KeyK) {
        // OPEN GENERAL OVERLAY (SPAWN MODE)
        state.active = true;
        state.target_cell = None;
        state.target_entity = None;
        state.current_name = "/spawn ".to_string();
        state.focus = ConsoleFocus::Name;

        if let Ok(mut vis) = q_overlay.single_mut() {
            *vis = Visibility::Visible;
        }
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
    mut spawn_events: EventWriter<SpawnCommandEvent>,
    player_name: Res<PlayerName>,
    mut q_buttons: Query<(&Interaction, &SpawnTypeButton), Changed<Interaction>>,
) {
    // A. Handle Button Interactions (Even if console not fully 'active' in registry mode)
    for (interaction, spawn_btn) in q_buttons.iter_mut() {
        if *interaction == Interaction::Pressed {
            spawn_events.write(SpawnCommandEvent(spawn_btn.0));
            save_events.write(SystemSavedEvent {
                name: format!("JUMPING TO {:?}", spawn_btn.0),
            });
            state.active = false;
            // Clear inputs for next time
            state.current_name.clear();
            state.current_note.clear();
            
            if let Ok(mut vis) = q_overlay.single_mut() {
                *vis = Visibility::Hidden;
            }
            time.unpause();
            return;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        state.active = false;
        state.target_entity = None; // Clear target on close to prevent stale data
        if let Ok(mut vis) = q_overlay.single_mut() {
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
        // Handle Commands first
        let current_text = state.current_name.trim().to_lowercase();
        if current_text.starts_with("/jump") || current_text.starts_with("/spawn") {
            let parts: Vec<&str> = current_text.split_whitespace().collect();
            if parts.len() > 1 {
                if let Some(star_type) = crate::universe::StarType::from_str(parts[1]) {
                    spawn_events.write(SpawnCommandEvent(star_type));
                    save_events.write(SystemSavedEvent {
                        name: format!("JUMPING TO {:?}", star_type),
                    });
                    state.current_name.clear();
                    state.current_note.clear();
                    state.active = false;
                    if let Ok(mut vis) = q_overlay.single_mut() {
                        *vis = Visibility::Hidden;
                    }
                    time.unpause();
                    return;
                }
            }
        }

        // Save (Existing logic)
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
                    finder: player_name.0.clone(), // Use dynamic name
                    note: note.clone(), // Save user note
                    date: "2026".to_string(),
                    object_type: "Star System".to_string(), // Keep simple for now
                };

                if let Err(e) = db.save_discovery(&discovery) {
                    error!("Failed to save console data: {}", e);
                } else {
                    info!("Saved system system: {} | Note: {}", name, note);

                    // Trigger Toast
                    save_events.write(SystemSavedEvent { name: name.clone() });
                }
            }
        }

        state.current_name.clear();
        state.current_note.clear();
        state.active = false;
        state.target_entity = None; // Clear target on save too
        if let Ok(mut vis) = q_overlay.single_mut() {
            *vis = Visibility::Hidden;
        }
        time.unpause();
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        match state.focus {
            ConsoleFocus::Name => {
                state.current_name.pop();
            }
            ConsoleFocus::Note => {
                state.current_note.pop();
            }
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
                            if state.current_name.len() < 100
                                && (c.is_alphanumeric() || "-+=# /_".contains(c))
                            {
                                state.current_name.push(c);
                            }
                        }
                        ConsoleFocus::Note => {
                            if state.current_note.len() < 500 {
                                // Max note length
                                state.current_note.push(c);
                            }
                        }
                    }
                }
            } else if let Key::Space = ev.logical_key {
                match state.focus {
                    ConsoleFocus::Name => {
                        if state.current_name.len() < 100 {
                            state.current_name.push(' ');
                        }
                    }
                    ConsoleFocus::Note => {
                        if state.current_note.len() < 500 {
                            state.current_note.push(' ');
                        }
                    }
                }
            }
        }
    }

    // 3. Update UI
    let cursor_char = "_";

    // Name Logic
    let mut is_named = false;

    if let Ok((mut txt, mut color)) = q_text_set.p0().single_mut() {
        let cursor = if state.focus == ConsoleFocus::Name {
            cursor_char
        } else {
            ""
        };

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
    if let Ok(mut txt) = q_text_set.p3().single_mut() {
        if is_named {
            txt.0 = format!("Named by: {}", player_name.0);
        } else {
            txt.0 = "".to_string();
        }
    }

    // Composition Logic
    if let Ok(mut txt) = q_text_set.p4().single_mut() {
        let mut desc = "Scanning...".to_string();

        let mut entity_to_check = state.target_entity;

        // If no entity (Enter key), try to find Star in current cell
        if entity_to_check.is_none() {
            if let Some(cell) = state.target_cell {
                if let Some(root_entities) = tracker.spawned_cells.get(&cell) {
                    for root_entity in root_entities {
                        // Find child with StarDetails
                        if let Ok(children) = q_children.get(*root_entity) {
                            for child in children {
                                if q_star_details.get(*child).is_ok() {
                                    entity_to_check = Some(*child);
                                    break;
                                }
                            }
                        }
                        if entity_to_check.is_some() {
                            break;
                        }
                    }
                }
            }
        }

        if let Some(e) = entity_to_check {
            if let Ok(star) = q_star_details.get(e) {
                // Approximate class
                let c = LinearRgba::from(star.color);
                let class = if c.red > 0.9 && c.green < 0.5 {
                    "M (Red Dwarf)"
                } else if c.blue > 0.8 {
                    "O (Blue Giant)"
                } else if c.green > 0.8 {
                    "G (Yellow Dwarf)"
                } else {
                    "K (Orange Dwarf)"
                };

                desc = format!("Class: {} | Radius: {:.1} units", class, star.size);
            } else if let Ok(planet) = q_planet_details.get(e) {
                desc = planet.0.description();
            }
        }

        // info!("Composition Text set to: {}", desc); // DEBUG
        txt.0 = desc;
    }

    // Target Label Logic
    if let Ok(mut txt) = q_text_set.p5().single_mut() {
        if let Some(e) = state.target_entity {
            // Try to find the Text2d child which has the label?
            // Actually, SystemLabel is on the text child of the entity usually?
            // Wait, my spawner puts SystemLabel on the Text2d child OF the star/planet.
            // So if `e` is the Star/Planet, we need to find its children with SystemLabel.

            let found_name;
            if let Ok(children) = q_children.get(e) {
                for child in children {
                    if let Ok(_label) = q_system_label.get(*child) {
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
    if let Ok(mut txt) = q_text_set.p2().single_mut() {
        if let Some(cell) = state.target_cell {
            txt.0 = format!("Coordinates: [{}, {}, {}]", cell.x, cell.y, cell.z);
        }
    }

    if let Ok(mut txt) = q_text_set.p1().single_mut() {
        let cursor = if state.focus == ConsoleFocus::Note {
            cursor_char
        } else {
            ""
        };
        // Simple word wrap simulation (visual only, for now just raw string)
        if state.current_note.is_empty() && state.focus != ConsoleFocus::Note {
            txt.0 = "[No Note]".to_string(); // Placeholder
        // Set color grey? Can't easily change color here without query.
        } else {
            txt.0 = format!("{}{}", state.current_note, cursor);
        }
    }
}

fn handle_spawn_command_event(
    mut events: EventReader<SpawnCommandEvent>,
    mut config: ResMut<crate::universe::UniverseConfig>,
    mut p_config: ResMut<crate::persistence::PersistenceConfig>,
    mut tracker: ResMut<SpawnTracker>,
    mut galaxy_map: ResMut<GalaxyMap>,
    mut q_player: Query<(&mut GridCell, &mut Transform, &mut Velocity), With<ZenCamera>>,
    mut commands: Commands,
) {
    for ev in events.read() {
        let star_type = ev.0;
        info!("TERMINAL: Executing Jump to {:?}", star_type);

        // 1. Update Configs
        config.star_override = Some(star_type);
        p_config.star_override = Some(star_type);

        // 2. Clear Origin Sector Data to force regeneration
        let origin_cell = GridCell::new(0, 0, 0);
        let origin_sector = SectorIndex::from_cell(origin_cell);

        // Remove from GalaxyMap so it generates from scratch with override
        galaxy_map.sectors.remove(&origin_sector);

        // Remove from SpawnTracker so it despawns and respawns
        if let Some(entities) = tracker.spawned_cells.remove(&origin_cell) {
            for entity in entities {
                commands.entity(entity).despawn();
            }
        }

        // 3. Teleport Player
        if let Ok((mut cell, mut tf, mut vel)) = q_player.single_mut() {
            *cell = origin_cell;
            vel.0 = Vec3::ZERO;

            // Positioning
            let (_, max_size) = star_type.get_size_range();
            let spawn_dist = (max_size * 3.0).max(100.0);
            tf.translation = Vec3::new(0.0, 0.0, spawn_dist);
            tf.look_at(Vec3::ZERO, Vec3::Y);

            info!(
                "TERMINAL: Jump complete. Positioned at distance {:.1}",
                spawn_dist
            );
        }
    }
}

fn teleport_to_origin_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_player: Query<
        (&mut GridCell, &mut crate::player::camera::Velocity),
        With<ZenCamera>,
    >,
) {
    if keys.just_pressed(KeyCode::KeyO) {
        if let Ok((mut cell, mut vel)) = q_player.single_mut() {
            *cell = GridCell::new(0, 0, 0);
            vel.0 = Vec3::ZERO;
            info!("PLAYER: Teleported to Origin System (0,0,0)");
        }
    }
}
