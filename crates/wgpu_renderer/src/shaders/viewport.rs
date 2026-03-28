use wgpu::util::DeviceExt;

use crate::GraphicsContext;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportTransform {
    scale: [f32; 2],
    translate: [f32; 2],
}
impl ViewportTransform {
    pub const fn new(width: f32, height: f32) -> Self {
        let scale = [2.0 / width, -2.0 / height];

        let translate = [-1.0 - (0. * scale[0]), 1.0 - (0. * scale[1])];

        Self { scale, translate }
    }
}

pub struct ViewportBinds {
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}
impl ViewportBinds {
    const VIEWPORT_BINDS_SIZE: std::num::NonZero<u64> =
        std::num::NonZero::new(size_of::<ViewportTransform>() as u64).unwrap();
    pub fn new(ctx: &GraphicsContext, viewport: ViewportTransform) -> Self {
        let layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ViewportBinds"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(Self::VIEWPORT_BINDS_SIZE),
                    },
                    count: None,
                }],
            });
        let buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("ViewportBinds"),
                contents: bytemuck::bytes_of(&viewport),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ViewportBinds"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: Some(Self::VIEWPORT_BINDS_SIZE),
                }),
            }],
        });

        Self {
            layout,
            buffer,
            bind_group,
        }
    }

    pub fn update_viewport(&self, queue: &wgpu::Queue, viewport: ViewportTransform) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&viewport));
    }

    pub const fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }
    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
impl ViewportBinds {
    pub fn new_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        viewport: ViewportTransform,
    ) -> (wgpu::BindGroup, wgpu::Buffer) {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ViewportBinds"),
            contents: bytemuck::bytes_of(&viewport),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ViewportBinds"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: Some(Self::VIEWPORT_BINDS_SIZE),
                }),
            }],
        });
        (bind_group, buffer)
    }
}
