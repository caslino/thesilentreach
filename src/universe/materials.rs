use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

// --- STAR MATERIAL ---
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StarMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub seed: f32,
}

impl Material for StarMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/star.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
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
}

impl Material for PlanetMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/planet.wgsl".into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
