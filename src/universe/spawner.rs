use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrame, FloatingOrigin};
use crate::universe::materials::{StarMaterial, PlanetMaterial};
use crate::universe::{UniverseSeed, Mass, Radius, Star, Planet, StarDetails, PlanetDetails, PlanetType, SectorIndex, SECTOR_SIZE};
use crate::persistence::Database;
use crate::universe::RenderConfig;
use crate::universe::RenderMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::universe::physics::GRID_SIZE;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

pub struct StarSystemSpawnerPlugin;

impl Plugin for StarSystemSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTracker>()
           .init_resource::<GalaxyMap>()
           .init_resource::<CommonMeshes>()
           .init_resource::<SectorTaskTracker>()
           .add_systems(Update, (manage_galaxy_sectors, handle_generation_tasks, sync_universe_view, update_lod_scaling, despawn_distant_systems));
    }
}

#[derive(Resource)]
pub struct CommonMeshes {
    pub unit_sphere_low: Handle<Mesh>,
    pub unit_sphere_high: Handle<Mesh>,
}

impl FromWorld for CommonMeshes {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        CommonMeshes {
            unit_sphere_low: meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap()),
            unit_sphere_high: meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LodLevel {
    Proxy,
    Full,
}

#[derive(Resource, Default)]
pub struct SpawnTracker {
    pub spawned_cells: HashMap<GridCell<i64>, (Entity, LodLevel)>,
}

// StarData moved to mod.rs

// 10x10x10 GridCells per Sector
// SECTOR_SIZE moved to mod.rs 

// PlanetType moved to mod.rs 

// SectorIndex moved to mod.rs

#[derive(Resource, Default)]
struct GalaxyMap {
    // Map Sector -> List of Stars in that sector
    sectors: HashMap<SectorIndex, Vec<(GridCell<i64>, StarDetails)>>,
}

#[derive(Resource, Default)]
struct SectorTaskTracker {
    tasks: HashMap<SectorIndex, Task<(SectorIndex, Vec<(GridCell<i64>, StarDetails)>)>>,
}

#[derive(Component)]
struct DistantProxy;

fn manage_galaxy_sectors(
    galaxy_map: Res<GalaxyMap>,
    mut task_tracker: ResMut<SectorTaskTracker>, // Track tasks
    seed: Res<UniverseSeed>,
    db: Res<Database>,
    q_camera: Query<&GridCell<i64>, (With<FloatingOrigin>, Changed<GridCell<i64>>)>, // Event Driven
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    
    // Check current sector + Neighbors
    let center_sector = SectorIndex::from_cell(*camera_cell);
    let view_dist = 1; // 3x3x3 sectors
    let seed_val = *seed; // Copy seed

    // Clone DB for async task usage
    let db_clone = db.clone();

    for x in -view_dist..=view_dist {
        for y in -view_dist..=view_dist {
            for z in -view_dist..=view_dist {
                let sector_idx = SectorIndex {
                    x: center_sector.x + x,
                    y: center_sector.y + y,
                    z: center_sector.z + z,
                };

                // If not in map AND not currently generating
                if !galaxy_map.sectors.contains_key(&sector_idx) && !task_tracker.tasks.contains_key(&sector_idx) {
                    let thread_pool = AsyncComputeTaskPool::get();
                    let db_for_task = db_clone.clone(); 
                    
                    let task = thread_pool.spawn(async move {
                        let data = generate_sector_data(sector_idx, seed_val, &db_for_task);
                        (sector_idx, data)
                    });
                    task_tracker.tasks.insert(sector_idx, task);
                }
            }
        }
    }
}

fn handle_generation_tasks(
    mut galaxy_map: ResMut<GalaxyMap>,
    mut task_tracker: ResMut<SectorTaskTracker>,
) {
    let mut completed = Vec::new();
    
    for (_sector, task) in task_tracker.tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            completed.push(result);
        }
    }

    for (sector, stars) in completed {
        galaxy_map.sectors.insert(sector, stars);
        task_tracker.tasks.remove(&sector);
        // info!("Generatated Sector (Async) {:?}, stars: {}", sector, galaxy_map.sectors.get(&sector).unwrap().len());
    }
}

fn generate_sector_data(
    sector: SectorIndex, 
    seed: UniverseSeed,
    db: &Database
) -> Vec<(GridCell<i64>, StarDetails)> {
    // 1. Check Database
    match db.get_sector_data(sector) {
        Ok(Some(data)) => {
            info!("PERSISTENCE: Loaded Sector {:?} with {} stars.", sector, data.len());
            return data;
        },
        Ok(None) => {
             info!("PERSISTENCE: Sector {:?} not found. Generating...", sector);
        },
        Err(e) => {
            error!("PERSISTENCE: Critical Error reading Sector {:?}: {}. Aborting generation to protect data.", sector, e);
            return Vec::new(); 
        }
    }

    // 2. Generate
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
                let density_chance = if is_origin { 1.0 } else { 0.005 }; // 0.5% chance 

                if rng.gen_bool(density_chance) {
                    let (color, size) = generate_star_params(&mut rng);
                    stars.push((cell, StarDetails { color, size }));
                }
            }
        }
    }



    // 3. Save to Database
    if let Err(e) = db.save_sector_data(sector, &stars) {
        error!("Failed to save sector data: {}", e);
    }

    stars
}


