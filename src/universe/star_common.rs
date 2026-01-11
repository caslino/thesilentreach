use bevy::prelude::*;

// PCG Hash for stateless random numbers - Ported to WGSL later
// Source: http://www.jcgt.org/published/0009/03/02/
pub fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277803737);
    return (word >> 22) ^ word;
}

pub fn rand_f32(seed: u32) -> f32 {
    let output = pcg_hash(seed);
    (output as f32) / (u32::MAX as f32)
}

// Generate Star Properties from Cell + UniverseSeed
pub fn get_star_data(x: i64, y: i64, z: i64, uni_seed: u64) -> Option<(Color, f32)> {
    // 1. Hash coordinates to a single u32 seed
    // We use a simple bit mixing for the inputs
    let mut seed = uni_seed as u32;
    seed = pcg_hash(seed ^ (x as u32));
    seed = pcg_hash(seed ^ (y as u32));
    seed = pcg_hash(seed ^ (z as u32));

    // 2. Existence Check
    // Reuse specific bits or re-hash
    let _exists_val = rand_f32(seed);

    // Logic from spawner.rs:
    // let is_origin = x == 0 && y == 0 && z == 0;
    // let density_chance = if is_origin { 1.0 } else { 0.005 };

    // Note: We need to handle coord wrapping if i64 is large, but for now casting to u32 is "okay"
    // for local noise, though technically it loops every 4 billion cells.
    // A better approach is hashing u64 chunks.
    // For this prototype, we'll assume the cast is fine or improve the hash mixing.

    // Improved Mixing for i64
    let mut h = (uni_seed as u64).wrapping_add(0x9E3779B97F4A7C15);
    h = h ^ (x as u64);
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h = h ^ (y as u64);
    h = h.wrapping_mul(0x94D049BB133111EB);
    h = h ^ (z as u64);
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);

    // Fold to u32
    let final_seed = (h ^ (h >> 32)) as u32;

    let rnd1 = rand_f32(final_seed);

    let is_origin = x == 0 && y == 0 && z == 0;
    let threshold = if is_origin { 1.0 } else { 0.001 };

    if rnd1 > threshold {
        return None;
    }

    // 3. Properties
    let rnd_r = rand_f32(pcg_hash(final_seed.wrapping_add(1)));
    let rnd_g = rand_f32(pcg_hash(final_seed.wrapping_add(2)));
    let rnd_b = rand_f32(pcg_hash(final_seed.wrapping_add(3)));
    let rnd_size = rand_f32(pcg_hash(final_seed.wrapping_add(4)));

    let color = Color::srgb(rnd_r, rnd_g, rnd_b);
    // size range 20.0 .. 100.0
    let size = 20.0 + 80.0 * rnd_size;

    Some((color, size))
}
