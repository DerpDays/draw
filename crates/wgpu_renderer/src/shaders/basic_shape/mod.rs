use graphics_v2::Primitive;

use crate::{GraphicsContext, arena::Arena, buffer::GrowableBuffer, shaders::ViewportBinds};

pub struct BasicShapeVertexArena;

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
            size_of::<Self>()
                .div_exact(wgpu::VERTEX_ALIGNMENT as usize)
                .is_some(),
            "vertex alignment is not aligned to {:?} bytes",
            wgpu::VERTEX_ALIGNMENT
        );
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}

pub struct BasicShapes {
    pub pipeline: wgpu::RenderPipeline,
    pub msaa_pipeline: wgpu::RenderPipeline,

    pub vertex_arena: Arena<BasicShapeVertexArena>,
    staging_index_buf: GrowableBuffer,
}

impl BasicShapes {
    fn create_pipeline(
        ctx: &GraphicsContext,
        viewport: &ViewportBinds,
        multisample: wgpu::MultisampleState,
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
                    compilation_options: Default::default(),
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
                    targets: &[],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}

impl BasicShapes {
    pub fn new(ctx: &GraphicsContext, viewport_binds: &ViewportBinds) -> Self {
        Self {
            pipeline: Self::create_pipeline(ctx, viewport_binds, wgpu::MultisampleState::default()),
            msaa_pipeline: Self::create_pipeline(
                ctx,
                viewport_binds,
                wgpu::MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            ),
            vertex_arena: Arena::new(&ctx.device, wgpu::BufferUsages::VERTEX),
            staging_index_buf: GrowableBuffer::new(
                &ctx.device,
                wgpu::BufferUsages::INDEX,
                Some("basic_shapes - index_buf"),
            ),
        }
    }

    pub fn render_primitive(&self, render_pass: &mut wgpu::RenderPass, _primitive: Primitive) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_arena.inner_buffer().slice(..));
        todo!()
    }
}
