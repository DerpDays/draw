use color::{LinearSrgb, PremulColor};
use euclid::default::Point2D;
use graphics_v2::{BasicLinearGradient, Primitive};

use crate::{
    GraphicsContext,
    Mesh,
    arena::Arena,
    shaders::{AllocMesh, ViewportBinds},
};

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BasicShapeVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}
impl BasicShapeVertex {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1=> Float32x4];

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

impl BasicShapeVertex {
    #[inline(always)]
    pub const fn new_color(position: [f32; 2], color: PremulColor<LinearSrgb>) -> Self {
        Self {
            position,
            color: color.components,
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

    /// Create the vertices needed for a linear gradient rectangle with min/max coordinates in
    /// CCW order. indices: 0,1,2 0,2,3
    #[inline(always)]
    pub fn new_gradient_rect(
        min: [f32; 2],
        max: [f32; 2],
        color: &BasicLinearGradient,
    ) -> [Self; 4] {
        [
            Self::new_color(
                [max[0], min[1]],
                color.get_point_premul_cs(Point2D::new(max[0], min[1])),
            ),
            Self::new_color(min, color.get_point_premul_cs(Point2D::new(min[0], min[1]))),
            Self::new_color(
                [min[0], max[1]],
                color.get_point_premul_cs(Point2D::new(min[0], min[1])),
            ),
            Self::new_color(max, color.get_point_premul_cs(Point2D::new(max[0], max[1]))),
        ]
    }
}

pub struct VertexArenaMarker;
pub struct IndexArenaMarker;

pub struct BasicShapeState {
    pub pipeline: wgpu::RenderPipeline,
    pub msaa_pipeline: wgpu::RenderPipeline,

    pub vertices_arena: Arena<VertexArenaMarker>,
    pub indices_arena: Arena<IndexArenaMarker>,
}

impl BasicShapeState {
    fn create_pipeline(
        ctx: &GraphicsContext,
        viewport: &ViewportBinds,
        multisample: wgpu::MultisampleState,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        let module = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));
        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("basic_shapes"),
                bind_group_layouts: &[viewport.bind_group_layout()],
                immediate_size: 0,
            });
        ctx.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("basic_shapes"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[BasicShapeVertex::buffer_layout()],
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
                multisample,
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

impl BasicShapeState {
    pub fn new(
        ctx: &GraphicsContext,
        viewport_binds: &ViewportBinds,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        Self {
            pipeline: Self::create_pipeline(
                ctx,
                viewport_binds,
                wgpu::MultisampleState::default(),
                render_targets,
            ),
            msaa_pipeline: Self::create_pipeline(
                ctx,
                viewport_binds,
                wgpu::MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                render_targets,
            ),
            vertices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::VERTEX),
            indices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::INDEX),
        }
    }

    #[inline(always)]
    pub fn insert_mesh(
        &mut self,
        ctx: &GraphicsContext,
        mesh: Mesh<BasicShapeVertex>,
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
        new: Mesh<BasicShapeVertex>,
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

    #[inline(always)]
    pub fn swap_pipeline(
        &self,
        render_pass: &mut wgpu::RenderPass,
        viewport_binds: &ViewportBinds,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, viewport_binds.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertices_arena.inner_buffer().slice(..));
        render_pass.set_index_buffer(
            self.indices_arena.inner_buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
    }

    #[inline(always)]
    pub fn render_mesh_alloc(
        &self,
        render_pass: &mut wgpu::RenderPass,
        alloc: &AllocMesh<VertexArenaMarker, IndexArenaMarker>,
    ) {
        render_pass.draw_indexed(
            (alloc.indices.byte_index() / 4) as u32
                ..((alloc.indices.byte_index() + alloc.indices.len()) / 4) as u32,
            (alloc.vertices.byte_index() / size_of::<BasicShapeVertex>()) as i32,
            0..1,
        );
    }
}
