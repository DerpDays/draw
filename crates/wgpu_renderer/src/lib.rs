use std::{cell::Cell, sync::Arc};

use atlas::{
    AllocatedTexture,
    LayeredAtlas,
    formats::{Mask, Rgba8},
};
use color::{LinearSrgb, PremulColor};
use graphics::primitives::AvailableSpace;
use parley::{
    FontContext,
    Layout,
    LayoutContext,
    swash::scale::{ScaleContext, image::Image},
};

use crate::buffer::GrowableBuffer;

pub mod arena;
pub mod buffer;
pub mod primitives;

#[cfg(feature = "gui")]
pub mod shaders;

#[derive(Debug, PartialEq, Clone)]
pub struct TextLayoutKey {
    pub text: String,
    pub available_space_width: AvailableSpace,
    pub options: graphics::primitives::TextLayoutOptions,
}

#[derive(Clone)]
pub struct TextLayoutCache {
    pub key: TextLayoutKey,
    pub layout: Layout<ColorBrush>,
}
#[derive(Clone, Default)]
pub struct PrimitiveCache {
    pub mask_textures: Vec<Arc<AllocatedTexture<Mask, TextureData>>>,
    pub color_textures: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>>,
    pub text_layout: Option<TextLayoutCache>,
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

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }
    pub fn indices(&self) -> &[u32] {
        &self.indices
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

pub struct GraphicsContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub texture_state: TextureState,
    pub text_state: TextState,
}

impl GraphicsContext {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            device: device.clone(),
            queue: queue.clone(),

            texture_state: TextureState::new(device),
            text_state: TextState::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorBrush {
    pub color: Cell<PremulColor<LinearSrgb>>,
}

impl Default for ColorBrush {
    fn default() -> Self {
        Self {
            color: Cell::new(color::AlphaColor::WHITE.premultiply()),
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
    Hash(u64),
    // TODO: add cache entry for POD through hashes (e.g. images without metadata).
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct GlyphCacheKey {
    /// Index of the font within [`TextState`]'s [`FontContext`]
    pub font_index: u32,
    /// ID of the glyph within the given font.
    pub glyph_id: u32,
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
        #[cfg(not(target_family = "wasm"))]
        let font_ctx = FontContext::new();
        // Register OpenSans as a default UISansSerif font for wasm since we don't have system
        // fonts.
        #[cfg(target_family = "wasm")]
        let font_ctx = {
            let mut font_ctx = FontContext::new();
            let family = font_ctx.collection.register_fonts(
                parley::fontique::Blob::new(Arc::new(include_bytes!(
                    "../../../resources/fonts/opensans_variable.ttf"
                ))),
                None,
            );
            font_ctx.collection.set_generic_families(
                parley::GenericFamily::UiSansSerif,
                vec![family.first().unwrap().0].into_iter(),
            );
            font_ctx
        };
        Self {
            font_ctx,
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
