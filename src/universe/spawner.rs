use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrame};

pub struct StarSystemSpawnerPlugin;

impl Plugin for StarSystemSpawnerPlugin {
    fn build(&self, app: &mut App) {
        // Run in Startup (implied after PreStartup)
        app.add_systems(Startup, spawn_test_universe);
    }
}

// Spawns a few celestial objects so the user has something to see/navigate to.
fn spawn_test_universe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
) {
    let big_space_id = q_big_space.single();

    commands.entity(big_space_id).with_children(|root| {
        // SUN
        root.spawn((
            Mesh3d(meshes.add(Sphere::new(100.0).mesh().ico(5).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.8, 0.1),
                emissive: LinearRgba::new(10.0, 8.0, 1.0, 1.0), // Glow
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
            GridCell::<i64>::ZERO,
        )).with_children(|parent| {
            // Light source for the sun
            parent.spawn(PointLight {
                intensity: 10_000_000_000.0, // Very bright
                range: 1_000_000.0,
                ..default()
            });
        });

        // Planet 1 (Green)
        root.spawn((
            Mesh3d(meshes.add(Sphere::new(50.0).mesh().ico(5).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgb(0.2, 0.8, 0.2)))),
            Transform::from_xyz(0.0, 0.0, -2000.0),
            GridCell::<i64>::ZERO, 
        ));

        // Planet 2 (Blue)
        root.spawn((
             Mesh3d(meshes.add(Sphere::new(80.0).mesh().ico(5).unwrap())),
             MeshMaterial3d(materials.add(StandardMaterial::from(Color::srgb(0.2, 0.2, 0.9)))),
             Transform::from_xyz(2000.0, 0.0, 2000.0),
             GridCell::<i64>::ZERO, 
        ));
    });

    // Ambient light to ensure we see things even if point light is far
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 100.0,
    });
}
