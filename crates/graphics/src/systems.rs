use atlas::LayeredAtlas;
use color::{PremulColor, Srgb};
use parley::{
    swash::scale::{image::Image, ScaleContext},
    FontContext, LayoutContext,
};

// Contains owned state for systems, with the renderer state is excluded.
pub struct SystemsOwned {
    pub text: TextState,
    pub texture: TextureState,
}

impl<'a> SystemsOwned {
    pub fn new(text: TextState, texture: TextureState) -> Self {
        Self { text, texture }
    }

    pub fn to_ref(&'a mut self, device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Systems<'a> {
        Systems {
            text: &mut self.text,
            texture: &mut self.texture,

            device,
            queue,
        }
    }
}

/// Contains references to useful core state that might be required by elements.
pub struct Systems<'a> {
    pub text: &'a mut TextState,
    pub texture: &'a mut TextureState,

    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CacheKey {
    Text(GlyphCacheKey),
    Image(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphCacheKey {
    /// Index of the font within [`TextState`]'s [`FontContext`]
    pub font_index: u32,
    /// ID of the glyph within the given font.
    pub glyph_id: u16,
    /// `f32` bits of font size
    pub font_size_bits: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TextureData {
    Text(TextData),
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
