#import bevy_pbr::forward_io::VertexOutput

struct StarfieldMaterial {
    galactic_pos: vec3<f32>,
    time: f32,
};

@group(2) @binding(0) var<uniform> material: StarfieldMaterial;

// Hash function for white noise
fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

// 3D Simplex Noise (simplified for Nebulae)
fn random(v: vec3<f32>) -> f32 {
    return fract(sin(dot(v, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

fn noise(x: vec3<f32>) -> f32 {
    let p = floor(x);
    let f = fract(x);
    let f2 = f * f * (3.0 - 2.0 * f);
    let n = p.x + p.y * 57.0 + 113.0 * p.z;
    return mix(
        mix(mix(random(p + vec3<f32>(0.0, 0.0, 0.0)), random(p + vec3<f32>(1.0, 0.0, 0.0)), f2.x),
            mix(random(p + vec3<f32>(0.0, 1.0, 0.0)), random(p + vec3<f32>(1.0, 1.0, 0.0)), f2.x), f2.y),
        mix(mix(random(p + vec3<f32>(0.0, 0.0, 1.0)), random(p + vec3<f32>(1.0, 0.0, 1.0)), f2.x),
            mix(random(p + vec3<f32>(0.0, 1.0, 1.0)), random(p + vec3<f32>(1.0, 1.0, 1.0)), f2.x), f2.y), f2.z
    );
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var x = p;
    for (var i = 0; i < 3; i++) {
        v += a * noise(x);
        x = x * 2.0;
        a *= 0.5;
    }
    return v;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Direction from center (normalized world position of the sphere fragment)
    // The sphere is huge, so world_position ~ local_position relative to camera
    // But since we are inside, we look at -normal or just position.
    let dir = normalize(in.world_position.xyz);
    
    // Galactic Coordinate for Noise Lookup
    // We scale the galactic_pos down massively so stars don't simulate being close.
    // However, for effect, let's make them shift slightly.
    let sky_coord = dir * 1000.0 + material.galactic_pos * 0.0001;
    
    // Layer 1: Stars (High frequency white noise)
    let h = hash3(floor(sky_coord * 1.5));
    var stars = 0.0;
    
    if (h > 0.995) {
        let base_brightness = pow((h - 0.995) / 0.005, 5.0);
        
        // Time-based Twinkle
        // Use a different seed for twinkle phase to prevent synchronization
        let twinkle_seed = hash3(floor(sky_coord * 1.5) + vec3<f32>(13.37, 42.0, 7.0));
        let twinkle_speed = 3.0 + twinkle_seed * 2.0; // Random speed
        let twinkle = 0.5 + 0.5 * sin(material.time * twinkle_speed + twinkle_seed * 100.0);
        
        stars = base_brightness * mix(0.7, 1.2, twinkle); // Modulate brightness
    }
    
    // Layer 2: Nebulae (Low frequency FBM)
    // Add slow time drift to nebula
    let deep_time = material.time * 0.05;
    let nebula_scale = 0.005; 
    let nebula_coord = dir * 500.0 + material.galactic_pos * 0.001; 
    // Creating flow by shifting noise sampling
    let flow = vec3<f32>(sin(deep_time * 0.1), cos(deep_time * 0.15), deep_time * 0.2);
    let n = fbm((nebula_coord + flow) * nebula_scale);
    
    // Colorize Nebulae
    let teal = vec3<f32>(0.1, 0.8, 0.9);
    let purple = vec3<f32>(0.5, 0.0, 0.5);
    let nebula_color = mix(purple, teal, n);
    let nebula_strength = pow(n, 3.0) * 0.3; // Gentle brightness
    
    // Combine
    let final_color = vec3<f32>(stars) + nebula_color * nebula_strength;
    
    return vec4<f32>(final_color, 1.0);
}
