use crate::persistence::Database;
use crate::universe::RenderConfig;
use crate::universe::RenderMode;
use crate::universe::gpu_star_renderer::StarSector;
use crate::universe::materials::{PlanetMaterial, StarMaterial};
use crate::universe::{
    Mass, Planet, PlanetDetails, PlanetType, Radius, SECTOR_SIZE, SectorIndex, Star, StarDetails,
    UniverseSeed, StarPresets, StarVisuals, PlanetPresets, PlanetVisuals,
};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use big_space::{FloatingOrigin, GridCell, ReferenceFrame};

// use crate::universe::physics::GRID_SIZE; // Unused
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bevy::ecs::system::SystemParam;

pub struct StarSystemSpawnerPlugin;

impl Plugin for StarSystemSpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpawnTracker>()
            .init_resource::<GalaxyMap>()
            .init_resource::<CommonMeshes>()
            .init_resource::<SectorTaskTracker>()
            .init_resource::<NoiseTextures>()
            .init_resource::<PlanetTextureAtlas>()
            .init_resource::<StarPresets>()
            .init_resource::<PlanetPresets>()
            .add_systems(
                Update,
                (
                    manage_galaxy_sectors,
                    handle_generation_tasks,
                    sync_star_presets, // Live Tuning System
                    sync_planet_presets, // Live Tuning System (Planets)
                    sync_universe_view,
                    update_lod_scaling,
                    despawn_distant_systems,
                    rotate_planets,
                ),
            );
    }
}

/// System to reload star_presets.json and update all materials live
fn sync_star_presets(
    mut presets: ResMut<StarPresets>,
    mut star_materials: ResMut<Assets<StarMaterial>>,
    time: Res<Time>,
    mut last_sync: Local<f32>,
    mut q_stars: Query<(&mut Transform, &StarDetails)>,
) {
    if time.elapsed_secs() - *last_sync < 1.0 { // Throttle disc read
        return;
    }
    *last_sync = time.elapsed_secs();

    let config_path = "assets/config/star_presets.json";
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(new_map) = serde_json::from_str::<HashMap<String, StarVisuals>>(&content) {
            // Check if anything actually changed
            if new_map != presets.map {
                presets.map = new_map;
                info!("STAR CONFIG: Reloaded presets from JSON.");
                
                // 1. Update the Material Asset cache
                for (_, material) in star_materials.iter_mut() {
                    let type_key = format!("{:?}", material.star_type);
                    if let Some(v) = presets.map.get(&type_key) {
                        material.convection_scale = v.convection_scale;
                        material.convection_speed = v.convection_speed;
                        material.warp_intensity = v.warp_intensity;
                        material.plasma_speed = v.plasma_speed;
                        material.hot_spot_intensity = v.hot_spot_intensity;
                        material.corona_intensity = v.corona_intensity;
                        material.rim_power = v.rim_power;
                        material.intensity = v.intensity;
                        material.flare_scale = v.flare_scale;
                        material.flare_speed = v.flare_speed;
                        material.flare_intensity = v.flare_intensity;
                        material.flare_height = v.flare_height;
                        material.flare_mode = v.flare_mode;
                        material.flare_enabled = if v.flare_enabled { 1 } else { 0 };
                    }
                }

                // 2. Update physical Mesh Scales of all active stars
                for (mut transform, details) in q_stars.iter_mut() {
                    let type_key = format!("{:?}", details.star_type);
                    if let Some(v) = presets.map.get(&type_key) {
                        // The mesh must be large enough to contain the flares: size * (1.1 + flare_height)
                        transform.scale = Vec3::splat(details.size * (1.1 + v.flare_height));
                    }
                }
            }
        }
    }
}

