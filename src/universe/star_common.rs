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
pub fn get_star_data(
    x: i64,
    y: i64,
    z: i64,
    uni_seed: u64,
) -> Option<(crate::universe::StarType, Color, f32)> {
    // 1. Hash coordinates to a single u32 seed
    // We use a simple bit mixing for the inputs
    let mut seed = uni_seed as u32;
    seed = pcg_hash(seed ^ (x as u32));
    seed = pcg_hash(seed ^ (y as u32));
    seed = pcg_hash(seed ^ (z as u32));

    // 2. Existence Check
    // Reuse specific bits or re-hash
    let _exists_val = rand_f32(seed);

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
    let threshold = if is_origin { 1.0 } else { 0.0002 };

    if rnd1 > threshold {
        return None;
    }

    // 3. Determine Star Type using OBAFGKM distribution
    // Based on real stellar census: M=76%, K=12%, G=7%, F=3%, A=1.5%, B=0.4%, O=0.09%
    // Plus exotic remnants: NeutronStar=0.009%, BlackHole=0.001%
    let rnd_type = rand_f32(pcg_hash(final_seed.wrapping_add(1)));

    use crate::universe::StarType;
    let star_type = if rnd_type < 0.76 {
        StarType::M_RedDwarf // 76% - Most common
    } else if rnd_type < 0.88 {
        StarType::K_OrangeDwarf // 12% - The new "Goldilocks" star
    } else if rnd_type < 0.95 {
        StarType::G_YellowDwarf // 7% - Sun-like
    } else if rnd_type < 0.98 {
        StarType::F_YellowWhite // 3% - Cream/Off-white
    } else if rnd_type < 0.995 {
        StarType::A_White // 1.5% - Pure white
    } else if rnd_type < 0.999 {
        StarType::B_BlueWhite // 0.4% - Pale blue
    } else if rnd_type < 0.9999 {
        StarType::O_BlueGiant // 0.09% - Rare giants
    } else if rnd_type < 0.99999 {
        StarType::NeutronStar // 0.009% - Very rare exotic
    } else {
        StarType::BlackHole // 0.001% - Extremely rare
    };

    // 4. Properties from Type
    let rnd_size = rand_f32(pcg_hash(final_seed.wrapping_add(4))); // Keep same seed offset for stability

    // Vary color slightly from base
    let base_color = star_type.get_base_color();
    let LinearRgba {
        red, green, blue, ..
    } = LinearRgba::from(base_color);

    // Slight random variation (+/- 5%)
    let tint_r = 0.95 + 0.1 * rand_f32(pcg_hash(final_seed.wrapping_add(2)));
    let tint_g = 0.95 + 0.1 * rand_f32(pcg_hash(final_seed.wrapping_add(3)));
    let tint_b = 0.95 + 0.1 * rand_f32(pcg_hash(final_seed.wrapping_add(5))); // New offset

    let color = Color::srgb(red * tint_r, green * tint_g, blue * tint_b);

    let (min_s, max_s) = star_type.get_size_range();
    let size = min_s + (max_s - min_s) * rnd_size;

    Some((star_type, color, size))
}
