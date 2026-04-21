use crate::persistence::Database;
use crate::universe::spawner::SystemLabel;
use crate::universe::{Planet, Star};
use bevy::prelude::*;
use big_space::prelude::*;

pub struct LabelUpdatePlugin;

impl Plugin for LabelUpdatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_system_labels, update_billboards));
    }
}

#[derive(Component)]
pub struct Billboard; // Tag for things that should face camera

#[derive(Component)]
struct LastLabelUpdate(f32);

fn update_billboards(
    mut q_billboards: Query<
        (&GlobalTransform, &mut Transform),
        (With<Billboard>, With<SystemLabel>),
    >,
    q_camera: Query<&GlobalTransform, With<crate::player::camera::ZenCamera>>,
) {
    let Ok(cam_tf) = q_camera.single() else {
        return;
    };
    let _cam_pos = cam_tf.translation();

    for (_global_tf, mut local_tf) in q_billboards.iter_mut() {
        // Simple look_at in local space is tricky if parent rotates.
        // But Star/Planet don't rotate heavily yet (except float).
        // Text2d is usually Y-up?
        // We want the text Z-axis to point to camera? No, Text2d normal is Z.
        // let target = cam_pos;
        // local_tf.look_at(target, Vec3::Y);
        // Wait, local_tf is relative to parent.
        // Easier: Set rotation to Camera's rotation?
        // If we want it strictly billboarded:
        // Render 2D pass? No.
        // Just match camera rotation.
        // But `local_tf.rotation = cam_tf.rotation` assumes parent has identity rotation.
        // Parent (Star/Planet) might rotate?
        // If parent rotates, we need to compensate.
        // For now, assuming Parent rotation is identity or we ignore it (Star doesn't rotate mesh, logic rotates?).

        // Actually, let's just use `look_at` on the global pos relative to camera?
        // We can't set global transform directly easily without `GlobalTransform` commands?
        // Bevy pro-tip: Just set `rotation` to `camera.rotation` if parents are static.
        // Stars/Planets are static rotational-wise (except Orbit which is translation).
        // Wait, `Star` mesh rotates? No.
        // So `local_tf.rotation = cam_tf.rotation()` works perfectly to align with camera plane.

        local_tf.rotation = cam_tf.rotation();
    }
}

fn update_system_labels(
    mut commands: Commands,
    mut q_labels: Query<
        (Entity, &mut Text2d, &ChildOf, Option<&mut LastLabelUpdate>),
        With<SystemLabel>,
    >,
    q_parents: Query<(Option<&Star>, Option<&Planet>)>,
    q_grandparents: Query<&GridCell>, // Parent (Star/Planet) -> Parent (SystemRoot/GridCell)
    // Wait, Star is child of SystemRoot. SystemRoot has GridCell.
    // Planet is child of SystemRoot too?
    // Let's check spawner.rs.
    // Star: root.spawn(...).with_children(|star| ... with label ...).
    // root is `system_root` (GridCell).
    // So Label -> Star -> SystemRoot(GridCell).
    // Planet: root.spawn(...).with_children(|planet| ... with label ...).
    // So Label -> Planet -> SystemRoot(GridCell).
    // Hierarchy is consistent: Label -> Object -> SystemRoot.
    q_hierarchy: Query<&ChildOf>,
    db: Res<Database>,
    time: Res<Time>,
) {
    for (entity, mut text, parent, last_update) in q_labels.iter_mut() {
        // Throttling: Check every 1 second
        let now = time.elapsed_secs();
        if let Some(mut last) = last_update {
            if now - last.0 < 1.0 {
                continue;
            }
            last.0 = now;
        } else {
            commands
                .entity(entity)
                .insert((LastLabelUpdate(now), Billboard)); // Add Billboard here!
        }

        if let Ok(object_parent) = q_hierarchy.get(parent.parent()) {
            // Determine Type
            let (_is_star, is_planet) = if let Ok((s, p)) = q_parents.get(parent.parent()) {
                (s.is_some(), p.is_some())
            } else {
                (false, false)
            };

            if let Ok(root_parent) = q_hierarchy.get(object_parent.parent()) {
                if let Ok(cell) = q_grandparents.get(root_parent.parent()) {
                    match db.get_discovery(*cell) {
                        Ok(Some(discovery)) => {
                            let default_name = format!("S {},{},{}", cell.x, cell.y, cell.z);

                            if discovery.name == default_name {
                                // DB has default "S ...".
                                if is_planet {
                                    // Ensure Planet keeps "P ..."
                                    let planet_name = format!("P {},{},{}", cell.x, cell.y, cell.z);
                                    if text.0 != planet_name {
                                        text.0 = planet_name;
                                    }
                                } else {
                                    // Star keeps "S ..."
                                    if text.0 != default_name {
                                        text.0 = default_name;
                                    }
                                }
                            } else {
                                // Custom Name "MyHome"
                                // Apply to both? Or prefix/suffix?
                                // For now, straightforward application.
                                // Maybe "MyHome (P)" for planet?
                                if text.0 != discovery.name {
                                    if is_planet {
                                        let p_name = format!("{} (Planet)", discovery.name);
                                        if text.0 != p_name {
                                            text.0 = p_name;
                                        }
                                    } else {
                                        text.0 = discovery.name.clone();
                                    }
                                }
                            }
                        }
                        Ok(None) => {} // Keep existing (spawned) label
                        Err(_) => {}
                    }
                }
            }
        }
    }
}
