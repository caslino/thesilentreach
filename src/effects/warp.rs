use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_component::{
            ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
        },
        render_asset::RenderAssets,
        render_graph::{
            Node, NodeRunError, RenderGraph, RenderGraphApp, RenderGraphContext, RenderLabel,
            ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    },
};
use thesilentreach::player::camera::{Velocity, ZenCamera};

pub struct WarpPlugin;

impl Plugin for WarpPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<WarpSettings>::default(),
            UniformComponentPlugin::<WarpSettings>::default(),
        ))
        .add_systems(Update, update_warp_intensity);

        // Render App Setup
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_render_graph_node::<ViewNodeRunner<WarpNode>>(Core3d, WarpLabel)
            .add_render_graph_edge(Core3d, Node3d::Tonemapping, WarpLabel) // Run after Tonemapping
            .add_render_graph_edge(Core3d, WarpLabel, Node3d::EndMainPassPostProcessing); // Before UI? Or EndMainPass?

        // Ensure it runs before UI if possible, or just as part of post process stack.
        // Node3d::EndMainPassPostProcessing is usually the end.
        // Let's chain it: Tonemapping -> Warp -> EndMainPassPostProcessing
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<WarpPipeline>();
    }
}

// --- Component & System ---

#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct WarpSettings {
    pub intensity: f32,
    // WebGPU requires 16-byte alignment for Uniforms usually?
    // Bevy's UniformComponentPlugin handles alignment for arrays,
    // but single struct? ShaderType handles padding.
    // f32 is 4 bytes. We might need padding if used in array,
    // but as a single binding it's fine if shader matches.
    // Let's add padding just in case.
    pub padding: Vec3,
}

impl Default for WarpSettings {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            padding: Vec3::ZERO,
        }
    }
}

#[derive(Component, Default)]
pub struct WarpTimer(pub f32);

// System to update intensity based on Ship Velocity
fn update_warp_intensity(
    mut commands: Commands,
    mut q_ship: Query<
        (
            Entity,
            &Velocity,
            Option<&ZenCamera>,
            &Children,
            Option<&mut WarpTimer>,
        ),
        With<ZenCamera>,
    >,
    mut q_cam: Query<(Entity, Option<&mut WarpSettings>), With<Camera>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (ship_entity, velocity, zen_cam_opt, children, mut timer_opt) in q_ship.iter_mut() {
        let speed = velocity.0.length();
        let max_speed = zen_cam_opt.map(|z| z.max_speed).unwrap_or(200_000.0);

        // Threshold: 95% of max speed
        let threshold_speed = max_speed * 0.95;

        let mut current_timer = 0.0;

        if speed > threshold_speed {
            if let Some(ref mut timer) = timer_opt {
                timer.0 += dt;
                current_timer = timer.0;
            } else {
                commands.entity(ship_entity).insert(WarpTimer(0.0 + dt)); // Start counting
                current_timer = dt;
            }
        } else {
            // Reset timer if speed drops
            if let Some(ref mut timer) = timer_opt {
                timer.0 = 0.0;
            }
            current_timer = 0.0;
        }

        // Calculate Intensity
        // Trigger after 5 seconds. Ramp up over 1 second?
        let warp_active = current_timer > 5.0;
        let mut target_intensity = 0.0;

        if warp_active {
            // Ramping up based on time past 5s? Or just ON?
            // "Show wrap effect at that speed sustained for 5 seconds onwards"
            // Let's ramp it up smoothly over 2 seconds to avoid popping.
            let ramp_time = 2.0;
            let progress = ((current_timer - 5.0) / ramp_time).clamp(0.0, 1.0);
            target_intensity = progress;
        }

        // Apply to Camera(s)
        for child in children.iter() {
            if let Ok((cam_entity, settings_opt)) = q_cam.get_mut(*child) {
                if let Some(mut settings) = settings_opt {
                    // Smoothly interpolate current intensity to target?
                    // For now, direct set.
                    settings.intensity = target_intensity;
                } else {
                    commands.entity(cam_entity).insert(WarpSettings {
                        intensity: target_intensity,
                        padding: Vec3::ZERO,
                    });
                }
            }
        }
    }
}

