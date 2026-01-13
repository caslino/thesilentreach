#import bevy_pbr::forward_io::VertexOutput
#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;

@group(2) @binding(0) var<uniform> camera_pos: vec3<f32>;
@group(2) @binding(1) var<uniform> camera_forward: vec3<f32>;
@group(2) @binding(2) var<uniform> camera_right: vec3<f32>;
@group(2) @binding(3) var<uniform> camera_up: vec3<f32>;
@group(2) @binding(4) var<uniform> time: f32;

// Pseudo-random number generator
fn hash(p: vec3<f32>) -> f32 {
    let p3 = fract(p * 0.1031);
    let d = dot(p3, vec3<f32>(p3.y + 19.19, p3.z + 19.19, p3.x + 19.19));
    return fract((p3.x + p3.y) * p3.z + d); // Fixed dot product logic
}

fn noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(mix(mix(hash(i + vec3<f32>(0.0, 0.0, 0.0)), 
                       hash(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
                   mix(hash(i + vec3<f32>(0.0, 1.0, 0.0)), 
                       hash(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y),
               mix(mix(hash(i + vec3<f32>(0.0, 0.0, 1.0)), 
                       hash(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
                   mix(hash(i + vec3<f32>(0.0, 1.0, 1.0)), 
                       hash(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var pos = p;
    for (var i = 0; i < 5; i++) {
        v += a * noise(pos);
        pos = pos * 2.0;
        a *= 0.5;
    }
    return v;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Setup Ray from Camera
    // We compute ray direction relative to the camera's orientation
    // mesh.uv is 0..1, map to -1..1
    let uv = (mesh.uv * 2.0 - 1.0);
    // Aspect ratio correction would happen here if we passed screen dims, 
    // but for now we'll assume a square-ish FOV or fix it in the app logic,
    // actually, let's assume 'camera_right' and 'camera_up' are scaled by aspect already.

    var ray_dir = normalize(camera_forward + uv.x * camera_right + uv.y * camera_up);
    var ray_pos = camera_pos;

    // Black Hole Parameters
    let bh_pos = vec3<f32>(0.0, 0.0, 0.0);
    let rs = 1.0; // Schwarzschild Radius (Event Horizon)
    // Accretion Disk Physics
    let disk_inner = 2.5 * rs;
    let disk_outer = 6.0 * rs;
    
    var color = vec3<f32>(0.0);
    var transmittance = 1.0;
    
    // Ray Marching
    let max_steps = 200;
    let step_size = 0.1; 
    var dist_sq_old = dot(ray_pos, ray_pos); // Distance Squared to BH center

    for (var i = 0; i < max_steps; i++) {
        let to_bh = bh_pos - ray_pos;
        let dist_sq = dot(to_bh, to_bh);
        let dist = sqrt(dist_sq);
        
        // 1. Interaction: Event Horizon
        if (dist < rs) {
            transmittance = 0.0;
            break; // Ray fell in
        }
        
        // 2. Interaction: Accretion Disk (Volumetric plane at Y=0)
        // We check if we crossed the Y=0 plane or are very close to it
        let height = abs(ray_pos.y);
        let radius_xz = length(ray_pos.xz);
        
        if (height < 0.2 && radius_xz > disk_inner && radius_xz < disk_outer) {
             // Calculate disk density/color
             // Noise animation
             let angle = atan2(ray_pos.z, ray_pos.x);
             
             // Speed decreases with distance (Keplerian: v ~ 1/sqrt(r))
             let speed = 2.0 / sqrt(radius_xz); 
             let rot_angle = angle + speed * time;
             
             let noise_val = fbm(vec3<f32>(radius_xz * 2.0, rot_angle * 3.0, time * 0.1));
             
             // Temperature Gradient (Hotter inside)
             let temp = 1.0 - (radius_xz - disk_inner) / (disk_outer - disk_inner);
             
             // Emission Color (Orange/Red/White)
             // Inner = Blue/White, Mid = Orange, Outer = Red
             let base_col = mix(vec3<f32>(1.0, 0.1, 0.0), vec3<f32>(0.2, 0.5, 1.0), pow(temp, 2.0));
             
             let emission = base_col * (noise_val * 2.0 + 0.5) * exp(-height * 10.0) * temp * 2.0;
             
             let sample_density = 0.1 * step_size * 20.0; // Opacity
             
             color += emission * sample_density * transmittance;
             transmittance *= (1.0 - sample_density);
             
             if (transmittance < 0.01) {
                 break;
             }
        }
        
        // 3. Gravity: Bend the ray
        // Newton approx: Force ~ 1/r^2
        // We nudge the ray direction towards the black hole
        // The force magnitude needs to be tuned for visual "lensing"
        
        let gravity_strength = 0.05; // Tunable parameter
        let force = (normalize(to_bh) * gravity_strength) / dist_sq;
        
        ray_dir += force * step_size;
        ray_dir = normalize(ray_dir);
        
        // Move Ray
        ray_pos += ray_dir * step_size;
        
        // Optimization: If we are far away and moving away, we break
        if (dist > 30.0 && dot(ray_dir, to_bh) < 0.0) {
            break;
        }
    }
    
    // Background Starfield
    if (transmittance > 0.0) {
         let star_dir = ray_dir; // The direction looked at currently
         // Simple star noise
         let star_noise = hash(floor(star_dir * 100.0)); // Grid of stars
         if (star_noise > 0.995) {
             color += vec3<f32>(1.0) * transmittance * 5.0; // Bright stars
         }
         // Nebula noise
         let neb = fbm(star_dir * 3.0 + vec3<f32>(0.1));
         color += vec3<f32>(0.05, 0.0, 0.1) * neb * transmittance;
    }

    // Tone mapping is applied by Bevy generally, but we output linear-ish values
    return vec4<f32>(color, 1.0);
}
