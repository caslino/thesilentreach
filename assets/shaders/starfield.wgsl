#import bevy_pbr::forward_io::VertexOutput

struct StarfieldMaterial {
    galactic_pos: vec3<f32>,
    time: f32,
    star_density: f32,
    star_brightness: f32,
    twinkle_speed: f32,
    twinkle_intensity: f32,
    nebula_intensity: f32,
    nebula_scale: f32,
    nebula_speed: f32,
    nebula_color_a: vec4<f32>,
    nebula_color_b: vec4<f32>,
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
    
    // --- STARS (Procedural) ---
    // Galactic Pos offset for stars
    let star_coord = dir * 1000.0 + material.galactic_pos * 0.0001;
    let h = hash3(floor(star_coord * 1.5));
    var stars = 0.0;
    
    let threshold = 1.0 - material.star_density;
    if (h > threshold) {
        let base_brightness = pow((h - threshold) / (1.0 - threshold), 5.0) * material.star_brightness;
        let twinkle_seed = hash3(floor(star_coord * 1.5) + vec3<f32>(13.37, 42.0, 7.0));
        let tw_speed = (3.0 + twinkle_seed * 2.0) * material.twinkle_speed; 
        let twinkle = 0.5 + 0.5 * sin(material.time * tw_speed + twinkle_seed * 100.0);
        stars = base_brightness * mix(1.0 - material.twinkle_intensity * 0.5, 1.0 + material.twinkle_intensity * 0.5, twinkle); 
    }

    // --- NEBULA (Textured Triplanar) ---
    let n_scale = material.nebula_scale; 
    let speed = 0.005 * material.nebula_speed;
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
    
    // Colorize using material uniforms
    let nebula_color = mix(material.nebula_color_b.rgb, material.nebula_color_a.rgb, n);
    let nebula_strength = pow(n, 2.5) * 0.4 * material.nebula_intensity; // Contrast curve

    let final_color = vec3<f32>(stars) + nebula_color * nebula_strength;
    
    return vec4<f32>(final_color, 1.0);
}
