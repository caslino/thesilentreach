#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct PlanetMaterial {
    base_color: vec4<f32>,
    second_color: vec4<f32>,
    seed: f32,
};

@group(2) @binding(0) var<uniform> material: PlanetMaterial;

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

// --- NOISE FUNCTIONS ---
// (Duplicate of star.wgsl for now to keep self-contained)
fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(mix(mix( hash(i + vec3<f32>(0.0,0.0,0.0)), 
                        hash(i + vec3<f32>(1.0,0.0,0.0)), u.x),
                   mix( hash(i + vec3<f32>(0.0,1.0,0.0)), 
                        hash(i + vec3<f32>(1.0,1.0,0.0)), u.x), u.y),
               mix(mix( hash(i + vec3<f32>(0.0,0.0,1.0)), 
                        hash(i + vec3<f32>(1.0,0.0,1.0)), u.x),
                   mix( hash(i + vec3<f32>(0.0,1.0,1.0)), 
                        hash(i + vec3<f32>(1.0,1.0,1.0)), u.x), u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var shift = vec3<f32>(100.0);
    var pos = p;
    for (var i = 0; i < 4; i++) {
        v += a * noise(pos);
        pos = pos * 2.0 + shift;
        a *= 0.5;
    }
    return v;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Terrain Noise (FBM)
    // Rotate 3D Pos based on Time to simulate rotation shader-side (if static mesh)
    // For now, simpler: just map world pos + seed.
    let pos = normalize(in.world_position.xyz) * 4.0 + vec3<f32>(material.seed);
    
    let n = fbm(pos);
    
    // 2. Mix Land/Sea
    // Threshold usually around 0.5 for water
    let water_level = 0.52;
    let grass_level = 0.58;
    
    // Smoothstep for anti-aliased coastlines
    let is_land = smoothstep(water_level - 0.01, water_level + 0.01, n);
    
    // Colors
    let deep_ocean = vec3<f32>(0.0, 0.05, 0.2);
    let shallow_ocean = vec3<f32>(0.0, 0.4, 0.7);
    let beach = vec3<f32>(0.8, 0.7, 0.5);
    let grass = material.base_color.rgb; // Usually Green
    let forest = material.second_color.rgb; // Dark Green/Brown
    let mountain = vec3<f32>(0.5, 0.5, 0.5);
    let snow = vec3<f32>(1.0, 1.0, 1.0);
    
    // Ocean Gradient
    var color = mix(deep_ocean, shallow_ocean, n / water_level);
    
    // Land Gradient
    if n > water_level {
        if n < grass_level {
            // Beach
            color = beach;
        } else if n < 0.75 {
            // Grass / Forest
            color = mix(grass, forest, (n - grass_level) * 4.0);
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
    
    // Fake "Sun" direction (Assume usually (1,1,1) for generic look, or passed in)
    // Since this is PBR replacement, we lose real lights if we don't calculate them.
    // For simplicity, let's just do ambient + rim.
    // Ideally we want Real Lighting. But Custom Materials in Bevy 0.14+ often support standard lighting 
    // IF we implement Material properly or use StandardMaterial + Custom Texture. 
    // But this is ShaderRef.
    // We will simulate a "Lit" look.
    
    let light_dir = normalize(vec3<f32>(1.0, 0.5, 1.0));
    let diffuse = max(dot(normal, light_dir), 0.1);
    
    // Rim / Atmosphere
    let fresnel = 1.0 - max(dot(view_dir, normal), 0.0);
    let atmosphere = pow(fresnel, 4.0) * vec3<f32>(0.4, 0.6, 1.0);
    
    color = color * diffuse + atmosphere;
    
    return vec4<f32>(color, 1.0);
}
