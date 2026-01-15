#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct PlanetMaterial {
    base_color: vec4<f32>,
    second_color: vec4<f32>,
    seed: f32,
    atmosphere_color: vec4<f32>,
    atmosphere_density: f32,
    atlas_offset: vec2<f32>,
    atlas_scale: f32,
    use_atlas: u32,
    planet_class: u32,
};

@group(2) @binding(0) var<uniform> material: PlanetMaterial;
@group(2) @binding(1) var crater_map: texture_2d<f32>;
@group(2) @binding(2) var crater_sampler: sampler;
@group(2) @binding(3) var ridge_map: texture_2d<f32>;
@group(2) @binding(4) var ridge_sampler: sampler;
@group(2) @binding(5) var sediment_map: texture_2d<f32>;
@group(2) @binding(6) var sediment_sampler: sampler;
@group(2) @binding(7) var atlas_texture: texture_2d<f32>;
@group(2) @binding(8) var atlas_sampler: sampler;

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
    var color: vec3<f32>;

    if (material.use_atlas != 0u) {
        let atlas_uv = in.uv * material.atlas_scale + material.atlas_offset;
        color = textureSample(atlas_texture, atlas_sampler, atlas_uv).rgb;
    } else if (material.planet_class == 1u) {
        // --- GAS GIANT GENERATION ---

        let uv = in.uv;
        
        // 1. Zonal Flow (Banding) with Turbulence
        
        let time = 0.0;
        let shift = vec2<f32>(cos(material.seed), sin(material.seed));

        // Layer 1: Low frequency band distortion
        let noise_low = textureSample(crater_map, crater_sampler, fract(uv * 3.0 + shift)).r;
        
        // Layer 2: High frequency turbulence
        let noise_high = textureSample(crater_map, crater_sampler, fract(uv * 12.0 - shift)).r;
        
        // Combine for detailed band distortion
        let dist_y = uv.y * 18.0 + noise_low * 0.4 + noise_high * 0.1;
        
        // Sharpen the bands slightly for distinct zones
        let band_noise = sin(dist_y);
        let band_detail = sin(dist_y * 3.0 + noise_high * 5.0) * 0.2; // Sub-bands
        
        let band_factor = (band_noise + band_detail) * 0.5 + 0.5;
        let band_factor_clamped = smoothstep(0.2, 0.8, band_factor); // Contrast
        
        // Mix Colors
        color = mix(material.base_color.rgb, material.second_color.rgb, band_factor_clamped);
        
        // Add subtle white clouds/turbulence on top
        let cloud_noise = textureSample(crater_map, crater_sampler, fract(uv * 8.0 + vec2<f32>(time*0.01, 0.0))).r;
        if (cloud_noise > 0.7) {
            color = mix(color, vec3<f32>(1.0, 1.0, 0.9), (cloud_noise - 0.7) * 0.5);
        }
        
        // 2. The Great Red Spot (Storms)
        // Check distance to storm center(s)
        // Use seed to position storm? Or fixed for Jupiter specifically?
        // Prompt asks for: uv(0.4, 0.6)
        
        let storm_center = vec2<f32>(0.4, 0.6);
        let storm_dist = distance(uv, storm_center);
        
        // Perturb storm edge
        let storm_radius = 0.08 + noise_low * 0.01;
        
        if (storm_dist < storm_radius) {
            let storm_alpha = smoothstep(storm_radius, storm_radius - 0.02, storm_dist);
            // Deep Red Storm Color
            let storm_color = vec3<f32>(0.6, 0.1, 0.05); 
            color = mix(color, storm_color, storm_alpha);
        }

    } else {
        // --- ORGANIC PROCEDURAL GENERATION (Terrestrial/Others) ---
        
        let scale = 4.0;
        let uv = in.uv * scale;
        
        // 1. Domain Warping (Perturb UVs with crater map for jaggedness)
        // This is key for "Organic Coastlines"
        let warp_scale = 0.02;
        let warp = textureSample(crater_map, crater_sampler, uv).r;
        let perturbed_uv = uv + vec2<f32>(warp) * warp_scale;

        // 2. Hybrid Noise Sampling
        // Use the seed to rotate/shift
        let angle = material.seed * 123.45;
        let shift = vec2<f32>(cos(material.seed), sin(material.seed));
        let rot_uv = rotate_uv(perturbed_uv + shift, angle);
        
        // Sample maps
        let raw_n = textureSample(crater_map, crater_sampler, fract(rot_uv)).r;
        let ridge = textureSample(ridge_map, ridge_sampler, fract(rot_uv)).r;
        let sediment = textureSample(sediment_map, sediment_sampler, fract(rot_uv * 2.0)).r;

        // Combine for terrain height
        let n = mix(raw_n, raw_n + ridge * 0.5, 0.5);

        // 3. Biome Coloring
        // Use smoothstep for coastlines instead of if/else
        let water_level = 0.55;
        let shore_width = 0.01; // Smooth transition

        let deep_ocean = vec3<f32>(0.0, 0.05, 0.2);
        let shallow_ocean = vec3<f32>(0.0, 0.4, 0.7);
        let beach = vec3<f32>(0.8, 0.7, 0.5);
        let land_base = material.base_color.rgb;
        let land_forest = material.second_color.rgb;
        let snow = vec3<f32>(1.0);

        // Ocean Factor (1.0 = Land, 0.0 = Water)
        let land_factor = smoothstep(water_level - shore_width, water_level + shore_width, n);

        // Water Color
        let water_col = mix(deep_ocean, shallow_ocean, n / water_level);
        
        // Land Color 
        var land_col = mix(beach, land_base, smoothstep(water_level, water_level + 0.05, n));
        land_col = mix(land_col, land_forest, smoothstep(0.65, 0.8, n)); // Forest
        land_col = mix(land_col, snow, smoothstep(0.85, 0.95, n)); // Peaks

        // Mix Land and Water
        color = mix(water_col, land_col, land_factor);
        
        // Apply sediment detail
        color = mix(color, color * 0.8, sediment * 0.3);
    }
    
    // --- LIGHTING & ATMOSPHERE ---

    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0)); // Fake Sun

    // Diffuse
    let NdotL = max(dot(normal, light_dir), 0.0);
    let diffuse = NdotL + 0.1; // Ambient
    color *= diffuse;

    // Atmospheric Scattering (Rim Light)
    let NdotV = max(dot(normal, view_dir), 0.0);
    let rim_strength = 1.0 - NdotV;
    let rim = pow(rim_strength, 4.0); // Sharp rim

    let atmos_color = material.atmosphere_color.rgb;
    let density = material.atmosphere_density;

    // 1. Horizon Glow (Additive)
    let horizon_glow = atmos_color * rim * density * 2.0;
    
    // 2. Day Side Haze (Mix)
    // Haze is stronger at grazing angles but also present on face
    let haze_factor = (rim_strength * 0.5 + 0.2) * density;
    color = mix(color, atmos_color, haze_factor * 0.5);

    // Add Horizon Glow
    color += horizon_glow;

    return vec4<f32>(color, 1.0);
}
