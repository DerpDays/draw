use std::{marker::PhantomData, sync::Arc};

use color::{PremulColor, Srgb};
use cosmic_text::{Attrs, Buffer, Cursor, Metrics, Shaping, SwashContent};
use euclid::default::{Box2D, Point2D, Vector2D};
use lyon::math::{Point, Size};
use serde::{Deserialize, Serialize};

use crate::{
    systems::{CacheKey, TextData, TextureData},
    ApplyCoordinates, Drawable, Mesh, Systems, Vertex, VertexKind,
};
use atlas::{
    formats::{Mask, Rgba8},
    AllocatedTexture, AtlasFormat, LayeredAtlas, UnallocatedTexture,
};

mod options;
pub use options::{Alignment, FontFamily, FontStretch, FontStyle, FontWeight};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Options {
    pub color: PremulColor<Srgb>,

    pub font_size: f32,
    pub line_height: f32,

    pub font_family: FontFamily,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_stretch: FontStretch,

    pub alignment: Alignment,
}

impl Options {
    fn to_attrs(&self) -> Attrs<'_> {
        Attrs::new()
            .family(self.font_family.as_cosmic())
            .weight(cosmic_text::Weight(self.font_weight as u16))
            .style(self.font_style.as_cosmic())
            .stretch(self.font_stretch.as_cosmic())
            .color(cosmic_text::Color(self.color.to_rgba8().to_u32()))
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 0., 0., 1.]),
            line_height: 28.,
            font_size: 24.,
            font_family: FontFamily::Name("JetBrainsMono Nerd Font".to_string()),
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_stretch: FontStretch::Normal,
            alignment: Alignment::Left,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Text<C: ApplyCoordinates> {
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    #[serde(skip)]
    mask_glyphs: Vec<Arc<AllocatedTexture<Mask, TextureData>>>,
    #[serde(skip)]
    color_glyphs: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>>,

    max_width: f32,
    max_height: f32,
    #[serde(skip)]
    buffer: Option<cosmic_text::Buffer>,
    #[serde(skip)]
    needs_redraw: bool,

    content: String,
    #[serde(skip)]
    cursor: Option<Cursor>,

    area: Box2D<f32>,

    options: Options,
    _marker: PhantomData<C>,
}

impl<C: ApplyCoordinates> Text<C> {
    pub fn new(content: String, options: Options, area: Box2D<f32>) -> Self {
        Self {
            render_cache: None,
            mask_glyphs: Vec::new(),
            color_glyphs: Vec::new(),

            max_width: 100.,
            max_height: 100.,
            buffer: None,
            needs_redraw: true,

            content,
            cursor: None,

            area,

            options,
            _marker: PhantomData,
        }
    }

    fn glyph_to_mesh<T: AtlasFormat>(
        start_position: Point2D<f32>,
        glyph: &Arc<AllocatedTexture<T, TextureData>>,
        atlas: &LayeredAtlas<T, CacheKey, TextureData>,
        kind: VertexKind,
    ) -> Mesh<Vertex> {
        let TextureData::Text(data) = &glyph.data else {
            unreachable!("data for a text cache key should be TextureData::Text")
        };
        let mesh = glyph.to_mesh(
            Box2D::from_origin_and_size(
                start_position + Size::new(data.placement_left as f32, -data.placement_top as f32),
                Size::new(data.width as f32, data.height as f32),
            ),
            atlas,
        );
        Mesh::from_texture_mesh(mesh, C::apply(kind))
    }
    pub fn content(&self) -> String {
        self.content.clone()
    }

    pub fn translate(&mut self, dx: Vector2D<f32>) {
        self.area.translate(dx);
        if let Some(cache) = &mut self.render_cache {
            cache.translate(dx);
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.render_cache = None;
    }

    pub fn clear_cache(&mut self) {
        self.render_cache = None;
    }

    pub fn update_rect(&mut self, area: Box2D<f32>) {
        self.render_cache = None;
        self.area = area;
    }
    pub fn measure(&self) {}
}

impl<C: ApplyCoordinates> Drawable for Text<C> {
    fn render(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        };

        let buffer = self.buffer.get_or_insert_with(|| {
            self.needs_redraw = true;
            Buffer::new(
                &mut systems.text.font_system,
                Metrics {
                    font_size: self.options.font_size,
                    line_height: self.options.line_height,
                },
            )
        });

        if self.needs_redraw {
            buffer.set_text(
                &mut systems.text.font_system,
                &self.content,
                &self.options.to_attrs(),
                Shaping::Advanced,
            );
            self.needs_redraw = false;
        }

        let mut mask_glyphs: Vec<Arc<AllocatedTexture<Mask, TextureData>>> = vec![];
        let mut color_glyphs: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>> = vec![];

