use crate::universe::physics::GRID_SIZE;
use crate::universe::{SECTOR_SIZE, SectorIndex};
use bevy::{
    core_pipeline::core_3d::{graph::Core3d, Transparent3d},
    ecs::query::QueryItem,
    ecs::system::SystemParamItem,
    pbr::{MeshPipeline, MeshViewBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_component::{ExtractComponent, ExtractComponentPlugin, ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner},
        render_phase::{AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        sync_world::MainEntity,
        view::{ExtractedView, RetainedViewEntity},
    },
};
use bytemuck::{Pod, Zeroable};

pub struct GPUStarPlugin;

impl Plugin for GPUStarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<StarSector>::default());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .init_resource::<StarComputePipeline>()
            .init_resource::<StarRenderPipeline>()
            .init_resource::<StarsToCompute>()
            .add_systems(Render, prepare_star_buffers.in_set(RenderSet::Prepare))
            .add_systems(Render, queue_star_bind_group.in_set(RenderSet::Queue))
            .add_systems(Render, clear_stars_to_compute.in_set(RenderSet::Cleanup))
            .add_render_command::<Transparent3d, DrawStarSector>();

        let node = StarComputeNode::from_world(render_app.world_mut());
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();

        if let Some(graph_3d) = graph.get_sub_graph_mut(Core3d) {
            graph_3d.add_node(StarComputeLabel, node);
            graph_3d.add_node_edge(
                StarComputeLabel,
                bevy::core_pipeline::core_3d::graph::Node3d::StartMainPass,
            );
        }
    }
}

#[derive(Resource, Default)]
struct StarsToCompute {
    entities: Vec<Entity>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct StarComputeLabel;

pub struct StarComputeNode {
    query_state: QueryState<(Entity, &'static StarSectorBuffers)>,
}

impl FromWorld for StarComputeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query_state: world.query::<(Entity, &StarSectorBuffers)>(),
        }
    }
}

#[derive(Component, Clone)]
pub struct StarSector {
    pub index: SectorIndex,
    pub seed: u32,
}

impl ExtractComponent for StarSector {
    type QueryData = (&'static StarSector, &'static GlobalTransform);
    type QueryFilter = ();
    type Out = ExtractedStarSector;

    fn extract_component((sector, transform): QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedStarSector {
            index: sector.index,
            seed: sector.seed,
            global_transform: *transform,
        })
    }
}

#[derive(Component, Clone)]
pub struct ExtractedStarSector {
    pub index: SectorIndex,
    pub seed: u32,
    pub global_transform: GlobalTransform,
}

// --- GPU Data Structures ---

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct ComputeInputs {
    sector_x: i32,
    sector_y: i32,
    sector_z: i32,
    pad0: u32,
    universe_seed: u32,
    sector_size: u32,
    grid_size: f32,
    pad1: u32, // align to 16 bytes? struct size 32 bytes (8 u32s).
               // i32,i32,i32,u32 = 16. u32,u32,f32,u32 = 16. Total 32. Aligned.
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct StarData {
    pos: [f32; 3], // vec3
    // padding required? vec3 usually aligns to 16 bytes in arrays if using std140?
    // In "storage" buffer (std430), dense packing is allowed for float arrays,
    // but structs usually align to 16 bytes.
    // Our struct in shader: vec3, vec3, f32.
    // vec3 (12) + 4 pad? OR vec3 (12) + vec3 (12) + f32 (4) = 28. + 4 pad = 32.
    // Let's implement explicit padding.
    pad0: f32,
    color: [f32; 3],
    pad1: f32,
    size: f32,
    pad2: [f32; 3], // Pad to... wait.
                    // vec3 is 16-byte aligned in WGSL usually.
                    // struct Star { pos: vec3, color: vec3, size: f32 }
                    // Offset 0: pos
                    // Offset 16: color
                    // Offset 32: size
                    // Size 48?
}
// Actually, let's use manual packing 4 floats + 4 floats
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct StarPacked {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    color_r: f32, // pack color R here? No.
                  // Let's rely on Shader align.
                  // Use [f32; 4] for vectors?
                  // struct Star { pos: vec3, color: vec3, size: f32 }
                  // If we use `var<storage>`, it is tightly packed? No, default is std430 but alignments apply.
                  // Simple:
                  // pos: vec4 (x,y,z,pad)
                  // color: vec4 (r,g,b,size)
                  // Total 32 bytes.
}
// Update Shader to:
// struct Star { pos_pad: vec4<f32>, color_size: vec4<f32> }
// pos = pos_pad.xyz
// size = color_size.w

// --- Resources ---

#[derive(Resource)]
pub struct StarComputePipeline {
    pub pipeline: CachedComputePipelineId,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for StarComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(
            Some("star_compute_layout"),
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
                    // Stars Buffer
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    // Indirect Buffer
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    }, // Atomic? Storage R/W
                    count: None,
                },
            ],
        );

        // Load shader
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/star_compute.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("star_compute_pipeline".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: vec![],
            shader: shader,
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        });

        StarComputePipeline {
            pipeline,
            bind_group_layout: layout,
        }
    }
}

