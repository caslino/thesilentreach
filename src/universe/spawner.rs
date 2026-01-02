use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrame, FloatingOrigin};
use crate::universe::{UniverseSeed, Mass};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct StarSystemSpawnerPlugin;

impl Plugin for StarSystemSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTracker>()
           .add_systems(Update, (spawn_procedural_systems, despawn_distant_systems));
    }
}

#[derive(Resource, Default)]
struct SpawnTracker {
    spawned_cells: HashMap<GridCell<i64>, Entity>,
}

fn spawn_procedural_systems(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    seed: Res<UniverseSeed>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    let Ok(big_space_entity) = q_big_space.get_single() else { return; };

    let radius = 1; // Check immediate neighbors (3x3x3 grid)

    for x in -radius..=radius {
        for y in -radius..=radius {
            for z in -radius..=radius {
                let neighbor_cell = GridCell::<i64>::new(
                    camera_cell.x + x,
                    camera_cell.y + y,
                    camera_cell.z + z,
                );

                if tracker.spawned_cells.contains_key(&neighbor_cell) {
                    continue;
                }

                // Deterministic Seed Generation
                let mut hasher = DefaultHasher::new();
                neighbor_cell.hash(&mut hasher);
                seed.0.hash(&mut hasher); 
                let cell_seed = hasher.finish();
                let mut rng = StdRng::seed_from_u64(cell_seed);

                // Force spawn at (0,0,0) so player always starts with a star
                let is_origin = neighbor_cell.x == 0 && neighbor_cell.y == 0 && neighbor_cell.z == 0;
                let chance = if is_origin { 1.0 } else { 0.5 }; // 50% density otherwise

                if rng.gen_bool(chance) {
                    let star_entity = spawn_star_system(
                        &mut commands, 
                        big_space_entity, 
                        neighbor_cell, 
                        &mut rng,
                        &mut meshes, 
                        &mut materials
                    );
                    tracker.spawned_cells.insert(neighbor_cell, star_entity);
                    info!("Spawned system at {:?}", neighbor_cell);
                }
            }
        }
    }
}

fn despawn_distant_systems(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    let removal_radius = 2; // Despawn if further than 2 cells away

    tracker.spawned_cells.retain(|cell, entity| {
        let dx = (cell.x - camera_cell.x).abs();
        let dy = (cell.y - camera_cell.y).abs();
        let dz = (cell.z - camera_cell.z).abs();
        
        if dx > removal_radius || dy > removal_radius || dz > removal_radius {
            commands.entity(*entity).despawn_recursive();
            info!("Despawned system at {:?}", cell);
            false
        } else {
            true
        }
    });
}

fn spawn_star_system(
    commands: &mut Commands,
    parent_id: Entity,
    cell: GridCell<i64>,
    rng: &mut StdRng,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let star_color = Color::srgb(rng.r#gen::<f32>(), rng.r#gen::<f32>(), rng.r#gen::<f32>());
    let star_size = rng.gen_range(50.0..200.0);

    let system_root = commands.spawn((
        Transform::default(),
        Visibility::default(),
        cell, // Position in the big_space
    )).id();

    // Attach to BigSpace
    commands.entity(parent_id).add_child(system_root);

    // Spawn Star Visuals
    commands.entity(system_root).with_children(|root| {
        // Star Sphere
        root.spawn((
            Mesh3d(meshes.add(Sphere::new(star_size).mesh().ico(5).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: star_color,
                emissive: LinearRgba::from(star_color) * 5.0,
                ..default()
            })),
            Mass(1_000_000.0), // Massive gravity well
            Transform::IDENTITY,
        )).with_children(|star| {
             // Light
             star.spawn(PointLight {
                color: star_color,
                intensity: 10_000_000_000.0,
                range: 2_000_000.0,
                ..default()
            });
        });

        // Spawn Planets
        let num_planets = rng.gen_range(0..=5);
        for _i in 0..num_planets {
            let dist = rng.gen_range(500.0..5000.0) + star_size;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let planet_size = rng.gen_range(10.0..40.0);
            let planet_color = Color::srgb(rng.r#gen::<f32>(), rng.r#gen::<f32>(), rng.r#gen::<f32>());

            let x = dist * angle.cos();
            let z = dist * angle.sin();

            root.spawn((
                Mesh3d(meshes.add(Sphere::new(planet_size).mesh().ico(3).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial::from(planet_color))),
                Mass(10_000.0), // Smaller gravity well
                Transform::from_xyz(x, 0.0, z),
            ));
        }
    });

    system_root
}
