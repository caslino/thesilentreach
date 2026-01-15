// Render Shader for GPU Orbits using Instancing

struct OrbitState {
    current_angle: f32,
    world_position: vec3<f32>,
}

@group(0) @binding(0) var<uniform> view: View; // Standard Bevy View
@group(1) @binding(0) var<storage, read> states: array<OrbitState>;

struct OrbitalElement {
    radius: f32,
    speed: f32,
    initial_angle: f32,
    eccentricity: f32,
    color: vec4<f32>,
}
@group(1) @binding(1) var<storage, read> elements: array<OrbitalElement>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
    // We can generate a simple quad or triangle in shader, or use a mesh buffer.
    // Prompt said "Link the State buffer... to the Asteroid/Planet Vertex Shader". 
    // Or better: Let's make a procedural billboard or simple point for start.
    // Let's assume we are drawing a Mesh (e.g. Instanced Cube/Sphere).
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

struct View {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
};

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    let state = states[input.instance_index]; // Instancing
    
    let orbit_pos = state.world_position;
    
    // Simple Model Transform (Identity rotation, Scale 1.0, Translate to orbit_pos)
    // If we want random rotation/scale, we need that in a buffer too.
    // For now, fixed scale.
    let scale = 10.0; // Asteroid size
    
    let world_pos = (input.position * scale) + orbit_pos;
    
    var out: VertexOutput;
    out.clip_position = view.view_proj * vec4<f32>(world_pos, 1.0);
    out.uv = input.uv;
    
    // Color based on index or something
    out.color = elements[input.instance_index].color;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple shading
    // Using UVs or just noise
    return in.color;
}