#[derive(Resource)]
pub struct StarRenderPipeline {
    pub pipeline_id: CachedRenderPipelineId,
    pub view_layout: BindGroupLayout,  // Group 0 (Bevy View)
    pub star_layout: BindGroupLayout,  // Group 1 (Stars)
    pub model_layout: BindGroupLayout, // Group 2 (Model)
}

impl FromWorld for StarRenderPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Group 0: View (Standard Bevy)
        let mesh_pipeline = world.resource::<MeshPipeline>();
        let view_layout = mesh_pipeline.view_layouts[0].bind_group_layout.clone();

        // Group 1: Stars (Storage Read)
        let star_layout = render_device.create_bind_group_layout(
            Some("star_render_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        // Group 2: Model (Uniform)
        let model_layout = render_device.create_bind_group_layout(
            Some("star_model_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        );

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/star_render.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("star_render_pipeline".into()),
            layout: vec![
                view_layout.clone(),
                star_layout.clone(),
                model_layout.clone(),
            ],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: shader,
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    // Main Pass determined to be Rgba8UnormSrgb from WGPU logs
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 4, // Default MSAA sample count
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        });

        StarRenderPipeline {
            pipeline_id,
            view_layout,
            star_layout,
            model_layout,
        }
    }
}

// --- Component to store GPU buffers on the Render Entity ---
#[derive(Component)]
pub struct StarSectorBuffers {
    pub star_buffer: Buffer,
    pub indirect_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub bind_group_compute: BindGroup,
    pub bind_group_render: BindGroup,
    pub bind_group_model: BindGroup,
    pub computed: bool,
}

// System to Prepare Buffers
fn prepare_star_buffers(
    mut commands: Commands,
    query: Query<(Entity, &ExtractedStarSector), Without<StarSectorBuffers>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    compute_pipeline: Res<StarComputePipeline>,
    render_pipeline: Res<StarRenderPipeline>,
) {
    for (entity, sector) in query.iter() {
        // 1. Inputs
        let inputs = ComputeInputs {
            sector_x: sector.index.x as i32,
            sector_y: sector.index.y as i32,
            sector_z: sector.index.z as i32,
            pad0: 0,
            universe_seed: sector.seed,
            sector_size: SECTOR_SIZE as u32,
            grid_size: GRID_SIZE,
            pad1: 0,
        };
        let input_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("star_compute_input"),
            contents: bytemuck::bytes_of(&inputs),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        // 2. Star Buffer (Max Stars? 1000 cells * density)
        // Max 1000 cells. 100% density (origin) = 1000 stars.
        // Usually 0.5% = 5 stars.
        // Safety max: 2000.
        let max_stars = 2000;
        let star_size = 32; // 4 floats + 4 floats
        let star_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("star_buffer"),
            size: (max_stars * star_size) as u64,
            usage: BufferUsages::STORAGE | BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // 3. Indirect Buffer
        let indirect_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("star_indirect"),
            size: 16, // 4 u32s
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Reset Indirect Buffer (Atomic counter must start at 0)
        // VertexCount=6, InstanceCount=0, FirstVertex=0, FirstInstance=0
        let indirect_data: [u32; 4] = [6, 0, 0, 0];
        render_queue.write_buffer(&indirect_buffer, 0, bytemuck::bytes_of(&indirect_data));

        // 4. Bind Groups
        let bind_group_compute = render_device.create_bind_group(
            Some("star_compute_group"),
            &compute_pipeline.bind_group_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: star_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: indirect_buffer.as_entire_binding(),
                },
            ],
        );

        // Render Bind Group (Stars)
        let bind_group_render = render_device.create_bind_group(
            Some("star_render_group"),
            &render_pipeline.star_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: star_buffer.as_entire_binding(),
            }],
        );
        // Model Bind Group
        let model: [f32; 16] = sector.global_transform.compute_matrix().to_cols_array();
        let model_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("star_model_buffer"),
            contents: bytemuck::cast_slice(&model),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group_model = render_device.create_bind_group(
            Some("star_model_group"),
            &render_pipeline.model_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
        );

        commands.entity(entity).insert(StarSectorBuffers {
            star_buffer,
            indirect_buffer,
            uniform_buffer: input_buffer,
            bind_group_compute,
            bind_group_render,
            bind_group_model,
            computed: false,
        });
    }
}

