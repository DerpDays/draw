use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::BufferUsages;

use graphics::Mesh;

pub struct State {
    /// this *is not* to be used to request a new device.
    pub adapter: wgpu::Adapter,

    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub texture_format: wgpu::TextureFormat,
}

impl State {
    pub async fn init() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .context("Failed to find an appropriate adapter")?;

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("Failed to create device")?;

        tracing::warn!("wgpu device limits: {:?}", device.limits());

        let texture_format = wgpu::TextureFormat::Bgra8Unorm;

        Ok(State {
            adapter,

            instance,
            device,
            queue,

            texture_format,
        })
    }
}

/// Owned growable buffer, the underlying buffer is destroyed when dropped.
/// TODO: make this actually drop
pub struct GrowableBuffer {
    pub buf: wgpu::Buffer,
    pub len: u64,
    pub capacity: u64,

    usages: BufferUsages,
    label: Option<&'static str>,
}

impl GrowableBuffer {
    pub fn new(
        device: &wgpu::Device,
        usages: BufferUsages,
        capacity: u64,
        label: Option<&'static str>,
    ) -> Self {
        let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
        let capacity = ((capacity + align_mask) & !align_mask).max(wgpu::COPY_BUFFER_ALIGNMENT);

        let usages = usages | BufferUsages::COPY_SRC | BufferUsages::COPY_DST;

        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: capacity,
            usage: usages,
            mapped_at_creation: false,
        });
        Self {
            buf,
            len: 0,
            capacity,

            usages,
            label,
        }
    }
    pub fn replace(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) -> Result<()> {
        let len = data.len() as u64;
        if len > self.capacity {
            self.grow(
                device,
                queue,
                len.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        queue.write_buffer(&self.buf, 0, data);
        self.len = data.len() as u64;
        Ok(())
    }
    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        offset: u64,
        data: &[u8],
    ) -> Result<()> {
        let end = offset + data.len() as u64;
        if end > self.capacity {
            self.grow(
                device,
                queue,
                end.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        self.len = self.len.max(end);
        queue.write_buffer(&self.buf, offset, data);
        Ok(())
    }

    // appends data to the growable buffer, returns the index where the data was written to.
    pub fn append(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) -> Result<u64> {
        let end = self.len + data.len() as u64;
        if end > self.capacity {
            self.grow(
                device,
                queue,
                end.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        queue.write_buffer(&self.buf, self.len, data);
        let res = Ok(self.len);
        self.len = end;
        res
    }

    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, new_size: u64) {
        assert!(new_size >= self.capacity, "New buffer must be larger!");
        tracing::debug!(
            "Growing ({}) buffer to size: {new_size}",
            self.label.unwrap_or("unlabelled")
        );

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrowableBuffer (grown)"),
            size: new_size,
            usage: self.usages | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Copy old buffer into new buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GrowableBuffer grow encoder"),
        });
        encoder.copy_buffer_to_buffer(&self.buf, 0, &new_buffer, 0, self.len);
        queue.submit(Some(encoder.finish()));

        self.buf = new_buffer;
        self.capacity = new_size;
    }

    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

/// Owned mesh buffer, contains an index and vertex buffer, which are *DESTROYED*
/// when *dropped*.
pub struct GrowableMeshBuffer {
    pub vertex: GrowableBuffer,
    pub index: GrowableBuffer,
    pub num_indices: u32,
}
impl GrowableMeshBuffer {
    pub fn new(device: &wgpu::Device, vertex_capacity: u64, index_capacity: u64) -> Self {
        Self {
            vertex: GrowableBuffer::new(
                device,
                BufferUsages::VERTEX,
                vertex_capacity,
                Some("vertex buffer"),
            ),
            index: GrowableBuffer::new(
                device,
                BufferUsages::INDEX,
                index_capacity,
                Some("index buffer"),
            ),
            num_indices: 0,
        }
    }

    pub fn reset_no_wipe(&mut self) {
        self.num_indices = 0;
    }

    pub fn replace_with_mesh<V>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Mesh<V>,
    ) -> Result<()>
    where
        V: Clone + Pod + Zeroable,
    {
        self.vertex.replace(
            device,
            queue,
            bytemuck::cast_slice(mesh.vertices.as_slice()),
        )?;
        self.index
            .replace(device, queue, bytemuck::cast_slice(mesh.indices.as_slice()))?;
        self.num_indices = mesh.indices.len() as u32;
        Ok(())
    }

    pub fn write_mesh<V>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertex_offset: i64,
        indices_offset: i64,
        mut mesh: graphics::Mesh<V>,
    ) -> Result<()>
    where
        V: Clone + Pod + Zeroable,
    {
        let insertion_offset = {
            if vertex_offset.is_negative() {
                self.vertex.append(
                    device,
                    queue,
                    bytemuck::cast_slice(mesh.vertices.as_slice()),
                )?
            } else {
                self.vertex.write(
                    device,
                    queue,
                    vertex_offset as u64,
                    bytemuck::cast_slice(mesh.vertices.as_slice()),
                )?;
                vertex_offset as u64
            }
        };

        if insertion_offset != 0 {
            mesh.offset_indices(insertion_offset as u32);
        }

        if indices_offset.is_negative() {
            self.index
                .append(device, queue, bytemuck::cast_slice(mesh.indices.as_slice()))?;
        } else {
            self.index.write(
                device,
                queue,
                indices_offset as u64,
                bytemuck::cast_slice(mesh.indices.as_slice()),
            )?;
        };
        Ok(())
    }
}

/// Owned mesh buffer, contains an index and vertex buffer, which are *DESTROYED*
/// when *dropped*.
pub struct MeshBuffer {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub index_count: u32,
}

impl Drop for MeshBuffer {
    fn drop(&mut self) {
        self.vertex.destroy();
        self.index.destroy();
    }
}

/// Used to render a specific color to a view
pub fn render_bg(texture_view: &wgpu::TextureView, wgpu_state: &State, color: wgpu::Color) {
    // Renders a GREEN screen
    let mut encoder = wgpu_state
        .device
        .create_command_encoder(&Default::default());
    // Create the renderpass which will clear the screen.
    {
        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    // Submit the command in the queue to execute
    wgpu_state.queue.submit([encoder.finish()]);
}