fn sync_universe_view(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    galaxy_map: Res<GalaxyMap>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>, // Run every frame to handle async loading
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    common_meshes: Res<CommonMeshes>,
    mut std_materials: ResMut<Assets<StandardMaterial>>, 
    mut star_materials: ResMut<Assets<StarMaterial>>, 
    mut planet_materials: ResMut<Assets<PlanetMaterial>>, 
    db: Res<Database>,
    render_config: Res<RenderConfig>,
    mut images: ResMut<Assets<Image>>,
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
                                &common_meshes,
                                &mut star_materials,
                                &mut planet_materials,
                                &mut std_materials,
                                &db,
                                &render_config,
                                &mut images
                            )
                        } else {
                            spawn_proxy_with_data(
                                &mut commands, 
                                big_space_entity, 
                                *cell, 
                                star_data,
                                &common_meshes,
                                &mut std_materials
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
    time: Res<Time>,
    mut timer: Local<f32>,
    q_camera: Query<(&GridCell<i64>, &Transform), With<FloatingOrigin>>,
    mut q_proxies: Query<(&GridCell<i64>, &mut Transform, &Children), (With<DistantProxy>, Without<FloatingOrigin>)>,
    mut q_children: Query<(&mut Transform, Option<&Radius>), (With<MeshMaterial3d<StandardMaterial>>, Without<DistantProxy>, Without<FloatingOrigin>)>,
) {
    *timer += time.delta_secs();
    if *timer < 0.05 { return; } // 20Hz
    *timer = 0.0;

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
            if let Ok((mut child_tf, radius)) = q_children.get_mut(*child) {
                 let base_scale = radius.map(|r| r.0).unwrap_or(1.0);
                 child_tf.scale = Vec3::splat(scale * base_scale);
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
    data: &StarDetails,
    common_meshes: &Res<CommonMeshes>,
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
            Mesh3d(common_meshes.unit_sphere_low.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: data.color,
                emissive: LinearRgba::from(data.color) * 15.0, 
                unlit: true,
                ..default()
            })),
            Radius(data.size),
            Transform::IDENTITY.with_scale(Vec3::ZERO), 
        ));
    });

    root
}

#[derive(Component)]
pub struct SystemLabel;

