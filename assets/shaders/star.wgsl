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
    let time = mesh_view_bindings::globals.time;

    // 1. Dynamic Noise Surface
    // We layer two frequencies of noise to create "roiling" plasma
    let animate_speed = 0.2;
    let shift = vec3<f32>(time * animate_speed);
    
    let pos_fast = normalize(in.world_position.xyz) * 6.0 + shift;
    let pos_slow = normalize(in.world_position.xyz) * 2.0 + vec3<f32>(material.seed);
    
    let n_fast = fbm(pos_fast);
    let n_slow = fbm(pos_slow);
    let n = mix(n_slow, n_fast, 0.6);

    // 2. Color Gradient (Black Body Simulation approximation)
    let base_color = material.color.rgb;
    let core_color = vec3<f32>(1.0, 1.0, 1.0) * 3.0; // Hot white core
    
    // Mix based on noise intensity
    // Hot spots are brighter
    let heat = smoothstep(0.2, 0.8, n);
    var color = mix(base_color, core_color, heat * heat);

    // 3. Fresnel Glow (Corona)
    // The edge of the star should glow intensely
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let fresnel = 1.0 - max(dot(view_dir, normal), 0.0);
    
    let corona_intensity = pow(fresnel, 3.0);
    color += base_color * corona_intensity * 10.0;

    // 4. HDR Bloom Push
    // Multiply the final color by a high value to force it into the HDR range for Bloom
    // 4. Hot Core Logic (Fix White Star)
    // We want the center to be hot white, but the edges to retain color
    let intensity = 20.0;
    let glow = mix(material.color.rgb, vec3<f32>(1.0, 1.0, 1.0), heat * 0.5);
    
    // Add corona to glow
    let final_col = glow + (material.color.rgb * corona_intensity * 2.0);

    return vec4<f32>(final_col * intensity, 1.0);
}
