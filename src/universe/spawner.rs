use bevy::prelude::*;
use big_space::{GridCell, ReferenceFrame, FloatingOrigin};
use crate::universe::materials::{StarMaterial, PlanetMaterial};
use crate::universe::{UniverseSeed, Mass, Radius, Star, Planet, StarDetails, PlanetDetails, PlanetType, SectorIndex, SECTOR_SIZE};
use crate::universe::gpu_star_renderer::StarSector; 
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
           .init_resource::<NoiseTextures>()
           .init_resource::<PlanetTextureAtlas>()
           .add_systems(Update, (
               manage_galaxy_sectors, 
               handle_generation_tasks, 
               // Ensure texture tasks are handled before potential despawns to avoid command races
               handle_texture_tasks.before(despawn_distant_systems), 
               sync_universe_view, 
               update_lod_scaling, 
               despawn_distant_systems, 
               rotate_planets
            ));
    }
}

#[derive(Component)]
pub struct TextureBakeTask(Task<Vec<u8>>);

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

#[derive(Resource)]
pub struct NoiseTextures {
    pub crater_map: Handle<Image>,
    pub ridge_map: Handle<Image>,
    pub sediment_map: Handle<Image>,
}

impl FromWorld for NoiseTextures {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        NoiseTextures {
            crater_map: asset_server.load("textures/crater_map.png"),
            ridge_map: asset_server.load("textures/ridge_map.png"),
            sediment_map: asset_server.load("textures/sediment_map.png"),
        }
    }
}

#[derive(Resource)]
pub struct PlanetTextureAtlas {
    pub atlas_handle: Handle<Image>,
    pub available_slots: Vec<usize>,
    pub slot_map: HashMap<Entity, usize>,
    pub grid_size: u32,
    pub slot_size: u32,
    pub atlas_size: u32,
}

impl FromWorld for PlanetTextureAtlas {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let atlas_size = 2048;
        let slot_size = 128;
        let grid_size = atlas_size / slot_size;
        let num_slots = (grid_size * grid_size) as usize;

        // Initialize atlas image (transparent black)
        let pixels = vec![0; (atlas_size * atlas_size * 4) as usize];
        let image = Image::new(
            Extent3d {
                width: atlas_size,
                height: atlas_size,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
        );
        let atlas_handle = images.add(image);

        let mut available_slots = Vec::with_capacity(num_slots);
        for i in (0..num_slots).rev() {
            available_slots.push(i);
        }

        PlanetTextureAtlas {
            atlas_handle,
            available_slots,
            slot_map: HashMap::new(),
            grid_size,
            slot_size,
            atlas_size,
        }
    }
}

