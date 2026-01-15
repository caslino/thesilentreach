use bevy::{
    ecs::system::SystemParamItem, // Added type
    prelude::*,
    render::{
        Render,
        RenderApp,
        RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_phase::AddRenderCommand, // Added trait
        render_resource::*,
        renderer::RenderDevice,
        view::ExtractedView,
    },
};
use bytemuck::{Pod, Zeroable};

pub struct GPUOrbitPlugin;

impl Plugin for GPUOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<OrbitBatch>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<OrbitComputePipeline>()
            .init_resource::<OrbitRenderPipeline>()
            .add_systems(Render, prepare_orbit_buffers.in_set(RenderSet::Prepare))
            .add_systems(Render, queue_orbit_draw.in_set(RenderSet::Queue))
            .add_render_command::<Transparent3d, DrawOrbitBatch>();

        // Note: Full Render Integration is complex (needs MaterialPlugin or Custom Phase).
        // For "Step 1", getting compute working is key.
        // We will try to rely on Bevy's Material infrastructure if possible, or use a custom node.
        // For visual validation, we might skip the custom render pipeline implementation in this file
        // and just focus on the compute part, or hook it into standard material via a specialized buffer binding
        // if we were resizing standard materials.
        // Given prompt "Use the State buffer directly in your Vertex Shaders", implies custom material or extended standard one.
        // Let's stick to Compute preparation first.
    }
}

// --- Data Structures ---

#[derive(Component, Clone)]
pub struct OrbitBatch {
    pub elements: Vec<OrbitalElement>,
}

#[derive(Clone, Copy, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct OrbitalElement {
    pub radius: f32,
    pub speed: f32,
    pub initial_angle: f32,
    pub eccentricity: f32,
    pub color: [f32; 4], // Added Color (16 bytes)
}

#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
#[repr(C)]
pub struct OrbitState {
    pub current_angle: f32,
    pub world_position: [f32; 3],
}

impl ExtractComponent for OrbitBatch {
    type QueryData = &'static OrbitBatch;
    type QueryFilter = ();
    type Out = OrbitBatch;

    fn extract_component(
        batch: bevy::ecs::query::QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(batch.clone())
    }
}

// --- GPU Resources ---

#[derive(Component)]
pub struct OrbitGPUData {
    pub element_buffer: Buffer,
    pub state_buffer: Buffer,
    pub bind_group: BindGroup,
    pub render_bind_group: BindGroup,
    pub count: u32,
}

#[derive(Resource)]
pub struct OrbitComputePipeline {
    pub pipeline: CachedComputePipelineId,
    pub layout: BindGroupLayout,
}

impl FromWorld for OrbitComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            Some("orbit_compute_layout"),
            &[
                BindGroupLayoutEntry {
                    // Uniforms
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    // Elements (Read)
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    // State (ReadWrite)
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/orbit_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("orbit_compute_pipeline".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        });

        OrbitComputePipeline { pipeline, layout }
    }
}

#[derive(Resource)]
pub struct OrbitRenderPipeline {
    pub pipeline: CachedRenderPipelineId,
    pub view_layout: BindGroupLayout,
    pub orbit_layout: BindGroupLayout,
}

impl FromWorld for OrbitRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let mesh_pipeline = world.resource::<MeshPipeline>();
        let view_layout = mesh_pipeline.view_layouts[0].bind_group_layout.clone();

        let orbit_layout = render_device.create_bind_group_layout(
            Some("orbit_render_layout"),
            &[
                // Uniforms (Time etc) ? No, View has time maybe.
                // Or reutilize compute uniforms?
                // For now, Group 1 is Storage Buffer (States)
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        );

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/orbit_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("orbit_render_pipeline".into()),
            layout: vec![view_layout.clone(), orbit_layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![
                    // If we used a mesh, we'd define layout here.
                    // But we are generating vertices in shader (VertexPulling) or using a helper mesh?
                    // Prompt implicitly suggests using Vertex Shader for transform.
                    // Let's use a simple hardcoded triangle/quad in shader or empty buffer if drawing strip.
                    // Actually, let's assume we draw 6 vertices per instance (quad).
                    // We won't bind vertex buffers.
                ],
            },
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater, // Bevy uses Reverse Z?
                // Bevy Default is standard Z?
                // Wait, Bevy 0.15 uses Reverse Z usually?
                // Let's check star_renderer. It used GreaterEqual.
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        });

        OrbitRenderPipeline {
            pipeline,
            view_layout,
            orbit_layout,
        }
    }
}

