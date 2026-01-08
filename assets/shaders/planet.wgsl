#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct PlanetMaterial {
    base_color: vec4<f32>,
    second_color: vec4<f32>,
    seed: f32,
    atmosphere_color: vec4<f32>,
    atmosphere_density: f32,
};

@group(2) @binding(0) var<uniform> material: PlanetMaterial;
@group(2) @binding(1) var crater_map: texture_2d<f32>;
@group(2) @binding(2) var crater_sampler: sampler;
@group(2) @binding(3) var ridge_map: texture_2d<f32>;
@group(2) @binding(4) var ridge_sampler: sampler;
@group(2) @binding(5) var sediment_map: texture_2d<f32>;
@group(2) @binding(6) var sediment_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Transform Position
    var world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position;
    out.clip_position = mesh_view_bindings::view.clip_from_world * out.world_position;
    
    // Transform Normal
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    
    out.uv = vertex.uv;
    return out;
}

// --- HYBRID SAMPLING ---

fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    let mat = mat2x2<f32>(c, -s, s, c);
    return mat * (uv - 0.5) + 0.5;
}

fn hybrid_sampling(uv: vec2<f32>, seed: f32) -> f32 {
    let scale = 4.0;
    
    // Apply seed based perturbation
    let angle = seed * 123.45;
    let shift = vec2<f32>(cos(seed), sin(seed));

    let perturbed_uv = rotate_uv(uv * scale + shift, angle);

    // Sample textures
    // Note: textures are small patches, so we tile them
    let uv_tiled = fract(perturbed_uv);

    let crater = textureSample(crater_map, crater_sampler, uv_tiled).r;
    let ridge = textureSample(ridge_map, ridge_sampler, uv_tiled).r;
    let sediment = textureSample(sediment_map, sediment_sampler, uv_tiled).r;

    // Combine logic (can be tweaked)
    // Crater map defines base shapes (continents/craters)
    // Ridge map adds detail
    // Sediment adds variation

    // Simple mix:
    let base = crater;
    let detail = ridge * 0.5;

    // Use sediment to mask detail or add smoothness
    let val = mix(base, base + detail, sediment);

    return val;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Terrain Noise (Hybrid)
    // Map world pos to UV for sphere (simple approximation or use mesh UVs)
    // Mesh UVs are usually good for spheres if unwrapped properly.
    // The previous shader used world position for 3D noise.
    // Here we use UVs with the hybrid sampling.
    
    let n = hybrid_sampling(in.uv, material.seed);
    
    // 2. Mix Land/Sea
    // Threshold usually around 0.5 for water
    let water_level = 0.52;
    let grass_level = 0.58;
    
    // Smoothstep for anti-aliased coastlines
    // let is_land = smoothstep(water_level - 0.01, water_level + 0.01, n);
    
    // Colors
    let deep_ocean = vec3<f32>(0.0, 0.05, 0.2);
    let shallow_ocean = vec3<f32>(0.0, 0.4, 0.7);
    let beach = vec3<f32>(0.8, 0.7, 0.5);
    let grass = material.base_color.rgb;
    let forest = material.second_color.rgb;
    let mountain = vec3<f32>(0.5, 0.5, 0.5);
    let snow = vec3<f32>(1.0, 1.0, 1.0);
    
    // Sediment sample for color variation
    // Use a different scale/rotation for sediment color map to decouple from height
    let sediment_val = textureSample(sediment_map, sediment_sampler, fract(in.uv * 2.0 + vec2<f32>(material.seed))).r;

    // Ocean Gradient
    var color = mix(deep_ocean, shallow_ocean, n / water_level);
    
    // Add sediment variegation to ocean
    color = mix(color, color * 1.2, sediment_val * 0.5);

    // Land Gradient
    if n > water_level {
        if n < grass_level {
            // Beach
            color = beach;
        } else if n < 0.75 {
            // Grass / Forest
            // Use ridge/sediment to mix grass and forest
            let mix_factor = smoothstep(grass_level, 0.75, n);
            color = mix(grass, forest, mix_factor);

            // Add sediment variegation
             color = mix(color, color * 0.9, sediment_val);

        } else if n < 0.85 {
            // Mountain
            color = mountain;
        } else {
            // Snow
            color = snow;
        }
    }
    
    // 3. Lighting (Simple Diffuse + Hemisphere)
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    
    // Fake "Sun" direction (Assume usually (1,1,1) for generic look)
    let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.1);
    
    // Apply Diffuse Lighting first
    color = color * diffuse;

    // 4. Atmospheric Scattering (Rim / Haze)
    // Calculate Fresnel effect
    let NdotV = max(dot(normal, view_dir), 0.0);
    
    // "Rim" creates the glowing edge
    let rim_strength = 1.0 - NdotV; 
    let rim = pow(rim_strength, 4.0);
    
    // "Scatter" mimics the atmosphere getting thicker at glancing angles, but also visible on day side.
    // We mix it based on density
    let atmosphere_color = material.atmosphere_color.rgb;
    let density = material.atmosphere_density;

    // Simple scattering approximation:
    // Mix surface color with atmosphere color based on Fresnel and Density.
    // Boosted at the rim (horizon).
    
    let scatter_factor = pow(rim_strength, 2.5) * density * 2.0;
    
    // Additive blend for light scattering (makes it look glowing/hazy)
    color = color + (atmosphere_color * scatter_factor);
    
    // "Daylight" Scattering: slightly tint the whole lit side
    let day_scatter = atmosphere_color * 0.2 * density * diffuse;
    color = color + day_scatter;

    return vec4<f32>(color, 1.0);
}
