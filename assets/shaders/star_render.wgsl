#import bevy_render::view::View

struct Star {
    position: vec3<f32>,
    color: vec3<f32>,
    size: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var<storage, read> stars: array<Star>;
@group(2) @binding(0) var<uniform> model: mat4x4<f32>;

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let star = stars[instance_index];

    // UV generation for 6-vertex quad (2 coords)
    // 0: 0,0
    // 1: 0,1
    // 2: 1,1
    // 3: 1,1
    // 4: 1,0
    // 5: 0,0
    var uv = vec2<f32>(0.0, 0.0);
    let idx = vertex_index % 6u;
    if (idx == 1u) { uv = vec2<f32>(0.0, 1.0); }
    else if (idx == 2u) { uv = vec2<f32>(1.0, 1.0); }
    else if (idx == 3u) { uv = vec2<f32>(1.0, 1.0); }
    else if (idx == 4u) { uv = vec2<f32>(1.0, 0.0); }
    
    // Get World Position (Sector Origin + Star Offset)
    let world_pos = model * vec4<f32>(star.position, 1.0);
    
    // Convert to View Space
    let view_pos = view.view_from_world * world_pos;

    // Billboard in View Space
    // Offset perpendicular to view direction (which is Z in view space)
    let offset = (uv - 0.5) * star.size;
    let final_view_pos = view_pos + vec4<f32>(offset, 0.0, 0.0);

    var out: VertexOutput;
    out.clip_position = view.clip_from_view * final_view_pos;
    
    // Boost brightness for distant stars - "Glow"
    out.color = vec4<f32>(star.color * 2.0, 1.0); 
    out.uv = uv;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft Circle
    let dist = distance(in.uv, vec2<f32>(0.5));
    if (dist > 0.5) {
        discard;
    }
    
    // Radial Gradient (Core is bright, edge is soft)
    let alpha = smoothstep(0.5, 0.0, dist);
    
    // Enhance core
    let core = smoothstep(0.2, 0.0, dist);
    let color = in.color.rgb + vec3<f32>(core); 

    return vec4<f32>(color, alpha);
}
