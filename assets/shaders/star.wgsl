#import bevy_pbr::mesh_view_bindings as mesh_view_bindings
#import bevy_pbr::mesh_functions as mesh_functions

struct StarMaterial {
    color: vec4<f32>,
    seed: f32,
    convection_scale: f32,
    convection_speed: f32,
    warp_intensity: f32,
    plasma_speed: f32,
    hot_spot_intensity: f32,
    corona_intensity: f32,
    rim_power: f32,
    intensity: f32,
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

// --- VORONOI FOR GRANULATION ---
fn voronoi(p: vec3<f32>) -> vec2<f32> {
    let cell = floor(p);
    let frac_p = fract(p);
    
    var min_dist = 8.0;
    var second_dist = 8.0;
    
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let neighbor = vec3<f32>(f32(x), f32(y), f32(z));
                let point = hash3(cell + neighbor);
                let diff = neighbor + point - frac_p;
                let dist = dot(diff, diff);
                
                if (dist < min_dist) {
                    second_dist = min_dist;
                    min_dist = dist;
                } else if (dist < second_dist) {
                    second_dist = dist;
                }
            }
        }
    }
    
    return vec2<f32>(sqrt(min_dist), sqrt(second_dist));
}

fn hash3(p: vec3<f32>) -> vec3<f32> {
    var q = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6))
    );
    return fract(sin(q) * 43758.5453123);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = mesh_view_bindings::globals.time;
    let sphere_pos = normalize(in.world_position.xyz);
    
    // --- 1. ORGANIC CONVECTION (Domain Warped Voronoi) ---
    // Warp the sphere coordinates with noise to break the "blocky" grid
    let warp_scale = 1.6;
    let warp_noise = fbm(sphere_pos * warp_scale + vec3<f32>(time * 0.1, 0.0, material.seed));
    let warped_pos = sphere_pos + vec3<f32>(warp_noise * material.warp_intensity); 

    let gran_scale = material.convection_scale + sin(material.seed) * 0.5;
    let gran_speed = material.convection_speed;
    let gran_pos = warped_pos * gran_scale + vec3<f32>(time * gran_speed, cos(time * gran_speed * 0.8), 0.0);
    
    let vor = voronoi(gran_pos);
    // Wider, softer edges
    let cell_edge = smoothstep(0.1, 0.4, vor.y - vor.x); 
    
    // --- 2. TURBULENT PLASMA FLOW ---
    let plasma_speed = material.plasma_speed;
    let shift = vec3<f32>(time * plasma_speed, time * plasma_speed * 0.6, material.seed);
    
    // Multi-scale noise for internal flow
    let turb1 = fbm(warped_pos * 4.0 + shift);
    let turb2 = fbm(warped_pos * 8.0 - shift * 0.4);
    let turbulence = (turb1 * 0.6 + turb2 * 0.4);
    
    // --- 3. HOT SPOTS ---
    let hot_pos = warped_pos * 2.5 + vec3<f32>(material.seed * 0.1);
    let hot_noise = fbm(hot_pos + vec3<f32>(time * 0.08));
    let hot_spots = smoothstep(0.6, 0.8, hot_noise);
    
    // --- 4. COLOR GRADIENT (Hue Preserving) ---
    let base_color = material.color.rgb;
    
    // Rim Darkening for depth
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let rim = 1.0 - max(dot(view_dir, normal), 0.0);
    
    // Sharp limb darkening
    let rim_factor = pow(rim, material.rim_power); 

    // Core is brighter but NOT white. 
    // Mix towards a deeper version of the hue at the edges
    let core_color = base_color * 1.6;
    let edge_color = base_color * 0.1; // Deep contrast at horizon
    
    var surface_color = mix(core_color, edge_color, rim_factor);
    
    // --- HIGH CONTRAST TEXTURES ---
    
    // Convection Cells (Organic bubbles)
    // Lightened borders (0.4) for more natural look
    let granulation = mix(0.4, 1.0, cell_edge);
    surface_color *= granulation;
    
    // Turbulence creates rifts and glows
    let turb_factor = 0.4 + turbulence * 1.5;
    surface_color *= turb_factor;
    
    // Hot spots add intense hue-locked glow
    surface_color += hot_spots * base_color * material.hot_spot_intensity;
    
    // --- 5. CORONA GLOW ---
    let corona_intensity = pow(rim, 6.0) * material.corona_intensity;
    let corona_color = base_color * 1.5;
    surface_color += corona_color * corona_intensity;
    
    // --- 6. GLOBAL LIMB DARKENING PASS ---
    // Final sphericity push
    surface_color *= (1.0 - pow(rim, 2.0) * 0.7);
    
    // --- 7. HDR OUTPUT ---
    let intensity = material.intensity;
    
    return vec4<f32>(surface_color * intensity, 1.0);
}
