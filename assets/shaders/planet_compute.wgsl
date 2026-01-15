@group(0) @binding(0) var texture: texture_storage_2d<rgba8unorm, write>;

struct PlanetParams {
    seed: f32,
    base_color: vec4<f32>,
    second_color: vec4<f32>,
    planet_type: u32, // 0: Terran, 1: GasGiant, 2: Ice, 3: Magma
};

@group(0) @binding(1) var<uniform> params: PlanetParams;

// --- Noise Functions ---

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * .1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);

    let a = hash12(i + vec2<f32>(0.0, 0.0));
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(st: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 0.0;
    
    var p = st;
    
    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * noise(p);
        p = p * 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// Stretched FBM for Gas Giants (Bands)
fn w_fbm(st: vec2<f32>, octaves: i32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    
    // Stretch Y coordinate to create bands
    var p = vec2<f32>(st.x * 2.0, st.y * 10.0 + params.seed * 5.0); 
    
    for (var i = 0; i < octaves; i = i + 1) {
        value += amplitude * noise(p);
        p = p * 2.0;
        amplitude *= 0.5;
    }
    return value;
}

@compute @workgroup_size(16, 16, 1)
fn generate_planet(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let dims = textureDimensions(texture);
    let size = vec2<u32>(dims.x, dims.y);
    
    if (invocation_id.x >= size.x || invocation_id.y >= size.y) {
        return;
    }

    let uv = vec2<f32>(f32(invocation_id.x) / f32(size.x), f32(invocation_id.y) / f32(size.y));

    // Seed offset
    let p = uv * 5.0 + vec2<f32>(params.seed, params.seed * 0.5);

    var n = 0.0;
    
    // Planet Logic
    if (params.planet_type == 1u) {
        // Gas Giant (Banded)
        let warp = fbm(p, 2); // Tiny warp
        let banded_p = p + vec2<f32>(warp * 0.1, 0.0);
        n = w_fbm(banded_p, 6);
        
        // Sharpen bands
        n = smoothstep(0.2, 0.8, n);
    } else if (params.planet_type == 3u) {
        // Magma (High contrast, turbulent)
        n = fbm(p * 2.0, 5);
        n = pow(n, 1.5); // Increase contrast
    } else {
        // Terran / Ice / Default
        n = fbm(p, 5);
    }

    let color = mix(params.base_color, params.second_color, n);
    
    textureStore(texture, invocation_id.xy, color);
}
