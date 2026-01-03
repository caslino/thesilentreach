#import bevy_pbr::forward_io::VertexOutput

struct StarfieldMaterial {
    galactic_pos: vec3<f32>,
    time: f32,
};

@group(2) @binding(0) var<uniform> material: StarfieldMaterial;

@group(2) @binding(1) var noise_texture: texture_2d<f32>;
@group(2) @binding(2) var noise_sampler: sampler;

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz);
    
    // --- STARS (Procedural - Keep as is, it's cheap/crisp) ---
    // Galactic Pos offset for stars
    let star_coord = dir * 1000.0 + material.galactic_pos * 0.0001;
    let h = hash3(floor(star_coord * 1.5));
    var stars = 0.0;
    
    if (h > 0.995) {
        let base_brightness = pow((h - 0.995) / 0.005, 5.0);
        let twinkle_seed = hash3(floor(star_coord * 1.5) + vec3<f32>(13.37, 42.0, 7.0));
        let twinkle_speed = 3.0 + twinkle_seed * 2.0; 
        let twinkle = 0.5 + 0.5 * sin(material.time * twinkle_speed + twinkle_seed * 100.0);
        stars = base_brightness * mix(0.7, 1.2, twinkle); 
    }

    // --- NEBULA (Textured Triplanar) ---
    // We sample the 2D noise texture three times (X, Y, Z planes) to fake 3D volume.
    // Plus 2 layers of scrolling for motion.

    let n_scale = 0.2; 
    let speed = 0.005;
    let time = material.time;
    
    // Triplanar Weights
    let w = pow(abs(dir), vec3<f32>(4.0)); // Sharp blend
    let weights = w / (w.x + w.y + w.z);

    // Coordinate scrolling (Layer 1)
    let scroll1 = vec2<f32>(time * speed, time * speed * 0.5);
    let uv_x1 = dir.yz * n_scale + scroll1;
    let uv_y1 = dir.xz * n_scale + scroll1 + vec2<f32>(0.5); // Offset
    let uv_z1 = dir.xy * n_scale + scroll1 + vec2<f32>(0.2); 

    let n1 = textureSample(noise_texture, noise_sampler, uv_x1).r * weights.x +
             textureSample(noise_texture, noise_sampler, uv_y1).r * weights.y +
             textureSample(noise_texture, noise_sampler, uv_z1).r * weights.z;

    // Layer 2 (Reverse Scroll)
    let scroll2 = vec2<f32>(-time * speed * 0.8, time * speed * 0.2);
    let uv_x2 = dir.yz * n_scale * 1.5 + scroll2; // Different scale
    let uv_y2 = dir.xz * n_scale * 1.5 + scroll2; 
    let uv_z2 = dir.xy * n_scale * 1.5 + scroll2; 

    let n2 = textureSample(noise_texture, noise_sampler, uv_x2).r * weights.x +
             textureSample(noise_texture, noise_sampler, uv_y2).r * weights.y +
             textureSample(noise_texture, noise_sampler, uv_z2).r * weights.z;
    
    // Combine Layers
    let n = (n1 + n2) * 0.5; // Average
    
    // Colorize
    let teal = vec3<f32>(0.1, 0.8, 0.9);
    let purple = vec3<f32>(0.5, 0.0, 0.5);
    let nebula_color = mix(purple, teal, n);
    let nebula_strength = pow(n, 2.5) * 0.4; // Contrast curve

    let final_color = vec3<f32>(stars) + nebula_color * nebula_strength;
    
    return vec4<f32>(final_color, 1.0);
}
