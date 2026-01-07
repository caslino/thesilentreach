#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: WarpSettings;

struct WarpSettings {
    intensity: f32, // 0.0 to 1.0 (mapped from speed)
    // pad to 16 bytes if necessary, but f32 is 4 bytes. Uniforms effectively align to 16 bytes min size usually.
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let center = vec2<f32>(0.5, 0.5);
    let to_center = center - uv;
    let dist = length(to_center);
    let dir = normalize(to_center);

    // 1. Radial Blur
    // Only apply if intensity > 0
    // Samples: 10
    let samples = 10;
    
    // Blur strength increases with distance from center AND intensity
    let blur_amount = settings.intensity * 0.05 * dist; 

    var color = vec4<f32>(0.0);
    var total_weight = 0.0;

    for (var i = 0; i < samples; i++) {
        let scale = 1.0 - blur_amount * (f32(i) / f32(samples - 1));
        let sample_uv = uv + to_center * (1.0 - scale);
        
        // 2. Chromatic Aberration (Integrate into blur loop or separate?)
        // Let's do it inside the sample for "smeared" aberration
        
        let aber_strength = settings.intensity * 0.02 * dist;
        
        // Offset R, G, B channels
        let r_uv = sample_uv - dir * aber_strength;
        let b_uv = sample_uv + dir * aber_strength;
        
        let r = textureSample(screen_texture, screen_sampler, r_uv).r;
        let g = textureSample(screen_texture, screen_sampler, sample_uv).g;
        let b = textureSample(screen_texture, screen_sampler, b_uv).b;
        
        // Weight samples (center samples heavier?)
        // weight = 1.0 is simple average
        let weight = 1.0;
        
        color += vec4<f32>(r, g, b, 1.0) * weight;
        total_weight += weight;
    }

    return color / total_weight;
}
