use renderer::State;
use wgpu::{
    util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindingResource, Buffer, Device, FilterMode, Sampler, SamplerDescriptor, TextureView,
};

use crate::projection::Projection;

#[derive(Debug)]
pub struct DrawPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layouts: Layouts,
    pub sampler: wgpu::Sampler,
}

#[derive(Debug)]
pub struct Layouts {
    pub projection: wgpu::BindGroupLayout,
    pub texture_atlas: wgpu::BindGroupLayout,
}

#[derive(Debug)]
pub struct Binds {
    pub projection: ProjectionBind,
    pub texture_atlases: wgpu::BindGroup,
}

impl DrawPipeline {
    pub fn new(device: &Device, texture_format: wgpu::TextureFormat) -> DrawPipeline {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/canvas.wgsl"));

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("atlas sampler"),
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0f32,
            lod_max_clamp: 0f32,
            ..Default::default()
        });

        let projection_layout = Layouts::projection_layout(device);
        let texture_atlas_layout = Layouts::texture_atlas_layout(device);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[&projection_layout, &texture_atlas_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Canvas Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[crate::Vertex::buffer_layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            render_pipeline,
            bind_group_layouts: Layouts {
                projection: projection_layout,
                texture_atlas: texture_atlas_layout,
            },
            sampler,
        }
    }
}
impl Layouts {
    pub fn new(device: &Device) -> Self {
        Self {
            projection: Self::texture_atlas_layout(device),
            texture_atlas: Self::texture_atlas_layout(device),
        }
    }

    pub fn projection_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("projection bind group layout"),
        })
    }
    pub fn texture_atlas_layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
            label: Some("texture atlas bind group layout"),
        })
    }
}
impl Layouts {
    pub fn new_projection_group(
        &self,
        device: &Device,
        world_projection: &Buffer,
        viewport_projection: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            layout: &self.projection,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: world_projection.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: viewport_projection.as_entire_binding(),
                },
            ],
            label: Some("projection bind group"),
        })
    }
    pub fn new_texture_atlas_bind_group(
        &self,
        device: &Device,
        mask_atlas: &TextureView,
        color_atlas: &TextureView,
        sampler: &Sampler,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            layout: &self.texture_atlas,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(mask_atlas),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(color_atlas),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
            label: Some("texture atlas bind group"),
        })
    }
}

#[derive(Debug)]
pub struct ProjectionBind {
    world_projection: Buffer,
    viewport_projection: Buffer,
    pub bind_group: BindGroup,
}
pub struct TextureAtlasBind {
    pub bind_group: BindGroup,
}
impl ProjectionBind {
    pub fn new(state: &State, layout: &Layouts, projection: &Projection) -> Self {
        let world_projection = state
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("World Projection Uniform Buffer"),
                contents: &bytemuck::cast_slice(&projection.world_to_uv().to_arrays()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let viewport_projection =
            state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Viewport Projection Uniform Buffer"),
                    contents: &bytemuck::cast_slice(&projection.viewport_to_uv().to_arrays()),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let bind_group =
            layout.new_projection_group(&state.device, &world_projection, &viewport_projection);
        ProjectionBind {
            world_projection,
            viewport_projection,
            bind_group,
        }
    }

    pub fn update_world(&self, state: &State, projection: &Projection) {
        state.queue.write_buffer(
            &self.world_projection,
            0,
            &bytemuck::cast_slice(&projection.world_to_uv().to_arrays()),
        );
    }

    pub fn update_viewport(&self, state: &State, projection: &Projection) {
        self.update_world(state, projection);
        state.queue.write_buffer(
            &self.viewport_projection,
            0,
            &bytemuck::cast_slice(&projection.viewport_to_uv().to_arrays()),
        );
    }
}
