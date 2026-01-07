
// Compute Shader for generating Distant Stars

struct Star {
    position: vec3<f32>,
    color: vec3<f32>,
    size: f32,
}

struct IndirectArgs {
    vertex_count: u32,
    instance_count: atomic<u32>,
    first_vertex: u32,
    first_instance: u32,
}

struct ComputeInputs {
    sector_x: i32,
    sector_y: i32,
    sector_z: i32,
    pad0: u32,
    universe_seed: u32,
    sector_size: u32, // e.g. 10
    grid_size: f32,   // e.g. 10000.0 or whatever
}

@group(0) @binding(0) var<uniform> inputs: ComputeInputs;
@group(0) @binding(1) var<storage, read_write> stars: array<Star>;
@group(0) @binding(2) var<storage, read_write> indirect: IndirectArgs;

// PCG Hash
fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn rand_f32(seed: u32) -> f32 {
    return f32(pcg_hash(seed)) / 4294967295.0; // u32::MAX
}

// Hsv to Rgb (reused or simple implementation)
fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

@compute @workgroup_size(10, 10, 10) // 1000 threads per group? Or dispatch 1 group per sector?
// Sector Size is 10. So 10x10x10 = 1000 threads. 
// We can do one workgroup per sector if sector size fits.
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);
    let z = i32(global_id.z);

    // Bounds check if sector size varies, but we assume 10x10x10
    if (x >= i32(inputs.sector_size) || y >= i32(inputs.sector_size) || z >= i32(inputs.sector_size)) {
        return;
    }

    // Calculate Absolute Cell Coordinates
    let abs_x = inputs.sector_x * i32(inputs.sector_size) + x;
    let abs_y = inputs.sector_y * i32(inputs.sector_size) + y;
    let abs_z = inputs.sector_z * i32(inputs.sector_size) + z;

    // Hash - MUST MATCH Rust `star_common.rs` logic
    // Rust:
    // let mut seed = (uni_seed as u32);
    // seed = pcg_hash(seed ^ (x as u32)); ...
    
    // Note: Rust casts i64 to u32 loosely. We do the same.
    var seed = inputs.universe_seed;
    seed = pcg_hash(seed ^ bitcast<u32>(abs_x));
    seed = pcg_hash(seed ^ bitcast<u32>(abs_y));
    seed = pcg_hash(seed ^ bitcast<u32>(abs_z));
    
    // Existence
    let exists_val = rand_f32(seed);
    
    // Density logic
    // Rust: if x=0y=0z=0 => 1.0 same logic.
    // Handling (0,0,0) global check might be tricky if we wrap coords. 
    // Assuming simple check for now match:
    let is_origin = (abs_x == 0 && abs_y == 0 && abs_z == 0);
    var threshold = 0.001;
    if (is_origin) { threshold = 1.0; }

    if (exists_val > threshold) {
        return;
    }

    // Generate Properties
    let r = rand_f32(pcg_hash(seed + 1u));
    let g = rand_f32(pcg_hash(seed + 2u));
    let b = rand_f32(pcg_hash(seed + 3u));
    let size_rnd = rand_f32(pcg_hash(seed + 4u));

    let color = vec3<f32>(r, g, b);
    let size = 20.0 + 80.0 * size_rnd;

    // Allocate Slot
    let index = atomicAdd(&indirect.instance_count, 1u);

    // Calculate Position relative to SECTOR Origin
    // Cell offset * grid_size
    let px = f32(x) * inputs.grid_size;
    let py = f32(y) * inputs.grid_size;
    let pz = f32(z) * inputs.grid_size;

    stars[index] = Star(
        vec3<f32>(px, py, pz),
        color,
        size
    );
}
