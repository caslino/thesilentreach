use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

pub struct PlanetBakerPlugin;

impl Plugin for PlanetBakerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (schedule_bakes, handle_bake_results));
    }
}

#[derive(Component)]
#[require(MeshMaterial3d<StandardMaterial>)]
pub struct DirtyPlanetTexture {
    pub seed: f32,
    pub base_color: LinearRgba,
    pub second_color: LinearRgba,
    pub planet_type: u32,
}

/// Task component holding the async texture generation job
#[derive(Component)]
struct BakingTask(Task<Image>);

/// System A: Schedule async bake tasks for planets with dirty textures
fn schedule_bakes(
    mut commands: Commands,
    q_dirty: Query<(Entity, &DirtyPlanetTexture), Without<BakingTask>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    for (entity, dirty) in q_dirty.iter() {
        // Clone values for the async block
        let seed = dirty.seed;
        let base_color = dirty.base_color;
        let second_color = dirty.second_color;
        let planet_type = dirty.planet_type;

        let task = thread_pool.spawn(async move {
            generate_planet_texture(seed, base_color, second_color, planet_type)
        });

        commands.entity(entity).insert(BakingTask(task));
    }
}

/// System B: Handle completed bake tasks and apply materials
fn handle_bake_results(
    mut commands: Commands,
    mut q_tasks: Query<(Entity, &mut BakingTask)>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut task) in q_tasks.iter_mut() {
        if let Some(image) = future::block_on(future::poll_once(&mut task.0)) {
            // Task completed - add image to assets
            let image_handle = images.add(image);

            // Create material with the generated texture
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(image_handle),
                unlit: true,
                ..default()
            });

            // Apply material and clean up markers
            commands
                .entity(entity)
                .insert(MeshMaterial3d(material))
                .remove::<DirtyPlanetTexture>()
                .remove::<BakingTask>();
        }
    }
}

/// CPU-based procedural texture generation (runs in background thread)
fn generate_planet_texture(
    seed: f32,
    base_color: LinearRgba,
    second_color: LinearRgba,
    planet_type: u32,
) -> Image {
    const SIZE: usize = 512;
    let mut data = vec![0u8; SIZE * SIZE * 4];

    for y in 0..SIZE {
        for x in 0..SIZE {
            // Simple procedural noise based on position and seed
            let u = x as f32 / SIZE as f32;
            let v = y as f32 / SIZE as f32;

            // Multi-octave noise frequency based on type
            let freq = match planet_type {
                2 => 12.0, // Ocean (Waves/Cloud clumps)
                4 => 6.0,  // Desert (Broad dunes)
                _ => 8.0,  // Default (Terran/Ice)
            };

            let noise = fbm_noise_2d(u * freq + seed, v * freq + seed);

            // Map noise to 0.0-1.0 range
            let t = (noise + 1.0) * 0.5;

            // Interpolate between base and second color
            let r = lerp(base_color.red, second_color.red, t);
            let g = lerp(base_color.green, second_color.green, t);
            let b = lerp(base_color.blue, second_color.blue, t);

            let idx = (y * SIZE + x) * 4;
            data[idx] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            data[idx + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            data[idx + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }

    Image::new(
        Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Improved 2D FBM noise using layered frequencies for organic continents
fn fbm_noise_2d(x: f32, y: f32) -> f32 {
    let mut val = 0.0;
    
    // Octave 1: Macro (Continents)
    val += (x * 0.8).sin() * (y * 0.8).cos() * 1.0;
    
    // Octave 2: Mid (Mountains)
    val += (x * 2.5 + 1.23).sin() * (y * 2.5 + 4.56).cos() * 0.5;
    
    // Octave 3: Detail (Ruggedness) - Use a high freq with phase-shifting
    val += (x * 6.0 + val * 2.0).sin() * (y * 6.0 + 7.89).cos() * 0.25;

    // Normalization and contrast
    (val * 0.6).clamp(-1.0, 1.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