// Queue

// Define Draw Function
type DrawStarSector = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    DrawStarSectorCommand,
);

struct DrawStarSectorCommand;
impl bevy::render::render_phase::RenderCommand<Transparent3d> for DrawStarSectorCommand {
    type Param = Res<'static, RenderDevice>;
    type ViewQuery = ();
    // Entity is the sector
    type ItemQuery = &'static StarSectorBuffers;

    #[inline]
    fn render<'w>(
        _item: &Transparent3d,
        _view: (),
        buffers: Option<&'w StarSectorBuffers>,
        _render_device: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(buffers) = buffers else {
            return RenderCommandResult::Failure("Missing buffers");
        };

        // Bind Group 0 is handled by SetMeshViewBindGroup<0>

        // Bind Group 1 & 2: Star Data & Transform
        pass.set_bind_group(1, &buffers.bind_group_render, &[]);
        pass.set_bind_group(2, &buffers.bind_group_model, &[]);
        pass.draw_indirect(&buffers.indirect_buffer, 0);

        RenderCommandResult::Success
    }
}


// Queue System
fn queue_star_bind_group(
    _commands: Commands,
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    render_pipeline: Res<StarRenderPipeline>,
    views: Query<&ExtractedView, With<MeshViewBindGroup>>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut sectors: Query<(Entity, &mut StarSectorBuffers)>,
    mut stars_to_compute: ResMut<StarsToCompute>,
) {
    let draw_function = transparent_3d_draw_functions
        .read()
        .get_id::<DrawStarSector>()
        .unwrap();

    for view in views.iter() {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        for (entity, mut buffers) in sectors.iter_mut() {
            // Always compute ONCE
            if !buffers.computed {
                buffers.computed = true;
                stars_to_compute.entities.push(entity);
            }

            // Add Draw Command
            phase.add(Transparent3d {
                entity: (entity, MainEntity::from(entity)),
                pipeline: render_pipeline.pipeline_id,
                draw_function,
                distance: 0.0,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: false,
            });
        }
    }
}

// Fix Node logic
impl Node for StarComputeNode {
    fn update(&mut self, world: &mut World) {
        self.query_state.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let compute_pipeline = world.resource::<StarComputePipeline>();
        let stars_to_compute = world.resource::<StarsToCompute>();

        if stars_to_compute.entities.is_empty() {
            return Ok(());
        }

        let Some(pipeline) = pipeline_cache.get_compute_pipeline(compute_pipeline.pipeline) else {
            return Ok(());
        };

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);

        for entity in stars_to_compute.entities.iter() {
            if let Ok((_, buffers)) = self.query_state.get_manual(world, *entity) {
                pass.set_bind_group(0, &buffers.bind_group_compute, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
        }

        Ok(())
    }
}

fn clear_stars_to_compute(mut stars: ResMut<StarsToCompute>) {
    stars.entities.clear();
}
