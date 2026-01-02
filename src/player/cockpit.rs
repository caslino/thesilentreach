use bevy::prelude::*;
use crate::player::camera::ZenCamera;

pub struct CockpitPlugin;

impl Plugin for CockpitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_cockpit_structure);
    }
}

#[derive(Component)]
struct CockpitFrame;

fn spawn_cockpit_structure(
    mut commands: Commands,
    q_camera: Query<Entity, With<ZenCamera>>, 
    q_children: Query<&Children>,
    q_frame: Query<&CockpitFrame>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for camera_entity in q_camera.iter() {
        let mut has_cockpit = false;
        if let Ok(children) = q_children.get(camera_entity) {
            for child in children {
                if q_frame.contains(*child) {
                    has_cockpit = true;
                    break;
                }
            }
        }

        if !has_cockpit {
            // Spawn Cockpit
            let cockpit_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.15),
                metallic: 0.8,
                perceptual_roughness: 0.2,
                ..default()
            });

            commands.entity(camera_entity).with_children(|parent| {
                // Main Frame
                parent.spawn((
                    Transform::default(),
                    Visibility::default(),
                    CockpitFrame,
                )).with_children(|frame| {
                    // Top Bar
                    frame.spawn((
                        Mesh3d(meshes.add(Cuboid::new(2.0, 0.1, 0.1))),
                        MeshMaterial3d(cockpit_mat.clone()),
                        Transform::from_xyz(0.0, 0.8, -1.0),
                    ));
                    // Bottom Bar
                    frame.spawn((
                        Mesh3d(meshes.add(Cuboid::new(2.0, 0.2, 0.2))),
                        MeshMaterial3d(cockpit_mat.clone()),
                        Transform::from_xyz(0.0, -0.8, -1.0),
                    ));
                    // Left Pillar
                    frame.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.1, 1.6, 0.1))),
                        MeshMaterial3d(cockpit_mat.clone()),
                        Transform::from_xyz(-1.0, 0.0, -1.0),
                    ));
                    // Right Pillar
                    frame.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.1, 1.6, 0.1))),
                        MeshMaterial3d(cockpit_mat.clone()),
                        Transform::from_xyz(1.0, 0.0, -1.0),
                    ));
                });
            });
        }
    }
}
