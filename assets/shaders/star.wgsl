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
    flare_mode: u32,
    flare_enabled: u32,
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

// Optimized 3D Simplex Noise for fluid stellar strands
fn mod289_3(x: vec3<f32>) -> vec3<f32> { return x - floor(x * (1.0 / 289.0)) * 289.0; }
fn mod289_4(x: vec4<f32>) -> vec4<f32> { return x - floor(x * (1.0 / 289.0)) * 289.0; }
fn permute_4(x: vec4<f32>) -> vec4<f32> { return mod289_4(((x * 34.0) + 1.0) * x); }
fn taylorInvSqrt_4(r: vec4<f32>) -> vec4<f32> { return 1.79284291400159 - 0.85373472095314 * r; }

fn snoise(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0/6.0, 1.0/3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);
    var i  = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);
    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;
    i = mod289_3(i);
    let p = permute_4(permute_4(permute_4(
             i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
           + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0))
           + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));
    let n_ = 0.142857142857;
    let ns = n_ * D.wyz - D.xzx;
    let j = p - 49.0 * floor(p * ns.z * ns.z);
    let x = floor(j * ns.z);
    let y = floor(j - 7.0 * x);
    let x_f = x * ns.x + ns.y;
    let y_f = y * ns.x + ns.y;
    let h = 1.0 - abs(x_f) - abs(y_f);
    let b0 = vec4<f32>(x_f.xy, y_f.xy);
    let b1 = vec4<f32>(x_f.zw, y_f.zw);
    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));
    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;
    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);
    let norm = taylorInvSqrt_4(vec4<f32>(dot(p0,p0), dot(p1,p1), dot(p2, p2), dot(p3,p3)));
    p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
    var m = max(0.6 - vec4<f32>(dot(x0,x0), dot(x1,x1), dot(x2,x2), dot(x3,x3)), vec4<f32>(0.0));
    m = m * m;
    return 42.0 * dot(m * m, vec4<f32>(dot(p0,x0), dot(p1,x1), dot(p2,x2), dot(p3,x3)));
}

fn sfbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;
    for (var i = 0; i < 3; i++) {
        v += a * (snoise(pos) * 0.5 + 0.5);
        pos = pos * 2.0 + vec3<f32>(100.0);
        a *= 0.5;
    }
    return v;
}

fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic interpolation for smoother "lines"
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0); 

    return mix(
        mix(mix(hash(i + vec3<f32>(0.0, 0.0, 0.0)), hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y),
        mix(mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y), 
        u.z
    );
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
    let core_r = 1.0 / (1.1 + material.flare_height);
    let ro = in.local_camera_pos;
    let rd = normalize(in.local_pos - ro);
    
    // Ray-Sphere intersection
    let b = dot(ro, rd);
    let c = dot(ro, ro) - core_r * core_r;
    let h = b * b - c;
    
    var final_color = vec3<f32>(0.0);
    var final_alpha = 0.0;
    
    // --- 1. CORE RENDERING ---
    if (h > 0.0) {
        let t = -b - sqrt(h);
        if (t > 0.0) {
            let hit_point = ro + t * rd;
            let sphere_pos = normalize(hit_point);
            
            // CONVECTION & PLASMA
            let warp = fbm(sphere_pos * 1.6 + vec3<f32>(time * 0.1, 0.0, material.seed));
            let w_pos = sphere_pos + vec3<f32>(warp * material.warp_intensity + material.seed);
            
            let vor = voronoi(w_pos * material.convection_scale + vec3<f32>(time * material.convection_speed));
            let cells = smoothstep(0.1, 0.4, vor.y - vor.x);
            
            let turb = 0.4 + sfbm(w_pos * 4.0 + vec3<f32>(time * material.plasma_speed)) * 1.5;
            let spots = smoothstep(0.6, 0.8, sfbm(w_pos * 2.5 + vec3<f32>(time * 0.08)));
            
            // Fresnel / Rim on surface
            let rim = 1.0 - max(dot(rd, -normalize(hit_point)), 0.0);
            let rim_f = pow(rim, material.rim_power);
            
            var surface = mix(base_color * 1.6, base_color * 0.1, rim_f);
            surface *= mix(0.4, 1.0, cells);
            surface *= turb;
            surface += spots * base_color * material.hot_spot_intensity;
            
            final_color = surface;
            final_alpha = smoothstep(0.0, 0.01, h); 
        }
    }
    
    // --- 2. FLARES & CORONA (Performance Optimized Toggle) ---
    if (material.flare_enabled == 1u) {
        let d = length(ro - rd * dot(ro, rd));
        
        if (d < 1.0) {
            let r_norm = (d - core_r) / (1.0 - core_r);
            
            // Determine the radial direction from the center
            let flare_dir = normalize(in.local_pos);
            
            // 1. Create an evolving "time coordinate"
            let flow_time = time * material.flare_speed;

            // 2. Sample noise with a "Warp" component to create roiling
            // We use cross product or a fixed offset to ensure motion isn't just radial
            let warp_coord = flare_dir * material.flare_scale;
            let roil = sfbm(warp_coord * 0.5 + flow_time * 0.2);

            // 3. Combine for the final sampling position
            // Adding 'roil' into the coordinate creates the "twisting" filament look
            let flare_pos = warp_coord + (roil * 2.0) + vec3<f32>(flow_time, flow_time * 0.5, flow_time) + vec3<f32>(material.seed);
            
            var strands = 0.0;
            if (material.flare_mode == 0u) {
                // UNIFORM DISTRIBUTED FLARES
                strands = pow(sfbm(flare_pos), 4.0);
            } else {
                // RANDOM SPOTTY FLARES (Eruptions)
                let spot_mask = pow(smoothstep(0.5, 0.8, sfbm(flare_pos * 0.08)), 6.0);
                let detail = pow(sfbm(flare_pos), 3.0);
                strands = spot_mask * detail * 25.0; 
            }
            
            // SURFACE MASK: Pins the strands to the surface silhouette
            let surface_noise = sfbm(normalize(in.local_pos) * 15.0 + material.seed) * 0.05;
            let surface_mask = smoothstep(-0.2 + surface_noise, 0.1 + surface_noise, r_norm);
            
            // GENTLER FALLOFF: pow(..., 0.8) makes strands reach further/longer before fading
            let flare_alpha = strands * (1.0 - pow(clamp(r_norm, 0.0, 1.0), 0.8)) * surface_mask * material.flare_intensity;
            let corona = pow(1.0 - pow(clamp(r_norm, 0.0, 1.0), 1.2), 5.0) * material.corona_intensity;
            
            let glow_rgb = base_color * (flare_alpha + corona);
            
            if (final_alpha < 0.99) {
                final_color = mix(final_color, glow_rgb, 1.0 - final_alpha);
                final_alpha = max(final_alpha, clamp(flare_alpha + corona * 0.5, 0.0, 1.0));
            } else {
                // Bleed flares slightly onto the edge of the core for a smoother transition
                let bleed = smoothstep(0.1, -0.05, r_norm);
                final_color += glow_rgb * bleed;
            }
        }
    }
    
    // Final brightness boost
    return vec4<f32>(final_color * material.intensity, final_alpha);
}