        let mut result: Mesh<Vertex> = Mesh::empty();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                // Calculate the output area for this glyph
                let start_position = Point::new(
                    self.area.min.x + glyph.x,
                    self.area.min.y + glyph.y + run.line_top + run.line_y,
                );
                let glyph_cache_key = glyph.physical((0., 0.), 1.).cache_key;
                let cache_key = CacheKey::Text(glyph_cache_key);
                // check if the glyph has already been rasterized and placed in a texture atlas
                // if so we should retain the reference to the allocation, and add the glyph to the
                // output mesh.
                if let Some(mask_glyph) = systems.texture.mask_atlas.is_allocated(cache_key.clone())
                {
                    let mesh = Self::glyph_to_mesh(
                        start_position,
                        &mask_glyph,
                        &systems.texture.mask_atlas,
                        VertexKind::MaskTexture(glyph.color_opt.map_or(self.options.color, |x| {
                            PremulColor::from_rgba8(x.r(), x.g(), x.b(), x.a())
                        })),
                    );
                    mask_glyphs.push(mask_glyph);
                    result.append(&mesh);
                    continue;
                }
                if let Some(color_glyph) =
                    systems.texture.color_atlas.is_allocated(cache_key.clone())
                {
                    let mesh = Self::glyph_to_mesh(
                        start_position,
                        &color_glyph,
                        &systems.texture.color_atlas,
                        VertexKind::ColorTexture,
                    );
                    color_glyphs.push(color_glyph);
                    result.append(&mesh);
                    continue;
                }

                // Otherwise if the glyph was not found in a texture atlas, we should add it to

                let Some(swash) = systems
                    .text
                    .swash_cache
                    .get_image(&mut systems.text.font_system, glyph_cache_key)
                else {
                    tracing::warn!("failed to rasterize a glyph!");
                    continue;
                };
                let texture = UnallocatedTexture::new(
                    swash.data.as_ref(),
                    swash.placement.width,
                    swash.placement.height,
                );

                match swash.content {
                    SwashContent::Mask => {
                        let Ok(mask_glyph) = systems.texture.mask_atlas.allocate(
                            &systems.device,
                            &systems.queue,
                            texture,
                            Some(cache_key),
                            TextureData::Text(TextData::from_swash(&swash)),
                        ) else {
                            continue;
                        };
                        tracing::warn!(
                            "coloring glyph with color: {:?}, self.color = {:?}, vertexkind: {:?}",
                            glyph.color_opt.map_or(self.options.color, |x| {
                                PremulColor::from_rgba8(x.r(), x.g(), x.b(), x.a())
                            }),
                            self.options.color,
                            VertexKind::MaskTexture(
                                glyph.color_opt.map_or(self.options.color, |x| {
                                    PremulColor::from_rgba8(x.r(), x.g(), x.b(), x.a())
                                })
                            ),
                        );
                        let mesh = Self::glyph_to_mesh(
                            start_position,
                            &mask_glyph,
                            &systems.texture.mask_atlas,
                            VertexKind::MaskTexture(
                                glyph.color_opt.map_or(self.options.color, |x| {
                                    PremulColor::from_rgba8(x.r(), x.g(), x.b(), x.a())
                                }),
                            ),
                        );
                        mask_glyphs.push(mask_glyph);
                        result.append(&mesh);
                        tracing::warn!("mesh for glyph: {mesh:?}");
                    }
                    SwashContent::Color => {
                        let Ok(color_glyph) = systems.texture.color_atlas.allocate(
                            &systems.device,
                            &systems.queue,
                            texture,
                            Some(cache_key),
                            TextureData::Text(TextData::from_swash(&swash)),
                        ) else {
                            continue;
                        };
                        let mesh = Self::glyph_to_mesh(
                            start_position,
                            &color_glyph,
                            &systems.texture.color_atlas,
                            VertexKind::ColorTexture,
                        );
                        color_glyphs.push(color_glyph);
                        result.append(&mesh);
                    }
                    SwashContent::SubpixelMask => {
                        unimplemented!(
                            "subpixel mask fonts are unimplemented, please use a different font"
                        )
                    }
                }
            }
        }

        if let Some(cursor) = self.cursor {
            if let Some(laid_cursor) = buffer.layout_cursor(&mut systems.text.font_system, cursor) {
                laid_cursor.glyph;
            }
        }

        // Update the stored glyph allocations, so that unused allocations can be dropped
        self.mask_glyphs = mask_glyphs;
        self.color_glyphs = color_glyphs;
        self.render_cache = Some(result.clone());
        self.render_cache.as_ref().unwrap()
    }

    fn bounding_box(&self) -> Box2D<f32> {
        // Box2D::new(
        //     self.center - Size::new(self.max_width / 2., 10.),
        //     self.center + Size::new(self.max_width / 2., 10.),
        // )
        self.area
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}
