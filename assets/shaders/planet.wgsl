#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct Globals {
    // We can access Bevy's time global via standard bindings usually,
    // but mesh_view_bindings provides `globals` struct with `time`.
    // No need to redeclare if we use that.
    _pad: f32,
}

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
    rim_intensity: f32,
    rim_power: f32,
    haze_intensity: f32,
    cloud_threshold: f32,
    cloud_opacity: f32,
    cloud_speed: f32,
    specular_intensity: f32,
    bio_intensity: f32,
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
    // --- 1. DOMAIN WARPING (Break the Grid) ---
    // Use sediment map at low freq to warp everything else
    let warp_uv = uv * 1.5 + vec2<f32>(material.seed * 0.1);
    let warp = textureSample(sediment_map, sediment_sampler, fract(warp_uv)).r;
    let warped_uv = uv + vec2<f32>(warp * 0.05);

    // --- 2. MULTI-OCTAVE FRACTAL SAMPLING ---
    let s1 = 1.2; // Macro (Continents)
    let s2 = 4.0; // Mid (Islands/Mountains)
    let s3 = 12.0; // Detail (Coasts)
    
    // Seed-based rotations for each octave
    let rot1 = rotate_uv(warped_uv * s1, seed * 1.23);
    let rot2 = rotate_uv(warped_uv * s2, seed * 4.56);
    let rot3 = rotate_uv(warped_uv * s3, seed * 7.89);

    // Sample different maps for different roles
    let macro_layer = textureSample(crater_map, crater_sampler, fract(rot1)).r;
    let mid = textureSample(ridge_map, ridge_sampler, fract(rot2)).r;
    let detail = textureSample(crater_map, crater_sampler, fract(rot3)).r;

    // --- 3. COMBINE (Non-linear layering) ---
    // macro_layer defines the big continents vs oceans (using power to sharpen)
    var val = pow(macro_layer, 1.5); 
    
    // Add mid-scale features only where there is macro land, or small islands
    val = mix(val, val + mid * 0.4, 0.6);
    
    // Add fine coastal jagging
    val = mix(val, val + detail * 0.15, 0.5);

    // Contrast boost
    val = smoothstep(0.1, 0.9, val);

    return val;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;
    let time = mesh_view_bindings::globals.time;
    let cam_pos = mesh_view_bindings::view.world_position.xyz;
    let dist_to_cam = distance(cam_pos, in.world_position.xyz);
    
    // Proximity factor (1.0 when very close, 0.0 when far)
    let prox_factor = 1.0 - smoothstep(20.0, 100.0, dist_to_cam);

    if (material.use_atlas != 0u) {
        let atlas_uv = in.uv * material.atlas_scale + material.atlas_offset;
        let sample = textureSample(atlas_texture, atlas_sampler, atlas_uv);
        
        // Fallback if texture is not ready (transparent)
        if (sample.a < 0.1) {
            color = material.base_color.rgb; // Show base color while loading
        } else {
            color = sample.rgb;
        }
    } else if (material.planet_class == 1u) {
        // --- GAS GIANT GENERATION ---
        let uv = in.uv;
        let time = mesh_view_bindings::globals.time;
        let shift = vec2<f32>(cos(material.seed), sin(material.seed));

        // Layer 1: Low frequency band distortion
        let noise_low = textureSample(crater_map, crater_sampler, fract(uv * 3.0 + shift)).r;
        let noise_high = textureSample(crater_map, crater_sampler, fract(uv * 12.0 - shift)).r;
        let dist_y = uv.y * 18.0 + noise_low * 0.4 + noise_high * 0.1;
        
        let band_noise = sin(dist_y);
        let band_detail = sin(dist_y * 3.0 + noise_high * 5.0) * 0.2; 
        let band_factor = (band_noise + band_detail) * 0.5 + 0.5;
        let band_factor_clamped = smoothstep(0.2, 0.8, band_factor); 
        
        color = mix(material.base_color.rgb, material.second_color.rgb, band_factor_clamped);
        
        let cloud_noise = textureSample(crater_map, crater_sampler, fract(uv * 8.0 + vec2<f32>(time*0.01 * material.cloud_speed, 0.0))).r;
        if (cloud_noise > material.cloud_threshold) {
            color = mix(color, vec3<f32>(1.0, 1.0, 0.9), (cloud_noise - material.cloud_threshold) * material.cloud_opacity);
        }
        
        let storm_center = vec2<f32>(0.4, 0.6);
        let storm_dist = distance(uv, storm_center);
        let storm_radius = 0.08 + noise_low * 0.01;
        
        if (storm_dist < storm_radius) {
            let storm_alpha = smoothstep(storm_radius, storm_radius - 0.02, storm_dist);
            let storm_color = vec3<f32>(0.6, 0.1, 0.05); 
            color = mix(color, storm_color, storm_alpha);
        }
    } else if (material.planet_class == 2u) {
        // --- OCEAN WORLD GENERATION ---
        let uv = in.uv;
        // 1. Water Surface (Deep/Shallow)
        let n = hybrid_sampling(uv, material.seed);
        let deep_water = material.base_color.rgb;
        let shallow_water = material.second_color.rgb;
        
        // Use noise for water depth variation
        color = mix(deep_water, shallow_water, smoothstep(0.3, 0.7, n));
        
        // --- LIFE: SURFACE CURRENTS & FOAM (Close-up only) ---
        let surface_speed = 0.01;
        let foam_uv = uv * 40.0 + vec2<f32>(time * surface_speed, 0.0);
        let foam = textureSample(ridge_map, ridge_sampler, fract(foam_uv)).r;
        if (foam > 0.8) {
            color = mix(color, vec3<f32>(0.8, 0.9, 1.0), (foam - 0.8) * 5.0 * prox_factor);
        }
        
        // 2. Dynamic Clouds (White clumpy overlays)
        let cloud_uv = uv * 6.0 + vec2<f32>(time * 0.005 * material.cloud_speed, time * 0.002 * material.cloud_speed);
        let cloud_noise = textureSample(crater_map, crater_sampler, fract(cloud_uv)).r;
        
        if (cloud_noise > material.cloud_threshold) {
            let cloud_alpha = smoothstep(material.cloud_threshold, material.cloud_threshold + 0.2, cloud_noise);
            color = mix(color, vec3<f32>(1.0, 1.0, 1.0), cloud_alpha * material.cloud_opacity);
            
            // --- LIFE: ATMOSPHERIC LIGHTNING ---
            let lightning_seed = fract(time * 0.8 + material.seed);
            let is_flash = step(0.99, lightning_seed); // Occasional flash
            let flash_intensity = sin(time * 50.0) * 0.5 + 0.5; // Flicker
            color += vec3<f32>(0.7, 0.8, 1.0) * is_flash * flash_intensity * cloud_alpha * 2.0 * prox_factor;
        }
        
        // 3. Small Archipelago (Occasional land spots)
        if (n > 0.85) {
            let land_alpha = smoothstep(0.85, 0.9, n);
            let land_col = vec3<f32>(0.2, 0.4, 0.1); // Lush green
            color = mix(color, land_col, land_alpha);
        }
    } else if (material.planet_class == 4u) {
        // --- DESERT WORLD GENERATION ---
        let uv = in.uv;
        let time = mesh_view_bindings::globals.time;
        
        // 1. Sand Dunes (Broad patterns)
        let n = hybrid_sampling(uv, material.seed);
        let sand_dark = material.base_color.rgb;
        let sand_light = material.second_color.rgb;
        color = mix(sand_dark, sand_light, n);
        
        // 2. Sand Swirls (Dynamic time-based streaks)
        let swirl_uv = uv * vec2<f32>(1.5, 8.0) + vec2<f32>(time * 0.02, 0.0);
        let swirl = textureSample(ridge_map, ridge_sampler, fract(swirl_uv)).r;
        if (swirl > 0.6) {
            color = mix(color, sand_light * 1.2, (swirl - 0.6) * 0.4);
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
    let rim = pow(rim_strength, material.rim_power); // Tunable rim

    let atmos_color = material.atmosphere_color.rgb;
    let density = material.atmosphere_density;

    // 1. Horizon Glow (Additive)
    // Reduce intensity to prevent whiteout
    let horizon_glow = atmos_color * rim * density * material.rim_intensity; 
    
    // 2. Day Side Haze (Mix)
    // Haze is stronger at grazing angles but also present on face
    let haze_factor = (rim_strength * 0.5 + 0.2) * density;
    color = mix(color, atmos_color, haze_factor * material.haze_intensity);

    // Add Horizon Glow
    color += horizon_glow;
    
    // --- LIFE: BIOLUMINESCENCE (Night Side) ---
    if (material.planet_class == 2u) {
        let night_factor = smoothstep(0.1, -0.2, dot(normal, light_dir));
        if (night_factor > 0.01) {
            // Aquatic Bio-Patterns
            let bio_uv = in.uv * 100.0;
            let bio_noise = textureSample(ridge_map, ridge_sampler, fract(bio_uv)).r;
            let bio_pulse = sin(time * 1.5 + material.seed) * 0.5 + 0.5;
            
            let glow_val = smoothstep(0.7, 0.9, bio_noise) * bio_pulse;
            let glow_color = vec3<f32>(0.0, 0.8, 1.0); // Cyan Bio-glow
            
            color += glow_color * glow_val * night_factor * prox_factor * material.bio_intensity;
        }
    }
    
    // 3. Ocean Glint (Specular)
    if (material.planet_class == 2u) {
        let half_vec = normalize(view_dir + light_dir);
        let NdotH = max(dot(normal, half_vec), 0.0);
        let glint = pow(NdotH, 64.0) * NdotL; // High power for sharp specular
        
        // Only apply where there is no cloud (using simple NdotV as proxy for cloud-free if we had a mask)
        // For now, just add it.
        color += vec3<f32>(0.8, 0.9, 1.0) * glint * material.specular_intensity * (1.0 - rim_strength * 0.5);
    }
    
    // Tone mapping helper: simple Reinhard-ish to keep things in range
    // color = color / (color + vec3<f32>(0.5)); // Optional if still too bright

    return vec4<f32>(color, 1.0);
}
