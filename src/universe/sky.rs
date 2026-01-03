use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use big_space::{FloatingOrigin, GridCell};
use crate::universe::physics::GRID_SIZE;

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<StarfieldMaterial>::default())
           .add_systems(Update, (spawn_sky_sphere, update_sky_position));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StarfieldMaterial {
    #[uniform(0)]
    pub galactic_pos: Vec3,
    #[uniform(0)]
    pub time: f32,
    #[texture(1)]
    #[sampler(2)]
    pub noise_texture: Handle<Image>,
}

impl Material for StarfieldMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/starfield.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
    
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

#[derive(Component)]
struct SkySphere;

fn spawn_sky_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StarfieldMaterial>>,
    mut images: ResMut<Assets<Image>>,
    q_ship: Query<Entity, (With<FloatingOrigin>, Without<SkySphere>)>,
) {
    if let Ok(ship_entity) = q_ship.get_single() {
         let noise_map = generate_noise_texture(&mut images);

         commands.entity(ship_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(50_000.0).mesh().ico(3).unwrap())), // Huge radius
                MeshMaterial3d(materials.add(StarfieldMaterial {
                    galactic_pos: Vec3::ZERO,
                    time: 0.0,
                    noise_texture: noise_map,
                })),
                SkySphere,
            ));
        });
    }
}

// Simple CPU Noise Generation (Baking)
fn generate_noise_texture(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    // Image is already in prelude

    let size = 256;
    let mut pixels = Vec::with_capacity(size * size * 4);
    
    // Simple noise filling
    for y in 0..size {
        for x in 0..size {
            // Use simple fract/sin hash to generate deterministic noise
            let px = x as f32;
            let py = y as f32;
            
            // Scaled coords
            let s = 0.05; 
            let mut val = 0.0;
            
            // 3 Octaves of Value Noise (Simulated)
            val += pseudo_noise(px * s, py * s) * 0.5;
            val += pseudo_noise(px * s * 2.0, py * s * 2.0) * 0.25;
            val += pseudo_noise(px * s * 4.0, py * s * 4.0) * 0.125;
            
            // Normalize
            val = val / 0.875;
            
            let v = (val * 255.0) as u8;
            pixels.extend_from_slice(&[v, v, v, 255]); // RGBA
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

fn pseudo_noise(x: f32, y: f32) -> f32 {
   let i = x.floor();
   let j = y.floor();
   let f_x = x.fract();
   let f_y = y.fract();
   
   // Hash corners
   let a = hash_2d(i, j);
   let b = hash_2d(i + 1.0, j);
   let c = hash_2d(i, j + 1.0);
   let d = hash_2d(i + 1.0, j + 1.0);
   
   // Bilinear Interpolation
   let u_x = f_x * f_x * (3.0 - 2.0 * f_x);
   let u_y = f_y * f_y * (3.0 - 2.0 * f_y);
   
   let h1 = a + (b - a) * u_x;
   let h2 = c + (d - c) * u_x;
   
   h1 + (h2 - h1) * u_y
}

fn hash_2d(x: f32, y: f32) -> f32 {
    let k = ((x * 12.9898 + y * 78.233).sin() * 43758.5453).fract();
    k
}

// Start of Update System
fn update_sky_position(
    q_camera: Query<(&GridCell<i64>, &Transform), With<FloatingOrigin>>,
    mut materials: ResMut<Assets<StarfieldMaterial>>,
    q_sky: Query<&MeshMaterial3d<StarfieldMaterial>, With<SkySphere>>,
    time: Res<Time>,
) {
    let Ok((cell, transform)) = q_camera.get_single() else { return; };
    let Ok(sky_mat_handle) = q_sky.get_single() else { return; };
    let Some(material) = materials.get_mut(sky_mat_handle) else { return; };

    // Convert GridCell + Local Pos to a "Rough Global Pos" for noise
    // We can just use the Cell coordinates as the primary driver since float precision 
    // at this scale is fine for noise offsets.
    let global_x = cell.x as f32 * GRID_SIZE + transform.translation.x;
    let global_y = cell.y as f32 * GRID_SIZE + transform.translation.y;
    let global_z = cell.z as f32 * GRID_SIZE + transform.translation.z;
    
    material.galactic_pos = Vec3::new(global_x, global_y, global_z);
    material.time = time.elapsed_secs();
}