#[derive(Resource, Default)]
pub struct SpawnTracker {
    pub spawned_cells: HashMap<GridCell<i64>, Entity>, // Only for Full stars
    pub spawned_sectors: HashMap<SectorIndex, Entity>, // Distant GPU sectors
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

fn handle_texture_tasks(
    mut commands: Commands,
    mut q_tasks: Query<(Entity, &mut TextureBakeTask, &PlanetDetails, &Radius)>,
    mut images: ResMut<Assets<Image>>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    mut atlas: ResMut<PlanetTextureAtlas>,
    noise_textures: Res<NoiseTextures>,
) {
    for (entity, mut task, _p_details, _radius) in q_tasks.iter_mut() {
        if let Some(pixels) = future::block_on(future::poll_once(&mut task.0)) {
            // Task Complete: Allocate Slot
            let slot_index = if let Some(idx) = atlas.available_slots.pop() {
                idx
            } else {
                warn!("Planet Texture Atlas Full! Fallback not implemented yet.");
                continue;
            };
            
            atlas.slot_map.insert(entity, slot_index);

            let grid_size = atlas.grid_size;
            let slot_size = atlas.slot_size;
            let atlas_size = atlas.atlas_size;

            if let Some(atlas_image) = images.get_mut(&atlas.atlas_handle) {
                bake_planet_texture(atlas_image, &pixels, slot_index, grid_size, slot_size, atlas_size);
            }

            // Calculate UV Offset/Scale
            let uv_scale = 1.0 / grid_size as f32;
            let row = (slot_index as u32 / grid_size) as f32;
            let col = (slot_index as u32 % grid_size) as f32;

            let offset = Vec2::new(col * uv_scale, row * uv_scale);

            // Create PlanetMaterial using Atlas
            let material = planet_materials.add(PlanetMaterial {
                base_color: LinearRgba::WHITE, // Not used heavily if using atlas? Or tinted?
                second_color: LinearRgba::BLACK,
                seed: 0.0, // Irrelevant for atlas
                atmosphere_color: LinearRgba::new(0.5, 0.7, 1.0, 1.0), // Keep atmosphere?
                atmosphere_density: 0.2, // Default
                atlas_offset: offset,
                atlas_scale: uv_scale,
                use_atlas: 1,
                atlas_texture: atlas.atlas_handle.clone(),
                crater_map: noise_textures.crater_map.clone(),
                ridge_map: noise_textures.ridge_map.clone(),
                sediment_map: noise_textures.sediment_map.clone(),
            });

            // Safely apply changes only if entity still exists
            commands.queue(move |world: &mut World| {
                if let Ok(mut entity_cmd) = world.get_entity_mut(entity) {
                    entity_cmd.insert(MeshMaterial3d(material));
                    entity_cmd.remove::<TextureBakeTask>();
                }
            });
        }
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
                
                 if let Some((color, size)) = crate::universe::star_common::get_star_data(x, y, z, seed.0) {
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
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>, 
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    common_meshes: Res<CommonMeshes>,
    mut std_materials: ResMut<Assets<StandardMaterial>>, 
    mut star_materials: ResMut<Assets<StarMaterial>>, 
    mut planet_materials: ResMut<Assets<PlanetMaterial>>, 
    db: Res<Database>,
    render_config: Res<RenderConfig>,
    mut images: ResMut<Assets<Image>>,
    seed: Res<UniverseSeed>,
    noise_textures: Res<NoiseTextures>,
    atlas: Res<PlanetTextureAtlas>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    let Ok(big_space_entity) = q_big_space.get_single() else { return; };

    let detail_radius = 1; // 1 Sector radius for Full stars
    let view_radius = 5; // 5 Sectors for GPU stars

    let center_sector = SectorIndex::from_cell(*camera_cell);
    
    // 1. Iterate View Area
    for x in -view_radius..=view_radius {
        for y in -view_radius..=view_radius {
            for z in -view_radius..=view_radius {
                 let sector_idx = SectorIndex {
                    x: center_sector.x + x,
                    y: center_sector.y + y,
                    z: center_sector.z + z,
                };

                let sub_dx = x.abs();
                let sub_dy = y.abs();
                let sub_dz = z.abs();
                let dist_sectors = sub_dx.max(sub_dy).max(sub_dz);

                // LOD Selection
                let want_full = dist_sectors <= detail_radius;
                
                // DISTANT (GPU) Logic
                if !want_full {
                    // We need a GPU Sector
                    if !tracker.spawned_sectors.contains_key(&sector_idx) {
                        // Spawn GPU Sector
                        // Calculate Sector Origin Cell
                        let origin_cell = GridCell::<i64>::new(
                             sector_idx.x * SECTOR_SIZE,
                             sector_idx.y * SECTOR_SIZE,
                             sector_idx.z * SECTOR_SIZE
                        );
                        
                        let entity = commands.spawn((
                            SpatialBundle::default(), // Provides Transform/GlobalTransform/Visibility
                            origin_cell, // BigSpace moves it
                            StarSector {
                                index: sector_idx,
                                seed: seed.0 as u32,
                            },
                        )).id();
                        commands.entity(big_space_entity).add_child(entity);
                        tracker.spawned_sectors.insert(sector_idx, entity);
                    }
                    
                    // Ensure Full stars are despawned for this sector
                    // Iterate cells in this sector?
                    // Optimized: Only check `spawned_cells` if we transitioned.
                    // But brute force check for keys in this sector:
                    // Only if we just transitioned? 
                    // Let's rely on standard despawn logic separately or just check here.
                    // Doing 10x10x10 check is 1000 iter. Too slow?
                    // Better: `despawn_distant_systems` handles removal of full stars out of range.
                    // `despawn_distant_systems` currently checks distance > removal_radius.
                    // If removal_radius matches `detail_radius`, it works.
                } 
                
                // FULL Logic
                else {
                    // We want FULL stars.
                    // 1. Despawn GPU Sector if exists
                    if let Some(entity) = tracker.spawned_sectors.remove(&sector_idx) {
                        commands.entity(entity).despawn_recursive();
                    }

                    // 2. Ensure Full stars spawned (if data exists)
                    if let Some(stars) = galaxy_map.sectors.get(&sector_idx) {
                        for (cell, star_data) in stars {
                            if !tracker.spawned_cells.contains_key(cell) {
                                let entity = spawn_star_with_data(
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
                                    &mut images,
                                    &noise_textures,
                                    &atlas,
                                );
                                tracker.spawned_cells.insert(*cell, entity);
                            }
                        }
                    } 
                }
            }
        }
    }
}


// Removed update_lod_scaling (handled by shader/GPU)
fn update_lod_scaling() {}

fn despawn_distant_systems(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>,
    mut atlas: ResMut<PlanetTextureAtlas>,
    q_children: Query<&Children>,
) {
    let Ok(camera_cell) = q_camera.get_single() else { return; };
    let removal_radius = 2; // Keep closer than previously (previously 6). Now > 1 is Distant/GPU? 
    // If detail_radius is 1, then >1 is handled by GPU.
    // So distinct entities should be removed if > 1.
    // Let's set to 2 to have some margin.
    // BUT we must despawn them if we are swapping to GPU.
    // Fix loop: full_radius must encompass the area spawned by sync_universe_view.
    // detail_radius is 1 sector. 1 sector = 10 units. 
    // Max distance to a neighbor sector cell: ~20 units.
    // Safe margin: 25.
    let full_radius = 25;
    let gpu_radius = 60; // 6 sectors * 10

    // Handle Full Cells
    tracker.spawned_cells.retain(|cell, entity| {
        let dx = (cell.x - camera_cell.x).abs();
        let dy = (cell.y - camera_cell.y).abs();
        let dz = (cell.z - camera_cell.z).abs();
        let dist = dx.max(dy).max(dz);

        if dist > full_radius {
            // Check for children (planets) that might have atlas slots
            if let Ok(children) = q_children.get(*entity) {
                for child in children.iter() {
                    if let Some(slot) = atlas.slot_map.remove(child) {
                        atlas.available_slots.push(slot);
                    }
                }
            }

            commands.entity(*entity).despawn_recursive();
            false
        } else {
            true
        }
    });

    // Handle GPU Sectors
    let center_sector = SectorIndex::from_cell(*camera_cell);
    tracker.spawned_sectors.retain(|sector_idx, entity| {
        let dx = (sector_idx.x - center_sector.x).abs();
        let dy = (sector_idx.y - center_sector.y).abs();
        let dz = (sector_idx.z - center_sector.z).abs();
        let dist = dx.max(dy).max(dz);

        if dist > gpu_radius {
            commands.entity(*entity).despawn_recursive();
            false
        } else {
            true
        }
    });
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
    noise_textures: &Res<NoiseTextures>,
    atlas: &Res<PlanetTextureAtlas>,
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
        
        let num_planets = rng.gen_range(1..=4);
        info!("SYSTEM {:?}: Spawning {} planets.", cell, num_planets); 
        for _i in 0..num_planets {
            let dist = rng.gen_range(5000.0..50000.0) + data.size;
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let planet_size = rng.gen_range(5.0..15.0);
            
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
                crate::universe::Orbit {
                    radius: dist,
                    speed: rng.gen_range(0.1..0.5) * (500.0 / dist), // Slower orbits
                    angle,
                },
                PlanetDetails(p_type),
                Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(planet_size)),
            ));

            if render_config.mode == RenderMode::Baked {
                 // Async Bake
                 let p_type_clone = p_type.clone();
                 let thread_pool = AsyncComputeTaskPool::get();
                 
                 let task = thread_pool.spawn(async move {
                     generate_planet_pixels(&p_type_clone, planet_seed)
                 });
                 
                 // Placeholder Material (while loading)
                 planet_entity.insert((
                    MeshMaterial3d(std_materials.add(StandardMaterial {
                        base_color: Color::WHITE, 
                        perceptual_roughness: 1.0,
                         ..default()
                    })),
                    TextureBakeTask(task)
                 ));

            } else {
                 // Procedural
                 let (atmos_col, atmos_density) = p_type.get_atmosphere_color();
                 planet_entity.insert(MeshMaterial3d(planet_materials.add(PlanetMaterial {
                     base_color: col1,
                     second_color: col2,
                     seed: planet_seed,
                     atmosphere_color: atmos_col,
                     atmosphere_density: atmos_density,
                     crater_map: noise_textures.crater_map.clone(),
                     ridge_map: noise_textures.ridge_map.clone(),
                     sediment_map: noise_textures.sediment_map.clone(),
                     atlas_offset: Vec2::ZERO,
                     atlas_scale: 1.0,
                     use_atlas: 0,
                     atlas_texture: atlas.atlas_handle.clone(), // Bind it even if not used
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

// 1. Off-thread pixel generation
fn generate_planet_pixels(
    p_type: &PlanetType,
    seed: f32,
) -> Vec<u8> {
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
    pixels
}

// 2. Main-thread atlas baking
fn bake_planet_texture(
    atlas: &mut Image,
    pixels: &[u8],
    slot_index: usize,
    grid_size: u32,
    slot_size: u32,
    atlas_size: u32,
) {
    let row = slot_index as u32 / grid_size;
    let col = slot_index as u32 % grid_size;

    let start_x = col * slot_size;
    let start_y = row * slot_size;

    // Copy pixels row by row
    let bytes_per_pixel = 4;
    let atlas_stride = (atlas_size * bytes_per_pixel) as usize;
    let slot_stride = (slot_size * bytes_per_pixel) as usize;

    for y in 0..slot_size {
        let atlas_y = start_y + y;
        let atlas_offset = (atlas_y as usize * atlas_stride) + (start_x as usize * bytes_per_pixel as usize);

        let slot_offset = (y as usize * slot_stride);

        if atlas_offset + slot_stride <= atlas.data.len() && slot_offset + slot_stride <= pixels.len() {
            atlas.data[atlas_offset..atlas_offset + slot_stride]
                .copy_from_slice(&pixels[slot_offset..slot_offset + slot_stride]);
        }
    }
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

fn rotate_planets(
    mut q_planets: Query<(&mut Transform, &mut crate::universe::Orbit)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut orbit) in q_planets.iter_mut() {
        orbit.angle += orbit.speed * dt;
        let x = orbit.radius * orbit.angle.cos();
        let z = orbit.radius * orbit.angle.sin();
        transform.translation.x = x;
        transform.translation.z = z;
    }
}
