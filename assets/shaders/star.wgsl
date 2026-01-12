#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct StarMaterial {
    color: vec4<f32>,
    seed: f32,
};

@group(2) @binding(0) var<uniform> material: StarMaterial;

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

// --- SIMPLEX NOISE IMPLEMENTATION ---
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
    for (var i = 0; i < 3; i++) {
        v += a * noise(pos);
        pos = pos * 2.0 + shift;
        a *= 0.5;
    }
    return v;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Base Noise (Animated)
    let animate_speed = 0.2;
    // Use globals.time
    let time = mesh_view_bindings::globals.time;
    
    // Layer 1: Base Turbulence
    let noise_pos = normalize(in.world_position.xyz) * 4.0 + vec3<f32>(material.seed) + vec3<f32>(time * animate_speed);
    let n1 = fbm(noise_pos);
    
    // Layer 2: Solar Flares / Tendrils (Higher freq, moving upwards or swirling)
    // We can simulate swirling by rotating the position based on time
    let swirl_pos = normalize(in.world_position.xyz) * 8.0 + vec3<f32>(time * 0.5); 
    let n2 = fbm(swirl_pos);
    
    // Combine noises
    let n = mix(n1, n2, 0.4); // Blend
    
    // 2. Heat Gradient
    // Map noise (0..1) to Color Gradient (Darker -> Brighter -> White Hot)
    let heat = smoothstep(0.1, 0.9, n); // Sharper transition
    
    let base_color = material.color.rgb;
    // "Blinding" white hot core
    let hot_color = vec3<f32>(2.0, 2.0, 1.8); 
    
    var final_color = mix(base_color * 0.2, base_color * 3.0, heat);
    final_color = mix(final_color, hot_color, smoothstep(0.6, 1.0, n));

    // 3. Fresnel / Atmosphere Glow
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let fresnel = 1.0 - max(dot(view_dir, normal), 0.0);
    
    // Rim glow should be the base color but very bright
    let rim = pow(fresnel, 2.5);
    final_color += base_color * rim * 8.0;

    // Emissive Push: Multiply everything to drive Bloom
    // This is the key "Radiance" factor
    let emission_strength = 20.0; // Blindingly bright

    return vec4<f32>(final_color * emission_strength, 1.0);
}