// Draw Command
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::pbr::{MeshPipeline, SetMeshViewBindGroup};
use bevy::render::render_phase::{
    DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, SetItemPipeline,
    TrackedRenderPass,
};

type DrawOrbitBatch = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    DrawOrbitBatchCommand,
);

struct DrawOrbitBatchCommand;
impl RenderCommand<Transparent3d> for DrawOrbitBatchCommand {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = &'static OrbitGPUData; // Query from the entity

    #[inline]
    fn render<'w>(
        _item: &Transparent3d,
        _view: (),
        gpu_data: Option<&'w OrbitGPUData>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(data) = gpu_data else {
            return RenderCommandResult::Failure("No GPU Data");
        };

        // Group 1: Orbit State
        pass.set_bind_group(1, &data.render_bind_group, &[]);

        // Draw 6 verts * count instances
        pass.draw(0..6, 0..data.count);

        RenderCommandResult::Success
    }
}

// Queue
use bevy::render::render_phase::ViewSortedRenderPhases;

fn queue_orbit_draw(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    render_pipeline: Res<OrbitRenderPipeline>,
    views: Query<Entity, With<ExtractedView>>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    query: Query<(Entity, &OrbitGPUData)>,
) {
    let draw_function = transparent_3d_draw_functions
        .read()
        .get_id::<DrawOrbitBatch>()
        .unwrap();

    for view_entity in views.iter() {
        let Some(phase) = phases.get_mut(&view_entity) else {
            continue;
        };

        for (entity, _) in query.iter() {
            phase.add(Transparent3d {
                entity: (entity, bevy::render::sync_world::MainEntity::from(entity)),
                pipeline: render_pipeline.pipeline,
                draw_function,
                distance: 1000.0, // Sort order?
                batch_range: 0..1,
                extra_index: bevy::render::render_phase::PhaseItemExtraIndex::NONE,
            });
        }
    }
}

fn prepare_orbit_buffers(
    mut commands: Commands,
    query: Query<(Entity, &OrbitBatch), Without<OrbitGPUData>>,
    render_device: Res<RenderDevice>,
    render_pipeline: Res<OrbitRenderPipeline>, // Added
    compute_pipeline: Res<OrbitComputePipeline>,
) {
    for (entity, batch) in query.iter() {
        if batch.elements.is_empty() {
            continue;
        }

        let count = batch.elements.len() as u32;

        // 1. Element Buffer
        let element_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("orbit_elements"),
            contents: bytemuck::cast_slice(&batch.elements),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        // 2. State Buffer
        let init_states = vec![OrbitState::default(); count as usize];
        let state_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("orbit_states"),
            contents: bytemuck::cast_slice(&init_states),
            usage: BufferUsages::STORAGE
                | BufferUsages::VERTEX
                | BufferUsages::COPY_DST
                | BufferUsages::COPY_SRC,
        });

        // 3. Uniform Buffer (Time) - Per batch or global?
        // Should be global ideally, but for simplicity creating one here or just binding a dummy for now.
        // We will need a system to update this uniform every frame.
        let uniform_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("orbit_uniforms"),
            size: 16, // time(f32), dt(f32), padding
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // 4. Compute Bind Group
        let bind_group = render_device.create_bind_group(
            Some("orbit_compute_group"),
            &compute_pipeline.layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: element_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: state_buffer.as_entire_binding(),
                },
            ],
        );

        // 5. Render Bind Group
        let render_bind_group = render_device.create_bind_group(
            Some("orbit_render_group"),
            &render_pipeline.orbit_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: state_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: element_buffer.as_entire_binding(),
                },
            ],
        );

        commands.entity(entity).insert(OrbitGPUData {
            element_buffer,
            state_buffer,
            bind_group,
            render_bind_group,
            count,
        });
    }
}
