use std::marker::PhantomData;

use wgpu::BufferUsages;

use crate::GrowableBuffer;

#[derive(Debug, PartialEq)]
pub struct Key<M> {
    index: usize,
    len: usize,
    _marker: PhantomData<M>,
}

impl<M> Key<M> {
    /// # Safety
    /// You must not use this key to access the arena, this exists purely for convenience of
    /// temporarily swapping/replacing keys since `Key` is neither copy or clone.
    pub const unsafe fn empty_key() -> Self {
        Self {
            index: 0,
            len: 0,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    const fn new(index: usize, len: usize) -> Self {
        Self {
            index,
            len,
            _marker: PhantomData,
        }
    }
    #[inline(always)]
    const fn copy(&self) -> Self {
        Self {
            index: self.index,
            len: self.len,
            _marker: PhantomData,
        }
    }

    pub const fn byte_index(&self) -> usize {
        self.index
    }

    pub const fn len(&self) -> usize {
        self.len
    }
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// A very simple freelist arena that is synced with a [`wgpu::Buffer`].
///
/// Values dropped from the arena will not be wiped from the wgpu buffer,
/// though they can be overwritten by new data.
pub struct Arena<M> {
    allocations: Vec<Key<M>>,
    freelist: Vec<Key<M>>,

    buf: GrowableBuffer,
}

impl<M> Arena<M> {
    pub fn new(device: &wgpu::Device, usages: BufferUsages) -> Self {
        Self {
            allocations: vec![],
            freelist: vec![],
            buf: GrowableBuffer::new(device, usages, None),
        }
    }
    pub fn with_capacity(device: &wgpu::Device, usages: BufferUsages, capacity: usize) -> Self {
        Self {
            allocations: vec![],
            freelist: vec![],
            buf: GrowableBuffer::with_capacity(device, usages, capacity, None),
        }
    }
}
impl<M> Arena<M> {
    pub fn inner_buffer(&self) -> &wgpu::Buffer {
        &self.buf.buf
    }
}

impl<M> Arena<M> {
    pub fn iter_allocations(&self) -> impl Iterator<Item = &Key<M>> {
        self.allocations.iter()
    }
    pub fn insert(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) -> Key<M> {
        log::info!("inserting data into arena {data:?}");
        for (idx, slot) in self.freelist.iter().map(Key::copy).enumerate() {
            if data.len() <= slot.len() {
                if data.len() == slot.len() {
                    self.freelist.remove(idx);
                } else {
                    self.freelist[idx] =
                        Key::new(slot.byte_index() + data.len(), slot.len() - data.len());
                };
                return self.write(device, queue, data, slot.byte_index());
            }
        }
        if let Some(last) = self.allocations.last() {
            self.write(device, queue, data, last.byte_index() + last.len())
        } else {
            self.write(device, queue, data, 0)
        }
    }
    pub fn remove(&mut self, key: Key<M>) {
        self.allocations.remove(
            self.allocations
                .binary_search_by_key(&key.byte_index(), |x| x.byte_index())
                .expect("removed key should be in the allocations"),
        );
        let inserted: Option<usize> = 'inserted: {
            for (idx, item) in self.freelist.iter_mut().enumerate() {
                // if we have passed any possible keys to merge,
                // just insert into the freelist maintaining index order.
                if item.byte_index() >= key.byte_index() + key.len() {
                    self.freelist.insert(idx, key);
                    break 'inserted Some(idx);
                // if the key we're removing starts at the end of an key in the freelist
                // merge them together
                } else if item.byte_index() + item.len() == key.byte_index() {
                    item.len += key.len();
                    break 'inserted Some(idx);
                };
            }
            self.freelist.push(key);
            None
        };
        // if we previously inserted/merged a key (i.e. not pushed onto the end),
        // check if we can merge the key with the next key.
        if let Some(index) = inserted {
            // check if we can also merge the next free key as well!
            let Some(next) = self.freelist.get(index + 1).map(Key::copy) else {
                return;
            };
            let current = self
                .freelist
                .get_mut(index)
                .expect("index should exist as we just modified it");
            if next.byte_index() == current.byte_index() + current.len() {
                current.len += next.len();
                self.freelist.remove(index + 1);
            }
        }
        self.shrink_to_fit()
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: Key<M>,
        data: &[u8],
    ) -> Key<M> {
        if data.len() <= key.len() {
            self.buf.write(device, queue, key.byte_index(), data);
            key
        } else {
            self.remove(key);
            self.insert(device, queue, data)
        }
    }

    pub fn shrink_to_fit(&mut self) {
        todo!()
    }
}

impl<M> Arena<M> {
    fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        offset: usize,
    ) -> Key<M> {
        assert!(
            !data.is_empty(),
            "tried to write empty data, this will result in overlapping keys!"
        );
        self.buf.write(device, queue, offset, data);
        let key = Key::new(offset, data.len());
        match self
            .allocations
            .binary_search_by_key(&key.byte_index(), |x| x.byte_index())
        {
            Ok(_) => unreachable!("an allocation is already present at this index"),
            Err(idx) => self.allocations.insert(idx, key.copy()),
        };
        key
    }
}
