use wgpu::BufferUsages;

/// Owned growable buffer, the underlying buffer is destroyed when dropped.
pub struct GrowableBuffer {
    pub buf: wgpu::Buffer,
    pub len: usize,
    pub capacity: usize,

    usages: BufferUsages,
    label: Option<&'static str>,
}

impl Drop for GrowableBuffer {
    fn drop(&mut self) {
        self.buf.destroy();
    }
}

impl GrowableBuffer {
    #[inline(always)]
    pub fn new(device: &wgpu::Device, usages: BufferUsages, label: Option<&'static str>) -> Self {
        Self::with_capacity(device, usages, 1024, label)
    }
    pub fn with_capacity(
        device: &wgpu::Device,
        usages: BufferUsages,
        capacity: usize,
        label: Option<&'static str>,
    ) -> Self {
        let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
        let capacity =
            ((capacity as u64 + align_mask) & !align_mask).max(wgpu::COPY_BUFFER_ALIGNMENT);

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
            capacity: capacity as usize,
            usages,
            label,
        }
    }

    pub fn replace(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        let len = data.len();
        if len > self.capacity {
            self.grow(
                device,
                queue,
                len.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        queue.write_buffer(&self.buf, 0, data);
        self.len = data.len();
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        offset: usize,
        data: &[u8],
    ) {
        let end = offset + data.len();
        if end > self.capacity {
            self.grow(
                device,
                queue,
                end.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        self.len = self.len.max(end);
        queue.write_buffer(&self.buf, offset as u64, data);
    }

    // appends data to the growable buffer, returns the index where the data was written to.
    pub fn append(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) -> usize {
        let end = self.len + data.len();
        if end > self.capacity {
            self.grow(
                device,
                queue,
                end.next_power_of_two()
                    .max(self.capacity.next_power_of_two()),
            );
        }
        queue.write_buffer(&self.buf, self.len as u64, data);
        let res = self.len;
        self.len = end;
        res
    }

    fn grow(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, new_size: usize) {
        assert!(new_size >= self.capacity, "New buffer must be larger!");
        log::debug!(
            "Growing ({}) buffer to size: {new_size}",
            self.label.unwrap_or("unlabelled")
        );

        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GrowableBuffer (grown)"),
            size: new_size as u64,
            usage: self.usages | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Copy old buffer into new buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GrowableBuffer grow encoder"),
        });
        encoder.copy_buffer_to_buffer(&self.buf, 0, &new_buffer, 0, self.len as u64);
        queue.submit(Some(encoder.finish()));

        self.buf = new_buffer;
        self.capacity = new_size;
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