/// System to reload planet_presets.json and update all materials live
fn sync_planet_presets(
    mut presets: ResMut<PlanetPresets>,
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    time: Res<Time>,
    mut last_sync: Local<f32>,
) {
    if time.elapsed_secs() - *last_sync < 1.0 { // Throttle disc read
        return;
    }
    *last_sync = time.elapsed_secs();

    let config_path = "assets/config/planet_presets.json";
    if let Ok(content) = std::fs::read_to_string(config_path) {
        if let Ok(new_map) = serde_json::from_str::<HashMap<String, PlanetVisuals>>(&content) {
            // Check if anything actually changed
            if new_map != presets.map {
                presets.map = new_map;
                info!("PLANET CONFIG: Reloaded presets from JSON.");
                
                // Update the Material Asset cache
                for (_, material) in planet_materials.iter_mut() {
                    let type_key = format!("{:?}", material.planet_type);
                    if let Some(v) = presets.map.get(&type_key) {
                        material.rim_intensity = v.rim_intensity;
                        material.rim_power = v.rim_power;
                        material.haze_intensity = v.haze_intensity;
                        material.cloud_threshold = v.cloud_threshold;
                        material.cloud_opacity = v.cloud_opacity;
                        material.cloud_speed = v.cloud_speed;
                        material.specular_intensity = v.specular_intensity;
                        material.bio_intensity = v.bio_intensity;
                    }
                }
            }
        }
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
            unit_sphere_low: meshes.add(Sphere::new(1.0).mesh().ico(5).unwrap()),
            unit_sphere_high: meshes.add(Sphere::new(1.0).mesh().ico(7).unwrap()),
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
            bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD
                | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
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
    pub spawned_cells: HashMap<GridCell<i64>, Vec<Entity>>, // Multiple entities per cell for predefined systems
    pub spawned_sectors: HashMap<SectorIndex, Entity>,      // Distant GPU sectors
}

// StarData moved to mod.rs

// 10x10x10 GridCells per Sector
// SECTOR_SIZE moved to mod.rs

// PlanetType moved to mod.rs

// SectorIndex moved to mod.rs

#[derive(Resource, Default)]
pub(crate) struct GalaxyMap {
    // Map Sector -> List of Stars in that sector
    sectors: HashMap<SectorIndex, Vec<(GridCell<i64>, StarDetails)>>,
}

#[derive(Resource, Default)]
pub(crate) struct SectorTaskTracker {
    tasks: HashMap<SectorIndex, Task<(SectorIndex, Vec<(GridCell<i64>, StarDetails)>)>>,
}

#[derive(SystemParam)]
struct SpawnerAssets<'w> {
    common_meshes: Res<'w, CommonMeshes>,
    std_materials: ResMut<'w, Assets<StandardMaterial>>,
    star_materials: ResMut<'w, Assets<StarMaterial>>,
    planet_materials: ResMut<'w, Assets<PlanetMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    noise_textures: Res<'w, NoiseTextures>,
    atlas: Res<'w, PlanetTextureAtlas>,
}

