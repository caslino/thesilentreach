use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

// --- STAR MATERIAL ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StarMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub seed: f32,
    #[uniform(0)]
    pub convection_scale: f32,
    #[uniform(0)]
    pub convection_speed: f32,
    #[uniform(0)]
    pub warp_intensity: f32,
    #[uniform(0)]
    pub plasma_speed: f32,
    #[uniform(0)]
    pub hot_spot_intensity: f32,
    #[uniform(0)]
    pub corona_intensity: f32,
    #[uniform(0)]
    pub rim_power: f32,
    #[uniform(0)]
    pub intensity: f32,
    #[uniform(0)]
    pub flare_scale: f32,
    #[uniform(0)]
    pub flare_speed: f32,
    #[uniform(0)]
    pub flare_intensity: f32,
    #[uniform(0)]
    pub flare_height: f32,
    #[uniform(0)]
    pub flare_mode: u32,
    
    // Metadata for runtime sync (not sent to GPU)
    pub star_type: super::StarType,
}

impl Material for StarMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/star.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/star.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

// --- PLANET MATERIAL ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct PlanetMaterial {
    #[uniform(0)]
    pub base_color: LinearRgba,
    #[uniform(0)]
    pub second_color: LinearRgba,
    #[uniform(0)]
    pub seed: f32,
    #[uniform(0)]
    pub atmosphere_color: LinearRgba,
    #[uniform(0)]
    pub atmosphere_density: f32,
    #[uniform(0)]
    pub atlas_offset: Vec2,
    #[uniform(0)]
    pub atlas_scale: f32,
    #[uniform(0)]
    pub use_atlas: u32,
    #[uniform(0)]
    pub planet_class: u32,

    #[texture(7)]
    #[sampler(8)]
    pub atlas_texture: Handle<Image>,

    #[texture(1)]
    #[sampler(2)]
    pub crater_map: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub ridge_map: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub sediment_map: Handle<Image>,
}

impl Material for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/planet.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/planet.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
