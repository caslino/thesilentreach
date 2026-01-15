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
    
    // --- 1. GRANULATION (Convection Cells) ---
    let gran_scale = 12.0;
    let gran_speed = 0.05;
    let gran_pos = sphere_pos * gran_scale + vec3<f32>(sin(time * gran_speed), cos(time * gran_speed * 0.7), 0.0);
    let vor = voronoi(gran_pos);
    let cell_edge = smoothstep(0.0, 0.15, vor.y - vor.x); // Dark edges between cells
    let granulation = mix(0.7, 1.0, cell_edge);
    
    // --- 2. TURBULENT PLASMA FLOW ---
    let plasma_speed = 0.15;
    let shift = vec3<f32>(time * plasma_speed, time * plasma_speed * 0.7, material.seed);
    
    // Multi-scale turbulence
    let turb1 = fbm(sphere_pos * 4.0 + shift);
    let turb2 = fbm(sphere_pos * 8.0 - shift * 0.5);
    let turb3 = fbm(sphere_pos * 16.0 + shift * 1.5);
    let turbulence = turb1 * 0.5 + turb2 * 0.35 + turb3 * 0.15;
    
    // --- 3. HOT SPOTS (Active Regions) ---
    let hot_pos = sphere_pos * 3.0 + vec3<f32>(material.seed * 0.1);
    let hot_noise = fbm(hot_pos + vec3<f32>(time * 0.1));
    let hot_spots = smoothstep(0.55, 0.75, hot_noise);
    
    // --- 4. COLOR GRADIENT ---
    // K-type orange dwarf: bright yellow-orange core, deeper orange-red edges
    let base_color = material.color.rgb;
    
    // Brighter core color (shift toward yellow)
    let core_color = base_color + vec3<f32>(0.3, 0.15, 0.0);
    // Darker edge color (shift toward red-orange)  
    let edge_color = base_color * vec3<f32>(0.9, 0.6, 0.4);
    
    // Fresnel for edge detection
    let view_dir = normalize(mesh_view_bindings::view.world_position.xyz - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let rim = 1.0 - max(dot(view_dir, normal), 0.0);
    let rim_factor = pow(rim, 1.5);
    
    // Mix core to edge based on rim and turbulence
    var surface_color = mix(core_color, edge_color, rim_factor * 0.6);
    
    // Apply granulation (darker between cells)
    surface_color *= granulation;
    
    // Apply turbulence variation
    let turb_brightness = 0.85 + turbulence * 0.3;
    surface_color *= turb_brightness;
    
    // Hot spots add brightness and yellow tint
    surface_color += hot_spots * vec3<f32>(0.4, 0.2, 0.0);
    
    // --- 5. CORONA GLOW ---
    let corona_power = 4.0;
    let corona_intensity = pow(rim, corona_power) * 0.8;
    let corona_color = base_color * 1.2;
    surface_color += corona_color * corona_intensity;
    
    // --- 6. LIMB DARKENING ---
    // Real stars are brighter at center, darker at edges
    let limb_darkening = 1.0 - rim_factor * 0.3;
    surface_color *= limb_darkening;
    
    // --- 7. HDR OUTPUT ---
    let intensity = 6.0;
    
    return vec4<f32>(surface_color * intensity, 1.0);
}
