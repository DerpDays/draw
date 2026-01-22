use atlas::TextureVertex;
use bytemuck::{Pod, Zeroable};
use color::{LinearSrgb, PremulColor};

use crate::{
    GraphicsContext,
    Mesh,
    arena::Arena,
    shaders::{AllocMesh, ViewportBinds},
};

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub kind: u32,
    pub texture: u32,
    pub tex_coords: [f32; 2],
}
impl TextVertex {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x2, 1=> Float32x4, 2=> Uint32, 3=> Uint32, 4=> Float32x2];

    pub const fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        debug_assert!(
            size_of::<Self>() % wgpu::VERTEX_ALIGNMENT as usize == 0,
            "vertex alignment is not aligned to wgpu::VERTEX_ALIGNMENT bytes",
        );
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VertexKind {
    Color = 0,
    MaskTexture = 1,
    ColorTexture = 2,
}

impl TextVertex {
    #[inline(always)]
    pub const fn new_color(position: [f32; 2], color: PremulColor<LinearSrgb>) -> Self {
        Self {
            position,
            color: color.components,
            kind: VertexKind::Color as u32,
            texture: 0,
            tex_coords: [0., 0.],
        }
    }
    /// Create the vertices needed for a solid single color rectangle with min/max coordinates in
    /// CCW order. indices: 0,1,2 0,2,3
    #[inline(always)]
    pub const fn new_solid_rect(
        min: [f32; 2],
        max: [f32; 2],
        color: PremulColor<LinearSrgb>,
    ) -> [Self; 4] {
        [
            Self::new_color([max[0], min[1]], color),
            Self::new_color(min, color),
            Self::new_color([min[0], max[1]], color),
            Self::new_color(max, color),
        ]
    }
    #[inline(always)]
    pub const fn from_texture_vertex(
        vertex: TextureVertex,
        color: PremulColor<LinearSrgb>,
        kind: VertexKind,
    ) -> Self {
        Self {
            position: vertex.position,
            color: color.components,
            kind: kind as u32,
            texture: vertex.texture_layer,
            tex_coords: vertex.texture_coords,
        }
    }
}

pub struct VertexArenaMarker;
pub struct IndexArenaMarker;
pub struct TextState {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,

    pub vertices_arena: Arena<VertexArenaMarker>,
    pub indices_arena: Arena<IndexArenaMarker>,
}
impl TextState {
    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text"),
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
            label: Some("text"),
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
                            label: Some("text"),
                            ..Default::default()
                        },
                    )),
                },
            ],
        })
    }
    fn create_pipeline(
        ctx: &GraphicsContext,
        viewport: &ViewportBinds,
        bind_group_layout: &wgpu::BindGroupLayout,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        let module = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("text"),
                bind_group_layouts: &[viewport.bind_group_layout(), &bind_group_layout],
                immediate_size: 0,
            });
        ctx.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("text"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[TextVertex::buffer_layout()],
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
impl TextState {
    pub fn new(
        ctx: &GraphicsContext,
        viewport_binds: &ViewportBinds,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        let bind_group_layout = Self::create_bind_group_layout(&ctx.device);
        Self {
            pipeline: Self::create_pipeline(
                ctx,
                viewport_binds,
                &bind_group_layout,
                render_targets,
            ),
            bind_group: Self::create_bind_group(&ctx, &bind_group_layout),
            vertices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::VERTEX),
            indices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::INDEX),
        }
    }
}

impl TextState {
    #[inline(always)]
    pub fn insert_mesh(
        &mut self,
        ctx: &GraphicsContext,
        mesh: Mesh<TextVertex>,
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
        new: Mesh<TextVertex>,
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

impl TextState {
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
            (alloc.vertices.byte_index() / size_of::<TextVertex>()) as i32,
            0..1,
        );
    }
}
