use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrame, FloatingOrigin};
use crate::universe::{UniverseSeed, Mass, Radius};
use crate::universe::physics::GRID_SIZE;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct StarSystemSpawnerPlugin;

impl Plugin for StarSystemSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTracker>()
           .init_resource::<GalaxyMap>()
           .add_systems(Update, (manage_galaxy_sectors, sync_universe_view, update_lod_scaling, despawn_distant_systems));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LodLevel {
    Proxy,
    Full,
}

#[derive(Resource, Default)]
struct SpawnTracker {
    spawned_cells: HashMap<GridCell<i64>, (Entity, LodLevel)>,
}

#[derive(Clone, Debug)]
struct StarData {
    color: Color,
    size: f32,
}

// 10x10x10 GridCells per Sector
const SECTOR_SIZE: i64 = 10; 

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct SectorIndex {
    x: i64,
    y: i64,
    z: i64,
}

impl SectorIndex {
    fn from_cell(cell: GridCell<i64>) -> Self {
        Self {
            x: cell.x.div_euclid(SECTOR_SIZE),
            y: cell.y.div_euclid(SECTOR_SIZE),
            z: cell.z.div_euclid(SECTOR_SIZE),
        }
    }
}

#[derive(Resource, Default)]
struct GalaxyMap {
    // Map Sector -> List of Stars in that sector
    sectors: HashMap<SectorIndex, Vec<(GridCell<i64>, StarData)>>,
}

#[derive(Component)]
struct DistantProxy;

fn manage_galaxy_sectors(
    mut galaxy_map: ResMut<GalaxyMap>,
    seed: Res<UniverseSeed>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    
    // Check current sector + Neighbors
    let center_sector = SectorIndex::from_cell(*camera_cell);
    let view_dist = 1; // 3x3x3 sectors

    for x in -view_dist..=view_dist {
        for y in -view_dist..=view_dist {
            for z in -view_dist..=view_dist {
                let sector_idx = SectorIndex {
                    x: center_sector.x + x,
                    y: center_sector.y + y,
                    z: center_sector.z + z,
                };

                if !galaxy_map.sectors.contains_key(&sector_idx) {
                    generate_sector(&mut galaxy_map, sector_idx, &seed);
                }
            }
        }
    }
}

fn generate_sector(
    galaxy_map: &mut GalaxyMap, 
    sector: SectorIndex, 
    seed: &UniverseSeed
) {
    let mut stars = Vec::new();
    
    let start_x = sector.x * SECTOR_SIZE;
    let start_y = sector.y * SECTOR_SIZE;
    let start_z = sector.z * SECTOR_SIZE;

    let end_x = start_x + SECTOR_SIZE;
    let end_y = start_y + SECTOR_SIZE;
    let end_z = start_z + SECTOR_SIZE;

    for x in start_x..end_x {
        for y in start_y..end_y {
            for z in start_z..end_z {
                 let cell = GridCell::<i64>::new(x, y, z);
                
                // Deterministic Check
                let mut hasher = DefaultHasher::new();
                cell.hash(&mut hasher);
                seed.0.hash(&mut hasher); 
                let cell_seed = hasher.finish();
                let mut rng = StdRng::seed_from_u64(cell_seed);

                // Density Check (2%)
                // Since this runs lazily, we don't need origin check, 
                // but if we want the origin (0,0,0) to always have a star:
                let is_origin = x == 0 && y == 0 && z == 0;
                let density_chance = if is_origin { 1.0 } else { 0.02 }; 

                if rng.gen_bool(density_chance) {
                    let (color, size) = generate_star_params(&mut rng);
                    stars.push((cell, StarData { color, size }));
                }
            }
        }
    }

    galaxy_map.sectors.insert(sector, stars);
    // info!("Generated Sector {:?}, stars: {}", sector, galaxy_map.sectors.get(&sector).unwrap().len());
}


fn sync_universe_view(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    galaxy_map: Res<GalaxyMap>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    let Ok(big_space_entity) = q_big_space.get_single() else { return; };

    let detail_radius = 1;

    // Iterate over relevant sectors (Current + neighbor)
    let center_sector = SectorIndex::from_cell(*camera_cell);
    // Only check sectors we know exist (manage_galaxy_sectors ensures they do)
    
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let sector_idx = SectorIndex {
                    x: center_sector.x + x,
                    y: center_sector.y + y,
                    z: center_sector.z + z,
                };
                
                if let Some(stars) = galaxy_map.sectors.get(&sector_idx) {
                     for (cell, star_data) in stars {
                        // Check distance to camera
                        let dx = (cell.x - camera_cell.x).abs();
                        let dy = (cell.y - camera_cell.y).abs();
                        let dz = (cell.z - camera_cell.z).abs();
                        let dist = dx.max(dy).max(dz);

                        // Only spawn if within view radius (5)
                        if dist > 5 { continue; }

                        let required_lod = if dist <= detail_radius { LodLevel::Full } else { LodLevel::Proxy };

                        if let Some((entity, current_lod)) = tracker.spawned_cells.get(cell) {
                            if *current_lod == required_lod {
                                continue; 
                            }
                            commands.entity(*entity).despawn_recursive();
                        }

                         // Spawn 
                        let new_entity = if required_lod == LodLevel::Full {
                            spawn_star_with_data(
                                &mut commands, 
                                big_space_entity, 
                                *cell, 
                                star_data,
                                &mut meshes, 
                                &mut materials
                            )
                        } else {
                            spawn_proxy_with_data(
                                &mut commands, 
                                big_space_entity, 
                                *cell, 
                                star_data,
                                &mut meshes, 
                                &mut materials
                            )
                        };
                        tracker.spawned_cells.insert(*cell, (new_entity, required_lod));
                     }
                }
            }
        }
    }
}

