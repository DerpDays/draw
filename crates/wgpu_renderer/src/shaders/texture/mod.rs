use euclid::default::Box2D;
use graphics::primitives::CustomPrimitiveImpl;

pub mod gui_element;

#[derive(Clone, Debug, PartialEq)]
pub struct TextureBindGroup(wgpu::BindGroup);
impl From<TextureBindGroup> for wgpu::BindGroup {
    fn from(value: TextureBindGroup) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TexturePrimitive {
    pub bind_group: TextureBindGroup,
    pub area: Box2D<f32>,
}
impl CustomPrimitiveImpl for TexturePrimitive {}

use crate::{
    GraphicsContext,
    arena::{Arena, Key},
    shaders::ViewportBinds,
};

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextureVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

impl TextureVertex {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1=> Float32x2];

    pub const fn buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        debug_assert!(
            size_of::<Self>().is_multiple_of(wgpu::VERTEX_ALIGNMENT as usize),
            "vertex alignment is not aligned to wgpu::VERTEX_ALIGNMENT bytes",
        );
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}

impl TextureVertex {
    pub const fn new_strip_quad(min: [f32; 2], max: [f32; 2]) -> [Self; 4] {
        [
            // bottom-left
            TextureVertex {
                position: [min[0], min[1]],
                uv: [0.0, 0.0],
            },
            // top-left
            TextureVertex {
                position: [min[0], max[1]],
                uv: [0.0, 1.0],
            },
            // bottom-right
            TextureVertex {
                position: [max[0], min[1]],
                uv: [1.0, 0.0],
            },
            // top-right
            TextureVertex {
                position: [max[0], max[1]],
                uv: [1.0, 1.0],
            },
        ]
    }
}

pub struct VertexArenaMarker;
pub struct TextureState {
    pub pipeline: wgpu::RenderPipeline,
    pub sampler_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,

    pub sampler_bind_group: wgpu::BindGroup,

    pub vertices_arena: Arena<VertexArenaMarker>,
}
impl TextureState {
    fn create_sampler_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture - sampler"),
            entries: &[
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
    fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture - texture"),
            entries: &[
                // texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        })
    }
    fn create_sampler_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture - sampler"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                    &wgpu::SamplerDescriptor {
                        label: Some("texture"),
                        compare: None,
                        anisotropy_clamp: 1,
                        ..Default::default()
                    },
                )),
            }],
        })
    }
    fn create_pipeline(
        device: &wgpu::Device,
        viewport: &ViewportBinds,
        sampler_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> wgpu::RenderPipeline {
        let module = device.create_shader_module(wgpu::include_wgsl!("./shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text"),
            bind_group_layouts: &[
                viewport.bind_group_layout(),
                sampler_bind_group_layout,
                texture_bind_group_layout,
            ],
            immediate_size: 0,
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[TextureVertex::buffer_layout()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
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
impl TextureState {
    pub fn new(
        ctx: &GraphicsContext,
        viewport_binds: &ViewportBinds,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        let sampler_bind_group_layout = Self::create_sampler_bind_group_layout(&ctx.device);
        let texture_bind_group_layout = Self::create_texture_bind_group_layout(&ctx.device);

        let sampler_bind_group =
            Self::create_sampler_bind_group(&ctx.device, &sampler_bind_group_layout);
        Self {
            pipeline: Self::create_pipeline(
                &ctx.device,
                viewport_binds,
                &sampler_bind_group_layout,
                &texture_bind_group_layout,
                render_targets,
            ),
            sampler_bind_group_layout,
            texture_bind_group_layout,

            sampler_bind_group,

            vertices_arena: Arena::new(&ctx.device, wgpu::BufferUsages::VERTEX),
        }
    }

    pub fn create_texture_bind_group(
        &self,
        ctx: &GraphicsContext,
        texture: wgpu::TextureView,
    ) -> TextureBindGroup {
        // self.self.texture_bind_group_layout
        TextureBindGroup(ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture - texture"),
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture),
            }],
        }))
    }
}

impl TextureState {
    #[inline(always)]
    pub fn insert_vertices(
        &mut self,
        ctx: &GraphicsContext,
        vertices: [TextureVertex; 4],
    ) -> Key<VertexArenaMarker> {
        self.vertices_arena
            .insert(&ctx.device, &ctx.queue, bytemuck::cast_slice(&vertices))
    }

    #[inline(always)]
    pub fn update_vertices(
        &mut self,
        ctx: &GraphicsContext,
        key: Key<VertexArenaMarker>,
        vertices: [TextureVertex; 4],
    ) -> Key<VertexArenaMarker> {
        self.vertices_arena.update(
            &ctx.device,
            &ctx.queue,
            key,
            bytemuck::cast_slice(&vertices),
        )
    }

    #[inline(always)]
    pub fn remove_vertices(&mut self, key: Key<VertexArenaMarker>) {
        self.vertices_arena.remove(key);
    }
}

impl TextureState {
    #[inline(always)]
    #[profiling::function]
    pub fn swap_pipeline(
        &self,
        render_pass: &mut wgpu::RenderPass,
        viewport_binds: &ViewportBinds,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, viewport_binds.bind_group(), &[]);
        render_pass.set_bind_group(1, &self.sampler_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertices_arena.inner_buffer().slice(..));
    }
    #[inline(always)]
    #[profiling::function]
    pub fn render_bind_group(
        &self,
        render_pass: &mut wgpu::RenderPass,
        key: &Key<VertexArenaMarker>,
        bind_group: &wgpu::BindGroup,
    ) {
        let start_index = (key.byte_index() / size_of::<TextureVertex>()) as u32;
        log::info!("rendering texture alloc {start_index:?}");
        render_pass.set_bind_group(2, bind_group, &[]);
        render_pass.draw(start_index..start_index + 4, 0..1);
    }
}
