use crate::persistence::{CurrentSystemData, Database};
use crate::universe::SystemSavedEvent;
use bevy::prelude::*;

pub struct StarMapPlugin;

impl Plugin for StarMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_starmap)
            .add_systems(
                Update,
                (
                    toggle_starmap.run_if(crate::player::interaction::console_is_inactive),
                    update_starmap_content,
                ),
            );
    }
}

#[derive(Component)]
struct StarMapRoot;

#[derive(Component)]
struct StarMapContent;

#[derive(Component)]
struct StarMapDirty;

fn setup_starmap(mut commands: Commands) {
    // Full screen overlay, hidden by default
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)), // Dark semi-transparent background
            Visibility::Hidden,
            StarMapRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("GALACTIC STAR MAP"),
                TextFont {
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.8, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(20.0)),
                    ..default()
                },
            ));

            // Map Container (The actual plotting area)
            parent.spawn((
                Node {
                    width: Val::Px(600.0), // Fixed size for now
                    height: Val::Px(600.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor(Color::srgba(0.3, 0.3, 0.5, 0.5)),
                BackgroundColor(Color::BLACK),
                StarMapContent,
            ));

            // Instructions
            parent.spawn((
                Text::new("[M] TO CLOSE"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
            ));
        });
}

fn toggle_starmap(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut q_root: Query<(Entity, &mut Visibility), With<StarMapRoot>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        if let Ok((entity, mut vis)) = q_root.get_single_mut() {
            *vis = match *vis {
                Visibility::Hidden => {
                    commands.entity(entity).insert(StarMapDirty);
                    Visibility::Visible
                }
                _ => Visibility::Hidden,
            };
        }
    }
}

fn update_starmap_content(
    mut commands: Commands,
    q_root: Query<(Entity, &Visibility, Option<&StarMapDirty>), With<StarMapRoot>>,
    q_content: Query<Entity, With<StarMapContent>>,
    q_children: Query<&Children>,
    db: Res<Database>,
    current_data: Res<CurrentSystemData>,
    mut events: EventReader<SystemSavedEvent>,
) {
    let Ok((root_entity, vis, dirty)) = q_root.single() else {
        return;
    };
    if *vis == Visibility::Hidden {
        return;
    }

    // Only update if visible (Optimization: In real app, only update on open or dirty)
    let system_saved = !events.is_empty();
    events.clear(); // Consume events

    if dirty.is_none() && !current_data.is_changed() && !system_saved {
        return;
    }

    if dirty.is_some() {
        commands.entity(root_entity).remove::<StarMapDirty>();
    }

    let Ok(content_entity) = q_content.single() else {
        return;
    };
    if let Ok(children) = q_children.get(content_entity) {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let my_cell = current_data.cell;

    // Fetch all discoveries (Optimization: Fetch range)
    // Assuming get_all_discoveries is fast enough for <100 items.
    if let Ok(discoveries) = db.get_all_discoveries() {
        commands.entity(content_entity).with_children(|parent| {
            // Map Definition
            // Center = my_cell
            // Scale: 100 GridCells width maps to 600px -> 6px per GridCell
            let range = 50; // +/- 50 GridCells
            let px_per_cell = 600.0 / (range as f32 * 2.0);

            // Plot Center (Player)
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(300.0 - 2.5),
                    top: Val::Px(300.0 - 2.5),
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 1.0, 0.0)), // Me
            ));

            for disc in discoveries {
                let dx = disc.cell_x - my_cell.x;
                let dy = disc.cell_z - my_cell.z; // Map Z to Y usually in 2D top-down

                if dx.abs() > range || dy.abs() > range {
                    continue;
                }

                let ui_x = 300.0 + (dx as f32 * px_per_cell);
                let ui_y = 300.0 + (dy as f32 * px_per_cell);

                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(ui_x - 4.0),
                            top: Val::Px(ui_y - 4.0),
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 1.0, 0.0)),
                        BorderRadius::all(Val::Percent(50.0)),
                    ))
                    .with_children(|dot| {
                        // Tooltip name (simple text child for now)
                        dot.spawn((
                            Text::new(disc.name.clone()),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(10.0),
                                left: Val::Px(-10.0), // center ish
                                ..default()
                            },
                        ));
                    });
            }
        });
    }
}
