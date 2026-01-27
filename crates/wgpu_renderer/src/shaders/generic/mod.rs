use crate::{
    GraphicsContext,
    Mesh,
    arena::Arena,
    shaders::{AllocMesh, ViewportBinds},
};

mod vertex;
use euclid::default::Box2D;
pub use vertex::{Vertex, VertexKind};

pub struct VertexArenaMarker;
pub struct IndexArenaMarker;

pub struct GenericRenderer {
    pub pipeline: wgpu::RenderPipeline,

    pub bind_group: wgpu::BindGroup,

    pub vertices_arena: Arena<VertexArenaMarker>,
    pub indices_arena: Arena<IndexArenaMarker>,
}
impl GenericRenderer {
    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("generic"),
            entries: &[
                // Mask texture atlas
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                // Color texture atlas
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        })
    }
    fn create_bind_group(ctx: &GraphicsContext, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("generic"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &ctx.texture_state.mask_atlas.texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &ctx.texture_state.color_atlas.texture_view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&ctx.device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            label: Some("generic"),
                            ..Default::default()
                        },
                    )),
                },
            ],
        })
    }
    fn create_pipeline(
        device: &wgpu::Device,
        viewport: &ViewportBinds,
        bind_group_layout: &wgpu::BindGroupLayout,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        let module = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[viewport.bind_group_layout(), bind_group_layout],
            immediate_size: 0,
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Vertex::buffer_layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: render_targets,
            }),
            multiview_mask: None,
            cache: None,
        })
    }
}
impl GenericRenderer {
    pub fn new(
        ctx: &GraphicsContext,
        viewport_binds: &ViewportBinds,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        let bind_group_layout = Self::create_bind_group_layout(&ctx.device);
        Self {
            pipeline: Self::create_pipeline(
                &ctx.device,
                viewport_binds,
                &bind_group_layout,
                render_targets,
            ),
            bind_group: Self::create_bind_group(ctx, &bind_group_layout),
            vertices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::VERTEX),
            indices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::INDEX),
        }
    }
}

impl GenericRenderer {
    #[inline(always)]
    pub fn insert_mesh(
        &mut self,
        ctx: &GraphicsContext,
        mesh: Mesh<Vertex>,
    ) -> AllocMesh<VertexArenaMarker, IndexArenaMarker> {
        AllocMesh {
            vertices: self.vertices_arena.insert(
                &ctx.device,
                &ctx.queue,
                bytemuck::cast_slice(&mesh.vertices),
            ),
            indices: self.indices_arena.insert(
                &ctx.device,
                &ctx.queue,
                bytemuck::cast_slice(&mesh.indices),
            ),
        }
    }

    #[inline(always)]
    pub fn update_mesh(
        &mut self,
        ctx: &GraphicsContext,
        alloc: AllocMesh<VertexArenaMarker, IndexArenaMarker>,
        new: Mesh<Vertex>,
    ) -> AllocMesh<VertexArenaMarker, IndexArenaMarker> {
        AllocMesh {
            vertices: self.vertices_arena.update(
                &ctx.device,
                &ctx.queue,
                alloc.vertices,
                bytemuck::cast_slice(&new.vertices),
            ),
            indices: self.indices_arena.update(
                &ctx.device,
                &ctx.queue,
                alloc.indices,
                bytemuck::cast_slice(&new.indices),
            ),
        }
    }

    #[inline(always)]
    pub fn remove_mesh(&mut self, alloc: AllocMesh<VertexArenaMarker, IndexArenaMarker>) {
        self.vertices_arena.remove(alloc.vertices);
        self.indices_arena.remove(alloc.indices);
    }
}

impl GenericRenderer {
    #[inline(always)]
    #[profiling::function]
    pub fn swap_pipeline(
        &self,
        render_pass: &mut wgpu::RenderPass,
        viewport_binds: &ViewportBinds,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, viewport_binds.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertices_arena.inner_buffer().slice(..));
        render_pass.set_index_buffer(
            self.indices_arena.inner_buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
    }
    #[inline(always)]
    #[profiling::function]
    pub fn render_mesh_alloc(
        &self,
        render_pass: &mut wgpu::RenderPass,
        alloc: &AllocMesh<VertexArenaMarker, IndexArenaMarker>,
    ) {
        log::info!(
            "rendering text alloc vertices: {:?} indices: {:?}",
            alloc.vertices.len(),
            alloc.indices.len()
        );
        render_pass.draw_indexed(
            (alloc.indices.byte_index() / 4) as u32
                ..((alloc.indices.byte_index() + alloc.indices.len()) / 4) as u32,
            (alloc.vertices.byte_index() / size_of::<Vertex>()) as i32,
            0..1,
        );
    }
}