fn spawn_star_with_data(
    commands: &mut Commands,
    parent_id: Entity,
    cell: GridCell<i64>,
    data: &StarDetails,
    common_meshes: &Res<CommonMeshes>,
    star_materials: &mut ResMut<Assets<StarMaterial>>,
    planet_materials: &mut ResMut<Assets<PlanetMaterial>>,
    std_materials: &mut ResMut<Assets<StandardMaterial>>,
    db: &Database,
    render_config: &Res<RenderConfig>,
    images: &mut ResMut<Assets<Image>>,
) -> Entity {
    // Determine Names
    let default_name = format!("S {},{},{}", cell.x, cell.y, cell.z);
    let mut star_name = default_name.clone();
    let mut planet_base_name = default_name.clone();
    let mut is_custom = false;

    if let Ok(Some(disc)) = db.get_discovery(cell) {
        if disc.name != default_name {
            star_name = disc.name.clone();
            planet_base_name = disc.name.clone();
            is_custom = true;
        }
    }

    let system_root = commands.spawn((
        Transform::default(),
        Visibility::default(),
        cell, 
    )).id();

    // Debug: Log spawn
    info!("SPAWNING STAR @ {:?}: Size: {}, Color: {:?}", cell, data.size, data.color);

    commands.entity(parent_id).add_child(system_root);

    commands.entity(system_root).with_children(|root| {
        // Star Sphere
        root.spawn((
            Mesh3d(common_meshes.unit_sphere_high.clone()),
            MeshMaterial3d(star_materials.add(StarMaterial {
                color: LinearRgba::from(data.color),
                seed: cell.x as f32 * 0.123 + cell.y as f32 * 0.456, // Simple Seed
            })),
            Mass(1_000_000.0), 
            Radius(data.size), 
            Star,
            StarDetails { color: data.color, size: data.size },
            Transform::IDENTITY.with_scale(Vec3::splat(data.size)),
        ))
        .observe(move |_trigger: Trigger<Pointer<Click>>, mut events: EventWriter<crate::universe::StarClicked>| {
            events.send(crate::universe::StarClicked { entity: _trigger.entity(), cell });
        })
        .with_children(|star| {
             // Light
             star.spawn(PointLight {
                color: data.color,
                intensity: 10_000_000_000.0,
                range: 2_000_000.0,
                ..default()
            });

             // System Label (Billboard)
             star.spawn((
                Text2d::new(star_name),
                TextFont { font_size: 100.0, ..default() }, // Large in-world font
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(JustifyText::Center),
                Transform::from_xyz(0.0, data.size * 2.5, 0.0) // Increased offset: 2.5x radius
                    .with_scale(Vec3::splat(1.0)), // Ensure scale 
                SystemLabel,
             ));
        });

        // Planets (Randomized per system instance, not stored in GalaxyMap for now as they are local details)
        let mut hasher = DefaultHasher::new();
        cell.hash(&mut hasher);
        let cell_seed = hasher.finish();
        let mut rng = StdRng::seed_from_u64(cell_seed);
        
        let num_planets = rng.gen_range(1..=9);
        info!("SYSTEM {:?}: Spawning {} planets.", cell, num_planets); 
        for _i in 0..num_planets {
            let dist = rng.gen_range(5000.0..50000.0) + data.size;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let planet_size = rng.gen_range(10.0..40.0);
            
            let planet_seed = dist * 0.123 + angle;
            let p_type = PlanetType::from_seed(planet_seed);
            let (col1, col2) = p_type.get_palette();

            let x = dist * angle.cos();
            let z = dist * angle.sin();

            let mut planet_entity = root.spawn((
                Mesh3d(common_meshes.unit_sphere_low.clone()),
                Mass(10_000.0), 
                Radius(planet_size),
                Planet,
                PlanetDetails(p_type),
                Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(planet_size)),
            ));

            if render_config.mode == RenderMode::Baked {
                 // Bake Texture
                 let tex = bake_planet_texture(images, &p_type, planet_seed);
                 planet_entity.insert(MeshMaterial3d(std_materials.add(StandardMaterial {
                     base_color_texture: Some(tex),
                     base_color: Color::WHITE,
                     perceptual_roughness: 0.8,
                     ..default()
                 })));
            } else {
                 // Procedural
                 planet_entity.insert(MeshMaterial3d(planet_materials.add(PlanetMaterial {
                     base_color: col1,
                     second_color: col2,
                     seed: planet_seed,
                })));
            }
            
            planet_entity
            .observe(move |_trigger: Trigger<Pointer<Click>>, mut events: EventWriter<crate::universe::StarClicked>| {
                events.send(crate::universe::StarClicked { entity: _trigger.entity(), cell });
            })
            .with_children(|planet| {
                 // Planet Label
                 let p_name = if is_custom {
                     format!("{} (Planet)", planet_base_name)
                 } else {
                     format!("P {},{},{}", cell.x, cell.y, cell.z)
                 };

                 planet.spawn((
                    Text2d::new(p_name),
                    TextFont { font_size: 80.0, ..default() }, // Slightly smaller than star
                    TextColor(Color::srgb(0.8, 0.8, 1.0)), // Blueish
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_xyz(0.0, planet_size * 3.0, 0.0) // 3x radius
                        .with_scale(Vec3::splat(1.0)),
                    SystemLabel, // Will sync with DB name
                 ));
            });
        }
    });

    system_root
}

fn bake_planet_texture(
    images: &mut Assets<Image>,
    p_type: &PlanetType,
    seed: f32,
) -> Handle<Image> {
    let size = 128; // Small texture for performance/style
    let mut pixels = Vec::with_capacity(size * size * 4);
    let (c1, c2) = p_type.get_palette(); // LinearRgba
    
    // Convert LinearRgba to Vec3 for mixing
    let col1 = Vec3::new(c1.red, c1.green, c1.blue);
    let col2 = Vec3::new(c2.red, c2.green, c2.blue);

    for y in 0..size {
        for x in 0..size {
            // Spherical Mapping approximation (just noise on 2D plane for now, usually needs equirectangular)
            // For simple "Planet Style", standard noise is okay if UVs are standard sphere.
            // Sphere mesh UVs are equirectangular.
            
            let u = x as f32 / size as f32;
            let v = y as f32 / size as f32;
            
            // Noise scale
            let scale = 10.0;
            let n = simple_noise(u * scale + seed, v * scale + seed);
            
            // Mix colors
            let final_col = col1.lerp(col2, n);
            
            pixels.extend_from_slice(&[
                (final_col.x * 255.0) as u8,
                (final_col.y * 255.0) as u8,
                (final_col.z * 255.0) as u8,
                255
            ]);
        }
    }

    let image = Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD, 
    );
    images.add(image)
}

// Re-implement simple noise locally to avoid pub sharing issues
fn simple_noise(x: f32, y: f32) -> f32 {
    let i = x.floor();
    let j = y.floor();
    let f_x = x.fract();
    let f_y = y.fract();
    
    // Hash
    let rand = |dX: f32, dY: f32| -> f32 {
        ((i + dX) * 12.9898 + (j + dY) * 78.233).sin().fract().abs()
    };
    
    let a = rand(0.0, 0.0);
    let b = rand(1.0, 0.0);
    let c = rand(0.0, 1.0);
    let d = rand(1.0, 1.0);
    
    // Mix
    let u_x = f_x * f_x * (3.0 - 2.0 * f_x);
    let u_y = f_y * f_y * (3.0 - 2.0 * f_y);
    
    let h1 = a + (b - a) * u_x;
    let h2 = c + (d - c) * u_x;
    
    h1 + (h2 - h1) * u_y
}