fn update_lod_scaling(
    q_camera: Query<(&GridCell<i64>, &Transform), With<FloatingOrigin>>,
    mut q_proxies: Query<(&GridCell<i64>, &mut Transform, &Children), (With<DistantProxy>, Without<FloatingOrigin>)>,
    mut q_children: Query<&mut Transform, (With<MeshMaterial3d<StandardMaterial>>, Without<DistantProxy>, Without<FloatingOrigin>)>,
) {
    let Ok((cam_cell, cam_tf)) = q_camera.get_single() else { return; };

    for (proxy_cell, _, children) in q_proxies.iter_mut() {
        // Calculate Distance (Continuous)
        let cell_diff = *proxy_cell - *cam_cell;
        let large_diff = Vec3::new(
            cell_diff.x as f32 * GRID_SIZE, 
            cell_diff.y as f32 * GRID_SIZE,
            cell_diff.z as f32 * GRID_SIZE,
        );
        
        // Exact distance: (CellDiff - CameraLocalPos)
        let dist = (large_diff - cam_tf.translation).length(); 

         // We want scale 0.0 at 5 * GRID_SIZE, and 1.0 at ~1.5 * GRID_SIZE (Scanning boundary)
        let min_dist = 1.5 * GRID_SIZE; 
        let max_dist = 5.0 * GRID_SIZE;
        
        // Inverse lerp: 1.0 at min, 0.0 at max
        let scale = ((max_dist - dist) / (max_dist - min_dist)).clamp(0.0, 1.0);
        
        // Apply scale to the visual child
        for child in children {
            if let Ok(mut child_tf) = q_children.get_mut(*child) {
                 child_tf.scale = Vec3::splat(scale);
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
    let removal_radius = 6; 

    tracker.spawned_cells.retain(|cell, (entity, _lod)| {
        let dx = (cell.x - camera_cell.x).abs();
        let dy = (cell.y - camera_cell.y).abs();
        let dz = (cell.z - camera_cell.z).abs();
        
        let dist = dx.max(dy).max(dz);

        if dist > removal_radius {
            commands.entity(*entity).despawn_recursive();
            false
        } else {
            true
        }
    });
}

// Helper to ensure deterministic parameters for both Proxy and Full System
fn generate_star_params(rng: &mut StdRng) -> (Color, f32) {
    let star_color = Color::srgb(rng.r#gen::<f32>(), rng.r#gen::<f32>(), rng.r#gen::<f32>());
    let star_size = rng.gen_range(50.0..200.0);
    (star_color, star_size)
}

fn spawn_proxy_with_data(
    commands: &mut Commands,
    parent_id: Entity,
    cell: GridCell<i64>,
    data: &StarData,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let root = commands.spawn((
        Transform::default(),
        Visibility::default(),
        cell,
        DistantProxy,
    )).id();

    commands.entity(parent_id).add_child(root);

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Mesh3d(meshes.add(Sphere::new(data.size).mesh().ico(3).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: data.color,
                emissive: LinearRgba::from(data.color) * 15.0, 
                unlit: true,
                ..default()
            })),
            Transform::IDENTITY.with_scale(Vec3::ZERO), 
        ));
    });

    root
}

fn spawn_star_with_data(
    commands: &mut Commands,
    parent_id: Entity,
    cell: GridCell<i64>,
    data: &StarData,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let system_root = commands.spawn((
        Transform::default(),
        Visibility::default(),
        cell, 
    )).id();

    commands.entity(parent_id).add_child(system_root);

    commands.entity(system_root).with_children(|root| {
        // Star Sphere
        root.spawn((
            Mesh3d(meshes.add(Sphere::new(data.size).mesh().ico(4).unwrap())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: data.color,
                emissive: LinearRgba::from(data.color) * 5.0,
                ..default()
            })),
            Mass(1_000_000.0), 
            Radius(data.size), 
            Transform::IDENTITY,
        )).with_children(|star| {
             // Light
             star.spawn(PointLight {
                color: data.color,
                intensity: 10_000_000_000.0,
                range: 2_000_000.0,
                ..default()
            });
        });

        // Planets (Randomized per system instance, not stored in GalaxyMap for now as they are local details)
        let mut hasher = DefaultHasher::new();
        cell.hash(&mut hasher);
        let cell_seed = hasher.finish();
        let mut rng = StdRng::seed_from_u64(cell_seed);
        
        let num_planets = rng.gen_range(0..=5);
        for _i in 0..num_planets {
            let dist = rng.gen_range(500.0..5000.0) + data.size;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let planet_size = rng.gen_range(10.0..40.0);
            let planet_color = Color::srgb(rng.r#gen::<f32>(), rng.r#gen::<f32>(), rng.r#gen::<f32>());

            let x = dist * angle.cos();
            let z = dist * angle.sin();

            root.spawn((
                Mesh3d(meshes.add(Sphere::new(planet_size).mesh().ico(3).unwrap())),
                MeshMaterial3d(materials.add(StandardMaterial::from(planet_color))),
                Mass(10_000.0), 
                Radius(planet_size),
                Transform::from_xyz(x, 0.0, z),
            ));
        }
    });

    system_root
}
