// Compute Shader for N-Body Orbital Physics
// Logic: Update angle based on speed, calculate position.

struct ComputeUniforms {
    time: f32,
    delta_time: f32,
}

struct OrbitalElement {
    radius: f32,
    speed: f32,
    initial_angle: f32,
    eccentricity: f32, // Optional, can use for elliptical orbits 
    // For circular: x = r cos(a), z = r sin(a)
    // For elliptical: x = a cos(E), z = b sin(E) ... simpler assumption: just offset center?
    // Let's stick to circular/slightly perturbed for now as per prompt "x = cos(angle) * radius..."
}

struct OrbitState {
    current_angle: f32,
    world_position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: ComputeUniforms;
@group(0) @binding(1) var<storage, read> elements: array<OrbitalElement>;
@group(0) @binding(2) var<storage, read_write> states: array<OrbitState>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&elements)) {
        return;
    }

    let elem = elements[index];
    var state = states[index];

    // Euler integration for angle
    // angle += speed * dt
    // However, for high precision over long time, simply: angle = initial + speed * time
    // This is more stable and deterministic (stateless).
    // Prompt says: "Update angle: angle += speed * dt"
    // Stateless is better if we just pass `time`. 
    // Let's use Stateless: current_angle = initial + speed * time.
    // Wait, prompt specifically mentioned "State: Read-Write buffer of Current Positions".
    // If we use stateless, we don't strictly need to READ the old angle. We can just write the new one.
    
    let angle = elem.initial_angle + elem.speed * uniforms.time;
    
    // Circular Orbit Logic
    // x = r * cos(a)
    // z = r * sin(a)
    // y = 0 (or small perturbation?)
    
    let x = elem.radius * cos(angle);
    let z = elem.radius * sin(angle);
    let y = 0.0; // Could add inclination if we had that in OrbitalElement
    
    // Write back
    state.current_angle = angle;
    state.world_position = vec3<f32>(x, y, z);
    
    states[index] = state;
}