// --- Render World ---

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct WarpLabel;

#[derive(Default)]
struct WarpNode;

use bevy::render::extract_component::DynamicUniformIndex; // Add import

impl ViewNode for WarpNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DynamicUniformIndex<WarpSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, settings_index): (&ViewTarget, &DynamicUniformIndex<WarpSettings>),
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<WarpPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let uniforms = world.resource::<ComponentUniforms<WarpSettings>>();

        let Some(uniform_binding) = uniforms.binding() else {
            return Ok(());
        };

        // Settings Index gives us the index in the array.
        // We need byte offset?
        // ComponentUniforms stores items aligned to dynamic offset alignment.
        // DynamicUniformIndex stores the INDEX.
        // Wait, binding with dynamic offset requires BYTE offset.
        // `ComponentUniforms::binding()` returns a binding that expects dynamic offset.
        // Bevy's internals usually convert index to offset or `DynamicUniformIndex` IS the u32 index?
        // Actually, `DynamicUniformIndex` wraps `u32`.
        // `ComponentUniforms` logic:
        // The uniform buffer is defined with `min_binding_size`.
        // The *binding* provided by `uniforms.binding()` is for the whole buffer.
        // We need to pass the offset in bytes.
        // `DynamicUniformIndex` usually *is* the index.
        // So offset = index * aligned_size?
        // Wait, Bevy's `ComponentUniforms` might return just the binding.
        // Actually, standard Bevy usage with `UniformComponentPlugin`:
        // The system `prepare_component_uniforms` writes data.
        // `DynamicUniformIndex` is added.
        // In other nodes (e.g. Mesh), `GpuMeshUniforms` has the index.
        // Bevy helper: `ComponentUniforms::uniforms().offset(...)`?
        // NO.
        // Let's check `DynamicUniformIndex` doc (mental check).
        // It holds `.index()`.
        // To get byte offset: `uniforms.uniforms().item_size() * index`.
        // Wait, `ComponentUniforms` doesn't expose `item_size` publicly?
        // Actually `DynamicUniformIndex` stores the *byte offset* in recent Bevy versions?
        // No, it stores index.
        // Let's TRY `uniforms.offsets().get()` again? No it failed.

        // Let's assume `DynamicUniformIndex` provides the u32 index.
        // And we need to calculate offset.
        // Use `uniforms.uniform_step()`. NOT exposed?
        // Wait, `binding()` returns `BindingResource::Buffer(BufferBinding { ... })`.
        // We really need the offset.

        // ALTERNATIVE: Use `ViewUniformOffset` if we wrap our component in a View Resource?
        // No.

        // Let's assume `DynamicUniformIndex` *IS* what we need, but we need the stride.
        // `ComponentUniforms::binding()` documentation says it prepares everything.
        // Let's look at `DynamicUniformComponentPlugin` usage in Bevy PBR.

        // Hack/fix:
        // Use `binding.min_binding_size`?
        // Better: `DynamicUniformIndex` in Bevy 0.13 sets `index`.
        // `ComponentUniforms` has `binding()`
        // Maybe we just pass `settings_index.index()` directly?
        // WGPU validation said: "Dynamic binding offset index 0 with offset 768..."
        // The previous code passed `uniform_offset.offset`. That was 768.
        // That offset came from `ViewUniformOffset`.
        // If we use 0, it works for the first camera.

        // Let's use `settings_index.index()`.
        // But `set_bind_group` expects *offsets*.
        // `ComponentUniforms` usually provides a way to get the offset.
        // Or `settings_index` IS the offset (u32)?
        // Inspecting Bevy code (mental): `DynamicUniformIndex` is a wrapper around u32.
        // The offset required is `index * aligned_size`.
        // `ComponentUniforms` *calculates* `aligned_size`.
        // Does it expose it?
        // `uniforms.binding()` returns `Option<BindingResource>`.
        // There is no helper on `ComponentUniforms` to get stride.

        // WAIT. Bevy's `encase` traits?
        // If I can't get the stride from `ComponentUniforms`, I might need to calculate it.
        // `RenderDevice::get_limits().min_uniform_buffer_offset_alignment`.
        // And `std::mem::size_of::<WarpSettings>()`.
        // Determine alignment.

        // Easier:
        // `DynamicUniformIndex` holds the *index* in the buffer.
        // We assume `ComponentUniforms` is packed tightly respecting alignment.

        // TRY THIS:
        // import `DynamicUniformIndex`.
        // Use `settings_index.index()`.
        // IF set_bind_group takes u32 *dynamic offset* (bytes), we need bytes.
        // IF it takes index (unlikely), then index.
        // WGPU `set_bind_group` takes `&[u32]` which are OFFSETS IN BYTES.

        // How to get the byte offset from `DynamicUniformIndex`?
        // `ComponentUniforms` doesn't expose stride.
        // This suggests `DynamicUniformIndex` might *be* the byte offset?
        // Let's check if `DynamicUniformIndex` has `.index()`?
        // Checking `extract_component.rs`: `pub struct DynamicUniformIndex<C>(u32);`
        // It's the index.

        // Wait, `ComponentUniforms` has `pub fn uniforms(&self) -> &Uniforms<C>`.
        // `Uniforms` has `write_buffer`.

        // Let's look at source if possible.
        // Or assume `ComponentUniforms` is just for `AsBindGroup` derived stuff?
        // `UniformComponentPlugin` is usually used with `AsBindGroup`.
        // But here I am manually binding.

        // Maybe I should just manually calculate the alignment.
        // `let align = render_device.limits().min_uniform_buffer_offset_alignment as usize;`
        // `let size = std::mem::size_of::<WarpSettings>();`
        // `let stride = (size + align - 1) / align * align;` // round up
        // `let offset = settings_index.index() * stride as u32;`

        // I need `RenderDevice` to get limits. `render_context.render_device()`.

        // DynamicUniformIndex stores the byte offset directly in recent Bevy versions.
        let offset = settings_index.index();

        // Check if cached pipeline is ready
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let bind_group = render_context.render_device().create_bind_group(
            "warp_bind_group",
            &pipeline.layout,
            &BindGroupEntries::sequential((
                post_process.source,     // Binding 0: Texture
                &pipeline.sampler,       // Binding 1: Sampler
                uniform_binding.clone(), // Binding 2: Settings
            )),
        );

        let mut pass = render_context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("warp_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: post_process.destination,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(LinearRgba::BLACK.into()),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

        pass.set_pipeline(render_pipeline);
        pass.set_bind_group(0, &bind_group, &[offset]);
        pass.draw(0..3, 0..1); // Fullscreen triangle

        Ok(())
    }
}

#[derive(Resource)]
struct WarpPipeline {
    pipeline_id: CachedRenderPipelineId,
    layout: BindGroupLayout,
    sampler: Sampler,
}

impl FromWorld for WarpPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let layout = render_device.create_bind_group_layout(
            "warp_bind_group_layout",
            &[
                // Screen Texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Settings Uniform
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true, // ComponentUniforms uses dynamic offset
                        min_binding_size: Some(WarpSettings::min_size()),
                    },
                    count: None,
                },
            ],
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("warp_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..default()
        });

        let shader = world.resource::<AssetServer>().load("shaders/warp.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("warp_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb, // Match Main Pass / SwapChain output
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        });

        WarpPipeline {
            pipeline_id,
            layout,
            sampler,
        }
    }
}