fn manage_galaxy_sectors(
    galaxy_map: Res<GalaxyMap>,
    mut task_tracker: ResMut<SectorTaskTracker>, // Track tasks
    seed: Res<UniverseSeed>,
    db: Res<Database>,
    config: Res<crate::universe::UniverseConfig>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>, // Run every frame (tracker/db logic handles optimization)
) {
    let Ok(camera_cell) = q_camera.get_single() else {
        return;
    };

    // Check current sector + Neighbors
    let center_sector = SectorIndex::from_cell(*camera_cell);
    let view_dist = 1; // 3x3x3 sectors
    let seed_val = *seed; // Copy seed
    let star_override = config.star_override;
    let planet_override = config.planet_override;

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
                if !galaxy_map.sectors.contains_key(&sector_idx)
                    && !task_tracker.tasks.contains_key(&sector_idx)
                {
                    let thread_pool = AsyncComputeTaskPool::get();
                    let db_for_task = db_clone.clone();

                    let task = thread_pool.spawn(async move {
                        let data = generate_sector_data(sector_idx, seed_val, &db_for_task, star_override, planet_override);
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

// Legacy texture task handler removed.

fn generate_sector_data(
    sector: SectorIndex,
    seed: UniverseSeed,
    db: &Database,
    star_override: Option<crate::universe::StarType>,
    planet_override: Option<crate::universe::PlanetType>,
) -> Vec<(GridCell<i64>, StarDetails)> {
    let is_origin_sector = sector.x == 0 && sector.y == 0 && sector.z == 0;
    let has_override = star_override.is_some() || planet_override.is_some();

    // 1. Check Database (Skip if override active at origin)
    if is_origin_sector && has_override {
        info!("PERSISTENCE: Override active ({:?}/{:?}). Bypassing DB for origin sector.", star_override, planet_override);
    } else {
        match db.get_sector_data(sector) {
            Ok(Some(data)) => {
                info!(
                    "PERSISTENCE: Loaded Sector {:?} with {} stars.",
                    sector,
                    data.len()
                );
                return data;
            }
            Ok(None) => {
                info!("PERSISTENCE: Sector {:?} not found. Generating...", sector);
            }
            Err(e) => {
                error!(
                    "PERSISTENCE: Critical Error reading Sector {:?}: {}. Aborting generation to protect data.",
                    sector, e
                );
                return Vec::new();
            }
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

                if let Some((mut star_type, color, size)) =
                    crate::universe::star_common::get_star_data(x, y, z, seed.0)
                {
                    let mut planets = None;
                    
                    // Apply override if at origin
                    if x == 0 && y == 0 && z == 0 {
                        if let Some(over) = star_override {
                            star_type = over;
                        }
                        
                        // If planet_override set, ensure we have a planet to override in spawn_star_with_data
                        if let Some(p_over) = planet_override {
                             planets = Some(vec![crate::universe::DetailedPlanet {
                                name: "Override Core".to_string(),
                                planet_type: p_over,
                                distance: 0.0, // Or fallback distance
                                size: 50.0,
                                color: Color::WHITE,
                                second_color: None,
                                atmosphere_color: None,
                                atmosphere_density: None,
                                orbit_speed: 0.0,
                            }]);
                        }
                    }

                    stars.push((
                        cell,
                        StarDetails {
                            star_type,
                            color,
                            size,
                            planets,
                        },
                    ));
                }
            }
        }
    }

    // 3. Save to Database (Skip if override applied to prevent corruption)
    if is_origin_sector && has_override {
        info!("PERSISTENCE: Skip saving origin sector override to DB.");
    } else if let Err(e) = db.save_sector_data(sector, &stars) {
        error!("Failed to save sector data: {}", e);
    }

    stars
}

fn sync_universe_view(
    mut commands: Commands,
    mut tracker: ResMut<SpawnTracker>,
    galaxy_map: Res<GalaxyMap>,
    q_camera: Query<&GridCell<i64>, With<FloatingOrigin>>, // Run every frame
    q_big_space: Query<Entity, With<ReferenceFrame<i64>>>,
    mut assets: SpawnerAssets,
    db: Res<Database>,
    render_config: Res<RenderConfig>,
    seed: Res<UniverseSeed>,
    config: Res<crate::universe::UniverseConfig>,
    presets: Res<StarPresets>,
    planet_presets: Res<PlanetPresets>,
) {
    let Ok(camera_cell) = q_camera.get_single() else {
        return;
    };
    let Ok(big_space_entity) = q_big_space.get_single() else {
        return;
    };

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
                            sector_idx.z * SECTOR_SIZE,
                        );

                        let entity = commands
                            .spawn((
                                Transform::default(),
                                Visibility::default(), // Provides Transform/GlobalTransform/Visibility
                                origin_cell,           // BigSpace moves it
                                StarSector {
                                    index: sector_idx,
                                    seed: seed.0 as u32,
                                },
                            ))
                            .id();
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
                    // We want FULL stars.
                    // Only switch if data is ready
                    if let Some(stars) = galaxy_map.sectors.get(&sector_idx) {
                        // 1. Spawn Full Stars
                        for (cell, star_data) in stars {
                            if !tracker.spawned_cells.contains_key(cell) {
                                let entities = spawn_star_with_data(
                                    &mut commands,
                                    big_space_entity,
                                    *cell,
                                    star_data,
                                    &assets.common_meshes,
                                    &mut assets.star_materials,
                                    &mut assets.planet_materials,
                                    &mut assets.std_materials,
                                    &db,
                                    &render_config,
                                    &mut assets.images,
                                    &assets.noise_textures,
                                    &assets.atlas,
                                    &seed,
                                    &config,
                                    &presets,
                                    &planet_presets,
                                );
                                tracker.spawned_cells.insert(*cell, entities);
                            }
                        }

                        // 2. Despawn GPU Sector (Now that full stars are spawning)
                        if let Some(entity) = tracker.spawned_sectors.remove(&sector_idx) {
                            commands.entity(entity).despawn_recursive();
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
    // Asset Cleanup Resources
    mut planet_materials: ResMut<Assets<PlanetMaterial>>,
    mut star_materials: ResMut<Assets<StarMaterial>>,
    q_planet_mat: Query<&MeshMaterial3d<PlanetMaterial>>,
    q_star_mat: Query<&MeshMaterial3d<StarMaterial>>,
) {
    let Ok(camera_cell) = q_camera.get_single() else {
        return;
    };

    // Tuning: Reduced radii to be closer to view distance to prevent zombie entities
    // view_radius is 1 sector (approx 10-15 units diag).
    // Set full_radius to 20 to allow small buffer.
    let full_radius = 20;

    // gpu view_radius is 5 sectors.
    // Set gpu_radius to 8 sectors to clear things behind us reasonably fast.
    let gpu_radius = 8;

    // Handle Full Cells
    tracker.spawned_cells.retain(|cell, entities| {
        let dx = (cell.x - camera_cell.x).abs();
        let dy = (cell.y - camera_cell.y).abs();
        let dz = (cell.z - camera_cell.z).abs();
        let dist = dx.max(dy).max(dz);

        if dist > full_radius {
            // Check for children (planets/stars) to clean up resources
            for entity in entities {
                if let Ok(children) = q_children.get(*entity) {
                    for child in children.iter() {
                        // 1. Release Atlas Slot
                        if let Some(slot) = atlas.slot_map.remove(child) {
                            atlas.available_slots.push(slot);
                        }

                        // 2. Cleanup Unique Materials (Asset Leak Fix)
                        if let Ok(mat_handle) = q_planet_mat.get(*child) {
                            planet_materials.remove(&mat_handle.0);
                        }
                        if let Ok(mat_handle) = q_star_mat.get(*child) {
                            star_materials.remove(&mat_handle.0);
                        }
                    }
                }
                // Check root itself (rarely has material, but good practice if structure changes)
                if let Ok(mat_handle) = q_star_mat.get(*entity) {
                    star_materials.remove(&mat_handle.0);
                }

                commands.entity(*entity).despawn_recursive();
            }
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
    _images: &mut ResMut<Assets<Image>>,
    noise_textures: &Res<NoiseTextures>,
    atlas: &Res<PlanetTextureAtlas>,
    _seed: &UniverseSeed,
    config: &crate::universe::UniverseConfig,
    presets: &Res<StarPresets>,
    planet_presets: &Res<PlanetPresets>,
) -> Vec<Entity> {
    let mut spawned_entities = Vec::new();

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

    // Special Case: Our Solar System
    let is_sun = cell.x == 0 && cell.y == 0 && cell.z == 0 && config.scenario_name == "our_system";
    if is_sun {
        star_name = "The Sun".to_string();
    }

    let system_root = commands
        .spawn((Transform::default(), Visibility::default(), cell))
        .id();
    spawned_entities.push(system_root);

    commands.entity(parent_id).add_child(system_root);

    commands.entity(system_root).with_children(|root| {
        // Special handling for exotic star types
        match data.star_type {
            crate::universe::StarType::BlackHole => {
                // Black Hole: Black sphere with no light emission
                // Note: Full BlackHoleMaterial with raymarching is in separate binary
                root.spawn((
                    Mesh3d(common_meshes.unit_sphere_high.clone()),
                    MeshMaterial3d(std_materials.add(StandardMaterial {
                        base_color: Color::BLACK,
                        emissive: LinearRgba::BLACK,
                        unlit: true,
                        ..default()
                    })),
                    Mass(10_000_000_000.0), // Extreme mass
                    Radius(data.size),
                    Star,
                    StarDetails {
                        star_type: data.star_type,
                        color: data.color,
                        size: data.size,
                        planets: None,
                    },
                    Transform::IDENTITY.with_scale(Vec3::splat(data.size)),
                ))
                .observe(
                    move |_trigger: Trigger<Pointer<Click>>,
                          mut events: EventWriter<crate::universe::StarClicked>| {
                        events.send(crate::universe::StarClicked {
                            entity: _trigger.entity(),
                            cell,
                        });
                    },
                )
                .with_children(|bh| {
                    // No light for Black Hole (it absorbs light)
                    // System Label
                    bh.spawn((
                        Text2d::new(star_name.clone()),
                        TextFont { font_size: 100.0, ..default() },
                        TextColor(Color::srgb(0.5, 0.0, 0.5)), // Purple for ominous feel
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_xyz(0.0, data.size * 2.5, 0.0).with_scale(Vec3::splat(1.0)),
                        SystemLabel,
                    ));
                });
            }
            crate::universe::StarType::NeutronStar => {
                // Neutron Star: Tiny but extremely bright with bloom
                let visuals = presets.map.get(&format!("{:?}", data.star_type))
                    .cloned()
                    .unwrap_or_default();

                root.spawn((
                    Mesh3d(common_meshes.unit_sphere_high.clone()),
                    MeshMaterial3d(star_materials.add({
                        StarMaterial {
                            color: LinearRgba::from(data.color),
                            seed: cell.x as f32 * 0.123 + cell.y as f32 * 0.456,
                            convection_scale: visuals.convection_scale,
                            convection_speed: visuals.convection_speed,
                            warp_intensity: visuals.warp_intensity,
                            plasma_speed: visuals.plasma_speed,
                            hot_spot_intensity: visuals.hot_spot_intensity,
                            corona_intensity: visuals.corona_intensity,
                            rim_power: visuals.rim_power,
                            intensity: visuals.intensity,
                            flare_scale: visuals.flare_scale,
                            flare_speed: visuals.flare_speed,
                            flare_intensity: visuals.flare_intensity,
                            flare_height: visuals.flare_height,
                            flare_mode: visuals.flare_mode,
                            flare_enabled: if visuals.flare_enabled { 1 } else { 0 },
                            star_type: data.star_type,
                        }
                    })),
                    Mass(1_000_000_000.0), // Very dense
                    Radius(data.size),
                    Star,
                    StarDetails {
                        star_type: data.star_type,
                        color: data.color,
                        size: data.size,
                        planets: None,
                    },
                    Transform::IDENTITY.with_scale(Vec3::splat(data.size * (1.1 + visuals.flare_height))),
                ))
                .observe(
                    move |_trigger: Trigger<Pointer<Click>>,
                          mut events: EventWriter<crate::universe::StarClicked>| {
                        events.send(crate::universe::StarClicked {
                            entity: _trigger.entity(),
                            cell,
                        });
                    },
                )
                .with_children(|ns| {
                    // Extreme point light
                    ns.spawn(PointLight {
                        color: data.color,
                        intensity: data.star_type.get_light_intensity(),
                        range: data.star_type.get_light_range(),
                        shadows_enabled: false,
                        ..default()
                    });
                    // System Label
                    ns.spawn((
                        Text2d::new(star_name.clone()),
                        TextFont { font_size: 100.0, ..default() },
                        TextColor(Color::srgb(0.8, 0.9, 1.0)), // Bright blue-white
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_xyz(0.0, data.size * 10.0, 0.0) // Extra offset for tiny star
                            .with_scale(Vec3::splat(1.0)),
                        SystemLabel,
                    ));
                });
            }
            _ => {
                // Standard star rendering for OBAFGKM types
                let visuals = presets.map.get(&format!("{:?}", data.star_type))
                    .cloned()
                    .unwrap_or_default();

                root.spawn((
                    Mesh3d(common_meshes.unit_sphere_high.clone()),
                    MeshMaterial3d(star_materials.add({
                        StarMaterial {
                            color: LinearRgba::from(data.color),
                            seed: cell.x as f32 * 0.123 + cell.y as f32 * 0.456,
                            convection_scale: visuals.convection_scale,
                            convection_speed: visuals.convection_speed,
                            warp_intensity: visuals.warp_intensity,
                            plasma_speed: visuals.plasma_speed,
                            hot_spot_intensity: visuals.hot_spot_intensity,
                            corona_intensity: visuals.corona_intensity,
                            rim_power: visuals.rim_power,
                            intensity: visuals.intensity,
                            flare_scale: visuals.flare_scale,
                            flare_speed: visuals.flare_speed,
                            flare_intensity: visuals.flare_intensity,
                            flare_height: visuals.flare_height,
                            flare_mode: visuals.flare_mode,
                            flare_enabled: if visuals.flare_enabled { 1 } else { 0 },
                            star_type: data.star_type,
                        }
                    })),
                    Mass(1_000_000.0),
                    Radius(data.size),
                    Star,
                    StarDetails {
                        star_type: data.star_type,
                        color: data.color,
                        size: data.size,
                        planets: None,
                    },
                    Transform::IDENTITY.with_scale(Vec3::splat(data.size * (1.1 + visuals.flare_height))),
                ))
                .observe(
                    move |_trigger: Trigger<Pointer<Click>>,
                          mut events: EventWriter<crate::universe::StarClicked>| {
                        events.send(crate::universe::StarClicked {
                            entity: _trigger.entity(),
                            cell,
                        });
                    },
                )
                .with_children(|star| {
                    // Light
                    star.spawn(PointLight {
                        color: data.color,
                        intensity: data.star_type.get_light_intensity(),
                        range: data.star_type.get_light_range(),
                        shadows_enabled: false,
                        ..default()
                    });
                    // System Label (Billboard)
                    star.spawn((
                        Text2d::new(star_name.clone()),
                        TextFont { font_size: 100.0, ..default() },
                        TextColor(Color::WHITE),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_xyz(0.0, data.size * 2.5, 0.0).with_scale(Vec3::splat(1.0)),
                        SystemLabel,
                    ));
                });
            }
        }

        // Asteroid Belt (GPU Instanced)
        // 10,000 asteroids
        let mut rng = rand::thread_rng();
        let asteroid_count = 10_000;
        let mut asteroids = Vec::with_capacity(asteroid_count);
        for _ in 0..asteroid_count {
            let dist: f32 = rng.gen_range(2000.0..15000.0); // Wide belt
            let speed = 50.0 / dist.sqrt() * 10.0; // Kepler-ish
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);

            // Random Color (Ice vs Rock)
            let is_ice = rng.gen_bool(0.3);
            let color = if is_ice {
                [0.8, 0.9, 1.0, 1.0] // Icon
            } else {
                [0.4, 0.35, 0.3, 1.0] // Rock
            };

            asteroids.push(crate::universe::gpu_physics::OrbitalElement {
                radius: dist,
                speed,
                initial_angle: angle,
                eccentricity: 0.0,
                color,
            });
        }

        root.spawn((
            crate::universe::gpu_physics::OrbitBatch {
                elements: asteroids,
            },
            Transform::IDENTITY,
            Visibility::default(),
        ));
    });

    // Planets (Separate from system_root hierarchy for GridCell independence)

    // Planets
    if let Some(planets) = &data.planets {
        commands.entity(system_root).with_children(|root| {
            for (p_idx, planet_data) in planets.iter().enumerate() {
                let mut p_type = planet_data.planet_type;
                
                // Apply CLI Override if at origin (overrides first planet)
                if cell.x == 0 && cell.y == 0 && cell.z == 0 && p_idx == 0 {
                    if let Some(over) = config.planet_override {
                        p_type = over;
                    }
                }

                let (_col1, col2) = p_type.get_palette();
                let (atmos_col, atmos_density) = p_type.get_atmosphere_color();

                let dist = planet_data.distance;
                let angle = (dist as f32 * 0.123f32).sin() * std::f32::consts::TAU;
                let x = dist * angle.cos();
                let z = dist * angle.sin();

                let mut planet_entity = root.spawn((
                    // Mesh3d(common_meshes.unit_sphere_low.clone()), // Removed for LOD
                    crate::universe::terrain::PlanetTerrain::new(
                        PlanetDetails(p_type),
                        planet_data.size,
                        8,   // Max Depth
                        2.0, // Split Factor
                    ),
                    Mass(10_000.0),
                    Radius(planet_data.size),
                    Planet,
                    crate::universe::Orbit {
                        radius: dist,
                        speed: planet_data.orbit_speed,
                        angle,
                    },
                    PlanetDetails(p_type),
                    Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(planet_data.size)),
                ));

                if render_config.mode == RenderMode::Baked {
                    planet_entity.insert(crate::universe::planet_baker::DirtyPlanetTexture {
                        seed: dist, // Reuse distance as seed for consistency
                        base_color: LinearRgba::from(planet_data.color),
                        second_color: LinearRgba::from(
                            planet_data.second_color.unwrap_or(Color::BLACK),
                        ),
                        planet_type: if matches!(p_type, PlanetType::GasGiant) {
                            1
                        } else if matches!(p_type, PlanetType::Magma) {
                            3
                        } else {
                            0
                        },
                    });
                } else {
                    let visuals = planet_presets.map.get(&format!("{:?}", p_type))
                        .cloned()
                        .unwrap_or_default();

                    planet_entity.insert(MeshMaterial3d(
                        planet_materials.add(PlanetMaterial {
                            base_color: LinearRgba::from(planet_data.color),
                            second_color: LinearRgba::from(
                                planet_data.second_color.unwrap_or(Color::from(col2)),
                            ),
                            seed: dist,
                            atmosphere_color: LinearRgba::from(
                                planet_data
                                    .atmosphere_color
                                    .unwrap_or(Color::from(atmos_col)),
                            ),
                            atmosphere_density: planet_data
                                .atmosphere_density
                                .unwrap_or(atmos_density),
                            crater_map: noise_textures.crater_map.clone(),
                            ridge_map: noise_textures.ridge_map.clone(),
                            sediment_map: noise_textures.sediment_map.clone(),
                            atlas_offset: Vec2::ZERO,
                            atlas_scale: 1.0,
                            use_atlas: 0,
                            planet_class: if matches!(p_type, PlanetType::GasGiant) {
                                1
                            } else if matches!(p_type, PlanetType::Ocean) {
                                2
                            } else if matches!(p_type, PlanetType::Magma) {
                                3
                            } else if matches!(p_type, PlanetType::Desert) {
                                4
                            } else {
                                0
                            },
                            atlas_texture: atlas.atlas_handle.clone(),
                            // Tunable Parameters from Presets
                            rim_intensity: visuals.rim_intensity,
                            rim_power: visuals.rim_power,
                            haze_intensity: visuals.haze_intensity,
                            cloud_threshold: visuals.cloud_threshold,
                            cloud_opacity: visuals.cloud_opacity,
                            cloud_speed: visuals.cloud_speed,
                            specular_intensity: visuals.specular_intensity,
                            bio_intensity: visuals.bio_intensity,
                            planet_type: p_type,
                        }),
                    ));
                }

                planet_entity.with_children(|planet| {
                    planet.spawn((
                        Text2d::new(planet_data.name.clone()),
                        TextFont {
                            font_size: 80.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_xyz(0.0, planet_data.size * 3.0, 0.0)
                            .with_scale(Vec3::splat(1.0)),
                        SystemLabel,
                    ));
                });
            }
        });
    } else if !is_sun {
        // Randomized Planets (Only if not the forced Sun scenario without data)
        commands.entity(system_root).with_children(|root| {
            let mut hasher = DefaultHasher::new();
            cell.hash(&mut hasher);
            let cell_seed = hasher.finish();
            let mut rng = StdRng::seed_from_u64(cell_seed);

            let num_planets = rng.gen_range(1..=4);
            let mut desert_ocean_count = 0;
            for _i in 0..num_planets {
                let mut dist = rng.gen_range(5000.0..50000.0) + data.size;
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let planet_size = rng.gen_range(5.0..15.0);

                let planet_seed = dist * 0.123 + angle;
                let mut p_type = PlanetType::from_seed(planet_seed);

                // Constraint: Near Star for Desert/Ocean, max 1-2 per system
                if matches!(p_type, PlanetType::Desert | PlanetType::Ocean) {
                    if desert_ocean_count >= 2 {
                        // Swap to a different type if we already have 2
                        p_type = PlanetType::Ice;
                    } else {
                        desert_ocean_count += 1;
                        // Force inner orbit (2,500 to 10,000 range)
                        dist = rng.gen_range(2500.0..10000.0) + data.size;
                    }
                }

                let x = dist * angle.cos();
                let z = dist * angle.sin();

                let mut planet_entity = root.spawn((
                    // Mesh3d(common_meshes.unit_sphere_low.clone()), // Removed for LOD
                    crate::universe::terrain::PlanetTerrain::new(
                        PlanetDetails(p_type),
                        planet_size,
                        8,   // Max Depth
                        2.0, // Split Factor
                    ),
                    Mass(10_000.0),
                    Radius(planet_size),
                    Planet,
                    crate::universe::Orbit {
                        radius: dist,
                        speed: rng.gen_range(0.0005..0.002) * (100.0 / dist), // Ultra slow majestic orbits (reduced again)
                        angle,
                    },
                    PlanetDetails(p_type),
                    Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(planet_size)),
                ));

                // Use GPU Baker for all planets unless specifically overridden
                // Note: RenderMode::Baked check is now implicit default.

                let (atmos_col, atmos_density) = p_type.get_atmosphere_color();
                let (col1, col2) = p_type.get_palette(); // Standard palette

                if render_config.mode == RenderMode::Baked {
                    planet_entity.insert(crate::universe::planet_baker::DirtyPlanetTexture {
                        seed: planet_seed,
                        base_color: col1,
                        second_color: col2,
                        planet_type: if matches!(p_type, PlanetType::GasGiant) {
                            1
                        } else if matches!(p_type, PlanetType::Ocean) {
                            2
                        } else if matches!(p_type, PlanetType::Magma) {
                            3
                        } else if matches!(p_type, PlanetType::Desert) {
                            4
                        } else {
                            0
                        },
                    });
                    // Note: We don't add MeshMaterial3d here, the Baker does it!
                } else {
                    // Legacy Procedural Material (Shader per pixel)
                    let visuals = planet_presets.map.get(&format!("{:?}", p_type))
                        .cloned()
                        .unwrap_or_default();

                    planet_entity.insert(MeshMaterial3d(planet_materials.add(PlanetMaterial {
                        base_color: col1,
                        second_color: col2,
                        seed: planet_seed,
                        atmosphere_color: atmos_col,
                        atmosphere_density: atmos_density,
                        planet_class: if matches!(p_type, PlanetType::GasGiant) {
                            1
                        } else if matches!(p_type, PlanetType::Ocean) {
                            2
                        } else if matches!(p_type, PlanetType::Magma) {
                            3
                        } else if matches!(p_type, PlanetType::Desert) {
                            4
                        } else {
                            0
                        },
                        crater_map: noise_textures.crater_map.clone(),
                        ridge_map: noise_textures.ridge_map.clone(),
                        sediment_map: noise_textures.sediment_map.clone(),
                        atlas_offset: Vec2::ZERO,
                        atlas_scale: 1.0,
                        use_atlas: 0,
                        atlas_texture: atlas.atlas_handle.clone(),
                        // Tunable Parameters from Presets
                        rim_intensity: visuals.rim_intensity,
                        rim_power: visuals.rim_power,
                        haze_intensity: visuals.haze_intensity,
                        cloud_threshold: visuals.cloud_threshold,
                        cloud_opacity: visuals.cloud_opacity,
                        cloud_speed: visuals.cloud_speed,
                        specular_intensity: visuals.specular_intensity,
                        bio_intensity: visuals.bio_intensity,
                        planet_type: p_type,
                    })));
                }

                planet_entity.with_children(|planet| {
                    let p_name = if is_custom {
                        format!("{} (Planet)", planet_base_name)
                    } else {
                        format!("P {},{},{}", cell.x, cell.y, cell.z)
                    };

                    planet.spawn((
                        Text2d::new(p_name),
                        TextFont {
                            font_size: 80.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                        Transform::from_xyz(0.0, planet_size * 3.0, 0.0)
                            .with_scale(Vec3::splat(1.0)),
                        SystemLabel,
                    ));
                });
            }
        });
    }
    spawned_entities
}

// 1. Off-thread pixel generation

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
