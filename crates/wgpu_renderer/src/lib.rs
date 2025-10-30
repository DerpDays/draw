use std::marker::PhantomData;

use atlas::LayeredAtlas;
use color::{PremulColor, Srgb};
use parley::{
    FontContext,
    LayoutContext,
    swash::scale::{ScaleContext, image::Image},
};
use wgpu::BufferUsages;

use crate::{
    arena::{Arena, Key},
    buffer::GrowableBuffer,
};

// pub mod pipeline;
pub mod arena;
pub mod buffer;
#[cfg(feature = "gui_reactive")]
pub mod gui_cache;

pub mod vertex;

#[derive(Clone, Debug)]
pub struct Mesh<V> {
    vertices: Vec<V>,
    indices: Vec<u32>,
}

slotmap::new_key_type! {
    struct AllocKey;
}

pub struct VertexArenaMarker;

pub struct GraphicsContext<V> {
    vertex_buf: Arena<VertexArenaMarker>,
    index_buf: GrowableBuffer,

    texture_state: TextureState,
    text_state: TextState,

    vertex_type: PhantomData<V>,
}

impl<V> GraphicsContext<V> {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            vertex_buf: Arena::new(device, BufferUsages::VERTEX),
            index_buf: GrowableBuffer::new(device, BufferUsages::INDEX, None),

            texture_state: TextureState::new(device),
            text_state: TextState::default(),

            vertex_type: PhantomData,
        }
    }
    pub fn with_capacity(
        device: &wgpu::Device,
        vertex_capacity: usize,
        index_capacity: usize,
    ) -> Self {
        Self {
            vertex_buf: Arena::with_capacity(device, BufferUsages::VERTEX, vertex_capacity),
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
    pub fn insert(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) -> Key<VertexArenaMarker> {
        self.vertex_buf.insert(device, queue, data)
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: Key<VertexArenaMarker>,
        data: &[u8],
    ) -> Key<VertexArenaMarker> {
        self.vertex_buf.update(device, queue, key, data)
    }

    // this is wrong for now
    #[deprecated]
    pub fn populate_index(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        order: &[&Key<VertexArenaMarker>],
    ) {
        let indices = order
            .iter()
            .flat_map(|k| {
                ((k.index() as u32)..(k.index() + k.len()) as u32).step_by(std::mem::size_of::<V>())
            })
            .collect::<Vec<_>>();
        self.index_buf
            .replace(device, queue, bytemuck::cast_slice(&indices));
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
            &device,
            atlas::DEFAULT_ATLAS_SIZE,
            atlas::DEFAULT_TILE_SIZE,
            device.limits(),
        );
        let color_atlas = LayeredAtlas::new(
            &device,
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
