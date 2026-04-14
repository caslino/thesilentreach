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
    @location(3) local_position: vec3<f32>,
};

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Transform Position
    out.local_position = vertex.position;
    var world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position;
    out.clip_position = mesh_view_bindings::view.clip_from_world * out.world_position;
    
    // Transform Normal
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    
    out.uv = vertex.uv;
    return out;
}

// --- HYBRID SAMPLING ---

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(443.897, 441.423, 437.195));
    p3 += dot(p3, p3.yzx + 19.19);
    return fract((p3.xx + p3.yz) * p3.zy);
}

fn triplanar_sample(tex: texture_2d<f32>, samp: sampler, p: vec3<f32>, n: vec3<f32>) -> f32 {
    let w = abs(n);
    let w_sum = w.x + w.y + w.z;
    let blend = w / max(w_sum, 0.00001);
    let cx = textureSampleLevel(tex, samp, fract(p.yz), 0.0).r;
    let cy = textureSampleLevel(tex, samp, fract(p.xz), 0.0).r;
    let cz = textureSampleLevel(tex, samp, fract(p.xy), 0.0).r;
    return cx * blend.x + cy * blend.y + cz * blend.z;
}

fn hybrid_sampling(pos: vec3<f32>, normal: vec3<f32>, seed: f32) -> f32 {
    let warp_offset = triplanar_sample(sediment_map, sediment_sampler, pos * 1.5, normal);
    let warped_pos = pos + normal * (warp_offset * 0.05);

    var amplitude = 0.5;
    var frequency = 1.0;
    var total_noise = 0.0;
    var total_amplitude = 0.0;
    
    for(var i = 0u; i < 4u; i = i + 1u) {
        let offset = hash22(vec2<f32>(f32(i), seed));
        let offset3d = vec3<f32>(offset.x, offset.y, offset.x * offset.y); // stochastic offset per octave
        let sample_pos = warped_pos * frequency + offset3d; 
        
        // Alternate textures for organic feel
        var val = 0.0;
        if (i % 2u == 0u) {
            val = triplanar_sample(crater_map, crater_sampler, sample_pos, normal);
        } else {
            val = triplanar_sample(ridge_map, ridge_sampler, sample_pos, normal);
        }
        
        total_noise += amplitude * val;
        total_amplitude += amplitude;
        
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    
    var val = total_noise / max(total_amplitude, 0.0001);
    
    // Combine and Contrast boost
    val = pow(max(val, 0.0), 1.5);
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
        let pos = in.local_position;
        let normal = normalize(in.local_position);
        let time = mesh_view_bindings::globals.time;
        let shift = vec3<f32>(cos(material.seed), sin(material.seed), 0.0);

        // Layer 1: Low frequency band distortion
        let noise_low = triplanar_sample(crater_map, crater_sampler, pos * 1.5 + shift, normal);
        let noise_high = triplanar_sample(crater_map, crater_sampler, pos * 6.0 - shift, normal);
        
        // Increase distortion to break horizontal lines into vortices
        let dist_y = uv.y * 18.0 + noise_low * 1.5 + noise_high * 0.3;
        
        let band_noise = sin(dist_y);
        let band_detail = sin(dist_y * 3.0 + noise_high * 5.0) * 0.2; 
        let band_factor = (band_noise + band_detail) * 0.5 + 0.5;
        let band_factor_clamped = smoothstep(0.2, 0.8, band_factor); 
        
        color = mix(material.base_color.rgb, material.second_color.rgb, band_factor_clamped);
        
        let time_shift = vec3<f32>(time * 0.01 * material.cloud_speed, 0.0, 0.0);
        let cloud_noise = triplanar_sample(crater_map, crater_sampler, pos * 4.0 + time_shift, normal);
        if (cloud_noise > material.cloud_threshold) {
            color = mix(color, vec3<f32>(1.0, 1.0, 0.9), (cloud_noise - material.cloud_threshold) * material.cloud_opacity);
        }
        
        let storm_center = vec2<f32>(0.4, 0.6);
        let storm_dist = distance(uv, storm_center);
        let storm_radius = 0.08 + noise_low * 0.02;
        
        if (storm_dist < storm_radius) {
            let storm_alpha = smoothstep(storm_radius, storm_radius - 0.02, storm_dist);
            let storm_color = vec3<f32>(0.6, 0.1, 0.05); 
            color = mix(color, storm_color, storm_alpha);
        }
    } else if (material.planet_class == 2u) {
        // --- OCEAN WORLD GENERATION ---
        let pos = in.local_position;
        let normal = normalize(in.local_position);
        let time = mesh_view_bindings::globals.time;
        // 1. Water Surface (Deep/Shallow)
        let n = hybrid_sampling(pos, normal, material.seed);
        let deep_water = material.base_color.rgb;
        let shallow_water = material.second_color.rgb;
        
        // Use noise for water depth variation
        color = mix(deep_water, shallow_water, smoothstep(0.3, 0.7, n));
        
        // --- LIFE: SURFACE CURRENTS & FOAM (Close-up only) ---
        let surface_speed = 0.01;
        let foam_shift = vec3<f32>(time * surface_speed, 0.0, 0.0);
        let foam = triplanar_sample(ridge_map, ridge_sampler, pos * 15.0 + foam_shift, normal);
        if (foam > 0.8) {
            color = mix(color, vec3<f32>(0.8, 0.9, 1.0), (foam - 0.8) * 5.0 * prox_factor);
        }
        
        // 2. Dynamic Clouds (White clumpy overlays)
        let cloud_shift = vec3<f32>(time * 0.005, time * 0.002, 0.0) * material.cloud_speed;
        let cloud_noise = triplanar_sample(crater_map, crater_sampler, pos * 3.0 + cloud_shift, normal);
        
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
    } else if (material.planet_class == 3u) {
        // --- MAGMA WORLD GENERATION ---
        let pos = in.local_position;
        let normal = normalize(in.local_position);
        let time = mesh_view_bindings::globals.time;
        
        let n = hybrid_sampling(pos, normal, material.seed);
        
        // Magma Base
        let rock_dark = material.base_color.rgb;
        let rock_light = material.second_color.rgb;
        color = mix(rock_dark, rock_light, n);
        
        // Magma Cracks
        let crack_noise = triplanar_sample(ridge_map, ridge_sampler, pos * 2.0 + vec3<f32>(time * 0.005), normal);
        let cracks = pow(crack_noise, 4.0); // As per user instruction
        let magma_glow = vec3<f32>(1.0, 0.3, 0.0) * cracks * material.bio_intensity;
        color += magma_glow;

        // Clouds (Ash/Smoke)
        let cloud_shift = vec3<f32>(time * 0.01 * material.cloud_speed, 0.0, 0.0);
        let cloud_noise = triplanar_sample(crater_map, crater_sampler, pos * 3.0 + cloud_shift, normal);
        if (cloud_noise > material.cloud_threshold) {
             let cloud_alpha = smoothstep(material.cloud_threshold, material.cloud_threshold + 0.2, cloud_noise);
             color = mix(color, vec3<f32>(0.1, 0.1, 0.1), cloud_alpha * material.cloud_opacity); 
        }

    } else if (material.planet_class == 4u) {
        // --- DESERT WORLD GENERATION ---
        let pos = in.local_position;
        let normal = normalize(in.local_position);
        let time = mesh_view_bindings::globals.time;
        
        // 1. Sand Dunes (Broad patterns)
        let n = hybrid_sampling(pos, normal, material.seed);
        let sand_dark = material.base_color.rgb;
        let sand_light = material.second_color.rgb;
        color = mix(sand_dark, sand_light, n);
        
        // 2. Sand Swirls (Dynamic time-based streaks)
        let swirl_shift = vec3<f32>(time * 0.02, 0.0, 0.0); 
        let swirl = triplanar_sample(ridge_map, ridge_sampler, pos * vec3<f32>(1.5, 8.0, 1.5) + swirl_shift, normal);
        if (swirl > 0.6) {
            color = mix(color, sand_light * 1.2, (swirl - 0.6) * 0.4);
        }
        
    } else {
        // --- ORGANIC PROCEDURAL GENERATION (Terrestrial/Others) ---
        let pos = in.local_position;
        let normal = normalize(in.local_position);
        
        let n = hybrid_sampling(pos, normal, material.seed);

        // Biome Coloring
        // For Terran: Use "Stepped" gradient (Hard transitions for coastlines)
        let water_level = 0.55;
        let deep_ocean = vec3<f32>(0.0, 0.05, 0.2);
        let shallow_ocean = vec3<f32>(0.0, 0.4, 0.7);
        let beach = vec3<f32>(0.8, 0.7, 0.5);
        let land_base = material.base_color.rgb;
        let land_forest = material.second_color.rgb;
        let snow = vec3<f32>(1.0);

        if (material.planet_class == 0u) {
            // Terran stepped gradient with hard transitions
            // Ocean Factor
            let is_land = step(water_level, n);
            let is_beach = step(water_level, n) * (1.0 - step(water_level + 0.02, n));
            let is_forest = step(water_level + 0.02, n) * (1.0 - step(0.75, n));
            let is_mountain = step(0.75, n) * (1.0 - step(0.85, n));
            let is_snow = step(0.85, n);
            
            let water_col = mix(deep_ocean, shallow_ocean, n / water_level);
            
            var land_col = vec3<f32>(0.0);
            land_col += beach * is_beach;
            land_col += land_base * is_forest;
            land_col += land_forest * is_mountain;
            land_col += snow * is_snow;
            
            color = mix(water_col, land_col, is_land);
            
            let sediment = triplanar_sample(sediment_map, sediment_sampler, pos * 4.0, normal);
            color = mix(color, color * 0.8, sediment * 0.3 * is_land);
        } else {
            // General Soft Organic Gradient
            let shore_width = 0.01;
            let land_factor = smoothstep(water_level - shore_width, water_level + shore_width, n);

            let water_col = mix(deep_ocean, shallow_ocean, n / water_level);
            var land_col = mix(beach, land_base, smoothstep(water_level, water_level + 0.05, n));
            land_col = mix(land_col, land_forest, smoothstep(0.65, 0.8, n)); // Forest
            land_col = mix(land_col, snow, smoothstep(0.85, 0.95, n)); // Peaks

            color = mix(water_col, land_col, land_factor);
            
            let sediment = triplanar_sample(sediment_map, sediment_sampler, pos * 2.0, normal);
            color = mix(color, color * 0.8, sediment * 0.3);
        }
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
            let bio_noise = textureSampleLevel(ridge_map, ridge_sampler, fract(bio_uv), 0.0).r;
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
