use std::{marker::PhantomData, sync::Arc};

use atlas::{
    formats::{Mask, Rgba8},
    AllocatedTexture,
    LayeredAtlas,
};
use color::{PremulColor, Srgb};
use parley::{
    swash::scale::{image::Image, ScaleContext},
    FontContext,
    LayoutContext,
};
use wgpu::BufferUsages;

use crate::{
    arena::{Arena, Key},
    buffer::GrowableBuffer,
};

// pub mod pipeline;
pub mod arena;
pub mod buffer;
pub mod primitives;
mod vertex;
pub use vertex::{Vertex, VertexKind};

#[cfg(feature = "gui")]
pub mod gui_cache;

#[derive(Clone)]
pub struct PrimitiveCache {
    pub mask_textures: Vec<Arc<AllocatedTexture<Mask, TextureData>>>,
    pub color_textures: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>>,
}

pub struct Mesh<V> {
    vertices: Vec<V>,
    indices: Vec<u32>,
}

impl<V: Clone> Mesh<V> {
    pub const fn empty() -> Mesh<V> {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn append(&mut self, vertices: &[V], mut indices: Vec<u32>) {
        indices.iter_mut().for_each(|x| {
            *x += self.vertices.len() as u32;
        });
        self.vertices.extend_from_slice(vertices);
        self.indices.extend_from_slice(&indices);
    }
    pub fn append_mesh(&mut self, mut mesh: Mesh<V>) {
        mesh.indices.iter_mut().for_each(|x| {
            *x += self.vertices.len() as u32;
        });
        self.vertices.extend_from_slice(&mesh.vertices);
        self.indices.extend_from_slice(&mesh.indices);
    }
}

slotmap::new_key_type! {
    struct AllocKey;
}

pub struct VertexArenaMarker;

pub struct GraphicsContext<V> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    vertex_arena: Arena<VertexArenaMarker>,
    index_buf: GrowableBuffer,

    pub texture_state: TextureState,
    text_state: TextState,

    vertex_type: PhantomData<V>,
}

impl<V> GraphicsContext<V> {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),

            vertex_arena: Arena::new(device, BufferUsages::VERTEX),
            index_buf: GrowableBuffer::new(device, BufferUsages::INDEX, None),

            texture_state: TextureState::new(device),
            text_state: TextState::default(),

            vertex_type: PhantomData,
        }
    }
    pub fn with_capacity(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertex_capacity: usize,
        index_capacity: usize,
    ) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),

            vertex_arena: Arena::with_capacity(device, BufferUsages::VERTEX, vertex_capacity),
            index_buf: GrowableBuffer::with_capacity(
                device,
                BufferUsages::INDEX,
                index_capacity,
                None,
            ),

            texture_state: TextureState::new(device),
            text_state: TextState::default(),

            vertex_type: PhantomData,
        }
    }
}

impl<V> GraphicsContext<V> {
    pub fn insert(&mut self, data: &[u8]) -> Key<VertexArenaMarker> {
        debug_assert!(
            data.len().is_multiple_of(size_of::<V>()),
            "inserted data does not align to the given vertex size"
        );
        self.vertex_arena.insert(&self.device, &self.queue, data)
    }

    pub fn update(&mut self, key: Key<VertexArenaMarker>, data: &[u8]) -> Key<VertexArenaMarker> {
        debug_assert!(
            data.len().is_multiple_of(size_of::<V>()),
            "inserted data does not align to the given vertex size"
        );
        self.vertex_arena
            .update(&self.device, &self.queue, key, data)
    }
}
impl<V> GraphicsContext<V> {
    pub fn vertex_buf(&self) -> &wgpu::Buffer {
        self.vertex_arena.inner_buffer()
    }
    pub fn indices_buf(&self) -> &wgpu::Buffer {
        &self.index_buf.buf
    }
    pub fn replace_indices(&mut self, indices: &[u32]) {
        self.index_buf
            .replace(&self.device, &self.queue, bytemuck::cast_slice(indices));
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ColorBrush {
    pub color: PremulColor<Srgb>,
}

impl Default for ColorBrush {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 1., 1., 1.]),
        }
    }
}
pub struct TextState {
    pub font_ctx: FontContext,
    pub layout_ctx: LayoutContext<ColorBrush>,
    pub scale_ctx: ScaleContext,
}

pub struct TextureState {
    pub mask_atlas: LayeredAtlas<atlas::formats::Mask, CacheKey, TextureData>,
    pub color_atlas: LayeredAtlas<atlas::formats::Rgba8, CacheKey, TextureData>,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum CacheKey {
    Text(GlyphCacheKey),
    Image(String),
    // TODO: add cache entry for POD through hashes (e.g. images without metadata).
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlyphCacheKey {
    /// Index of the font within [`TextState`]'s [`FontContext`]
    pub font_index: u32,
    /// ID of the glyph within the given font.
    pub glyph_id: u16,
    /// `f32` bits of font size
    pub font_size_bits: u32,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TextureData {
    Text(TextData),
    None,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TextData {
    pub width: u32,
    pub height: u32,
    pub placement_left: i32,
    pub placement_top: i32,
}

impl TextData {
    pub fn from_swash(swash: &Image) -> Self {
        Self {
            width: swash.placement.width,
            height: swash.placement.height,
            placement_left: swash.placement.left,
            placement_top: swash.placement.top,
        }
    }
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font_ctx: FontContext::new(),
            layout_ctx: LayoutContext::new(),
            scale_ctx: ScaleContext::new(),
        }
    }
}

impl TextureState {
    pub fn new(device: &wgpu::Device) -> Self {
        let mask_atlas = LayeredAtlas::new(
            device,
            atlas::DEFAULT_ATLAS_SIZE,
            atlas::DEFAULT_TILE_SIZE,
            device.limits(),
        );
        let color_atlas = LayeredAtlas::new(
            device,
            atlas::DEFAULT_ATLAS_SIZE,
            atlas::DEFAULT_TILE_SIZE,
            device.limits(),
        );
        Self {
            mask_atlas,
            color_atlas,
        }
    }
    pub fn needs_rebinding(&self) -> bool {
        self.mask_atlas.needs_rebinding || self.color_atlas.needs_rebinding
    }

    pub fn mark_bound(&mut self) {
        self.mask_atlas.needs_rebinding = false;
        self.color_atlas.needs_rebinding = false;
    }
}
