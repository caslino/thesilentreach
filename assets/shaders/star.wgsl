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
    flare_scale: f32,
    flare_speed: f32,
    flare_intensity: f32,
    flare_height: f32,
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
    @location(3) local_pos: vec3<f32>,
    @location(4) local_camera_pos: vec3<f32>,
};

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    
    // Transform Position
    var world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position;
    out.clip_position = mesh_view_bindings::view.clip_from_world * out.world_position;
    
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.local_pos = vertex.position; 
    
    // Manual Local Camera Position calculation (avoid missing get_local_from_world helper)
    let star_center = world_from_local[3].xyz;
    let star_scale = length(world_from_local[0].xyz);
    let world_camera = mesh_view_bindings::view.world_position.xyz;
    out.local_camera_pos = (world_camera - star_center) / star_scale;
    
    return out;
}

// --- NOISE ---
fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(mix(hash(i + vec3<f32>(0.0)), hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
                   mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y),
               mix(mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
                   mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i++) {
        v += a * noise(pos);
        pos = pos * 2.0 + vec3<f32>(100.0);
        a *= 0.5;
    }
    return v;
}

fn hash3(p: vec3<f32>) -> vec3<f32> {
    var q = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6))
    );
    return fract(sin(q) * 43758.5453123);
}

fn voronoi(p: vec3<f32>) -> vec2<f32> {
    let cell = floor(p);
    let fr = fract(p);
    var min_dist = 8.0;
    var second_dist = 8.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let neighbor = vec3<f32>(f32(x), f32(y), f32(z));
                let point = hash3(cell + neighbor);
                let diff = neighbor + point - fr;
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

// --- FRAGMENT ---
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = mesh_view_bindings::globals.time;
    let base_color = material.color.rgb;
    
    // RAY SETUP
    // Mesh is scale S = (1.1 + H). Internal core is at radius 1.0 (local of physical star).
    // In these relative local coordinates (where mesh is 1.0), the core is at:
    let core_r = 1.0 / (1.1 + material.flare_height);
    
    let ro = in.local_camera_pos;
    let rd = normalize(in.local_pos - ro);
    
    // Ray-Sphere intersection
    let b = dot(ro, rd);
    let c = dot(ro, ro) - core_r * core_r;
    let h = b * b - c;
    
    var final_color = vec3<f32>(0.0);
    var final_alpha = 0.0;
    
    // Core Logic
    if (h > 0.0) {
        let t = -b - sqrt(h);
        if (t > 0.0) {
            let hit_point = ro + t * rd;
            let sphere_pos = normalize(hit_point);
            
            // CONVECTION & PLASMA
            let warp = fbm(sphere_pos * 1.6 + vec3<f32>(time * 0.1, 0.0, material.seed));
            let w_pos = sphere_pos + vec3<f32>(warp * material.warp_intensity);
            
            let vor = voronoi(w_pos * material.convection_scale + vec3<f32>(time * material.convection_speed));
            let cells = smoothstep(0.1, 0.4, vor.y - vor.x);
            
            let turb = 0.4 + fbm(w_pos * 4.0 + vec3<f32>(time * material.plasma_speed)) * 1.5;
            let spots = smoothstep(0.6, 0.8, fbm(w_pos * 2.5 + vec3<f32>(time * 0.08)));
            
            // Fresnel / Rim on surface
            let rim = 1.0 - max(dot(rd, -normalize(hit_point)), 0.0);
            let rim_f = pow(rim, material.rim_power);
            
            var surface = mix(base_color * 1.6, base_color * 0.1, rim_f);
            surface *= mix(0.4, 1.0, cells);
            surface *= turb;
            surface += spots * base_color * material.hot_spot_intensity;
            
            final_color = surface;
            final_alpha = 1.0;
        }
    }
    
    // FLARES & CORONA (In the shell volume)
    // d is the distance of the ray from the center at its closest point
    let d = length(ro - rd * dot(ro, rd));
    
    if (d < 1.0) {
        // Position relative to physical star silhouette
        // Normalize distance: 0 at core surface, 1 at mesh edge
        let r_norm = clamp((d - core_r) / (1.0 - core_r), 0.0, 1.0);
        
        // Solar strands (Stretched noise)
        let flare_pos = normalize(in.local_pos) * material.flare_scale + vec3<f32>(time * material.flare_speed);
        let strands = pow(fbm(flare_pos * vec3<f32>(1.0, 0.1, 1.0)), 4.0);
        
        let flare_alpha = strands * (1.0 - r_norm) * material.flare_intensity;
        let corona = pow(1.0 - r_norm, 4.0) * material.corona_intensity;
        
        let glow_rgb = base_color * (flare_alpha + corona);
        
        if (final_alpha == 0.0) {
            final_color = glow_rgb;
            final_alpha = clamp(flare_alpha + corona * 0.5, 0.0, 1.0);
        } else {
            // Overlay flares at the edges
            final_color += glow_rgb * pow(r_norm, 0.5);
        }
    }
    
    // FALLBACK DIAGNOSTICS (If nothing rendered, show very faint sphere bounds)
    if (final_alpha < 0.01) {
        final_color = base_color * 0.05;
        final_alpha = 0.05;
    }
    
    return vec4<f32>(final_color * material.intensity, final_alpha);
}
