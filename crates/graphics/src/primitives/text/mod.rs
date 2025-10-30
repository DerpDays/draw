use std::{marker::PhantomData, sync::Arc};

use color::{AlphaColor, Srgb};
use euclid::default::{Box2D, Point2D, Size2D, Vector2D};
use parley::{
    Alignment,
    AlignmentOptions,
    Cursor,
    FontFamily,
    FontStack,
    Glyph,
    GlyphRun,
    Layout,
    LineHeight,
    PositionedLayoutItem,
    StyleProperty,
    swash::{
        FontRef,
        scale::{Render, Scaler, Source, StrikeWith, image::Content},
        zeno::{Format, Vector},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    ApplyCoordinates,
    Drawable,
    Mesh,
    Systems,
    Vertex,
    VertexKind,
    make_positive_box,
    systems::{CacheKey, ColorBrush, GlyphCacheKey, TextData, TextureData},
};
use atlas::{
    AllocatedTexture,
    AtlasFormat,
    LayeredAtlas,
    UnallocatedTexture,
    formats::{Mask, Rgba8},
};

mod options;

#[derive(Clone, Debug, Default)]
pub struct TextureAtlasKeys {
    mask_glyphs: Vec<Arc<AllocatedTexture<Mask, TextureData>>>,
    color_glyphs: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>>,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Options {
    pub color: AlphaColor<Srgb>,

    pub font_family: options::FontFamily,
    pub font_size: f32,
    pub font_style: options::FontStyle,
    pub font_weight: options::FontWeight,
    pub font_width: options::FontWidth,
    pub line_height: options::LineHeight,
    pub overflow_wrap: options::OverflowWrap,
    pub whitespace_collapse: options::WhiteSpaceCollapse,
    pub word_break_strength: options::WordBreakStrength,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: AlphaColor::WHITE,

            font_family: parley::FontFamily::Generic(parley::GenericFamily::UiSansSerif).into(),
            font_size: 16.,
            font_style: parley::FontStyle::default().into(),
            font_weight: parley::FontWeight::default().into(),
            font_width: parley::FontWidth::default().into(),
            line_height: parley::LineHeight::default().into(),
            overflow_wrap: parley::OverflowWrap::default().into(),
            whitespace_collapse: parley::WhiteSpaceCollapse::Preserve.into(),
            word_break_strength: parley::WordBreakStrength::default().into(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Text<C: ApplyCoordinates> {
    #[serde(skip)]
    render_cache: Option<Mesh<Vertex>>,

    #[serde(skip)]
    atlas_keys: TextureAtlasKeys,

    #[serde(skip)]
    layout: Option<Layout<ColorBrush>>,
    #[serde(skip)]
    content: String,

    area: Box2D<f32>,

    options: Options,
    _marker: PhantomData<C>,
}
impl<C: ApplyCoordinates> std::fmt::Debug for Text<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text<C>")
            .field("render_cache", &self.render_cache)
            .field("atlas_keys", &self.atlas_keys)
            .field("layout", &self.layout.as_ref().map(|_| "Built layout"))
            .field("content", &self.content)
            .field("area", &self.area)
            .field("options", &self.options)
            .field("_marker", &self._marker)
            .finish()
    }
}

impl<C: ApplyCoordinates> Text<C> {
    pub fn new(content: String, options: Options, area: Box2D<f32>) -> Self {
        let area = make_positive_box(area);
        Self {
            render_cache: None,

            atlas_keys: Default::default(),
            layout: None,

            content,

            area,

            options,
            _marker: PhantomData,
        }
    }

    pub fn content(&self) -> &String {
        &self.content
    }
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.clear_cache();
    }

    pub fn translate(&mut self, dx: Vector2D<f32>) {
        self.area = self.area.translate(dx);
        if let Some(cache) = &mut self.render_cache {
            cache.translate(dx);
        }
    }

    pub fn clear_cache(&mut self) {
        self.render_cache = None;
        self.layout = None;
    }

    pub fn update_rect(&mut self, area: Box2D<f32>) {
        self.area = make_positive_box(area);
        self.clear_cache();
    }
}
impl<C: ApplyCoordinates> Text<C> {
    fn glyph_to_mesh<T: AtlasFormat>(
        area: Box2D<f32>,
        glyph: &Arc<AllocatedTexture<T, TextureData>>,
        atlas: &LayeredAtlas<T, CacheKey, TextureData>,
        kind: VertexKind,
    ) -> Mesh<Vertex> {
        Mesh::from_texture_mesh(glyph.to_mesh(area, atlas), C::apply(kind))
    }

    fn create_layout(&mut self, systems: &mut Systems) -> Layout<ColorBrush> {
        let layout = self
            .layout
            .get_or_insert_with(|| Layout::<ColorBrush>::new());
        if let Some(layout) = self.layout.take() {
            return layout;
        }

        let max_advance = None;

        let mut builder = systems.text.layout_ctx.ranged_builder(
            &mut systems.text.font_ctx,
            &self.content,
            1.25,
            true,
        );

        // Set default text colour styles (set foreground text color)
        let color_brush = ColorBrush {
            color: self.options.color.premultiply(),
        };
        let brush_style = StyleProperty::Brush(color_brush);

        builder.push_default(brush_style);

        builder.push_default(FontStack::Single(self.options.font_family.clone().into()));
        builder.push_default(StyleProperty::FontSize(self.options.font_size));
        builder.push_default(StyleProperty::FontWeight(self.options.font_weight.into()));
        builder.push_default(StyleProperty::FontStyle(self.options.font_style.into()));
        builder.push_default(StyleProperty::FontWidth(self.options.font_width.into()));
        builder.push_default(StyleProperty::LineHeight(self.options.line_height.into()));
        builder.push_default(StyleProperty::OverflowWrap(
            self.options.overflow_wrap.into(),
        ));
        // builder.push_default(self.options.whitespace_collapse.into());
        builder.push_default(StyleProperty::WordBreak(
            self.options.word_break_strength.into(),
        ));

        let mut layout: Layout<ColorBrush> = builder.build(&self.content);
        layout.break_all_lines(max_advance);
        layout.align(max_advance, Alignment::Start, AlignmentOptions::default());
        layout
    }

    fn prepare_layout(&mut self, systems: &mut Systems) -> Layout<ColorBrush> {
        if let Some(layout) = self.layout.take() {
            return layout;
        }

        let max_advance = None;

        let mut builder = systems.text.layout_ctx.ranged_builder(
            &mut systems.text.font_ctx,
            &self.content,
            1.25,
            true,
        );

        // Set default text colour styles (set foreground text color)
        let color_brush = ColorBrush {
            color: self.options.color.premultiply(),
        };
        let brush_style = StyleProperty::Brush(color_brush);
        // let font_stack = FontStack::Single(FontFamily::Generic(parley::GenericFamily::SystemUi));
        let font_stack = FontStack::Single(FontFamily::Named("JetBrainsMono Nerd Font".into()));
        builder.push_default(brush_style);
        builder.push_default(font_stack);
        builder.push_default(LineHeight::FontSizeRelative(1.3));
        builder.push_default(StyleProperty::FontSize(self.options.font_size));
        builder.push_default(StyleProperty::FontWeight(parley::FontWeight::NORMAL));
        builder.push_default(StyleProperty::OverflowWrap(parley::OverflowWrap::BreakWord));

        let mut layout: Layout<ColorBrush> = builder.build(&self.content);
        layout.break_all_lines(max_advance);
        layout.align(max_advance, Alignment::Start, AlignmentOptions::default());
        layout
    }

    pub fn measure(&mut self, systems: &mut Systems) -> Size2D<f32> {
        let layout = self.prepare_layout(systems);
        let res = Size2D::new(layout.full_width(), layout.height());
        self.layout = Some(layout);
        res
    }

    fn render_glyph_run(
        &mut self,
        systems: &mut Systems,
        glyph_run: &GlyphRun<'_, ColorBrush>,
        start_position: Point2D<f32>,
        mesh: &mut Mesh<Vertex>,
    ) {
        // Resolve properties of the GlyphRun
        let mut run_x = glyph_run.offset() + start_position.x;
        let run_y = glyph_run.baseline() + start_position.y;
        let style = glyph_run.style();
        let color = style.brush;

        // Get the "Run" from the "GlyphRun"
        let run = glyph_run.run();

        // Resolve properties of the Run
        let font = run.font();
        let font_size = run.font_size();
        let normalized_coords = run.normalized_coords();

        // Convert from parley::Font to swash::FontRef
        let font_ref = FontRef::from_index(font.data.as_ref(), font.index as usize).unwrap();

        // Build a scaler. As the font properties are constant across an entire run of glyphs
        // we can build one scaler for the run and reuse it for each glyph.
        let scale_context = &mut systems.text.scale_ctx;
        let queue = &systems.queue;
        let device = &systems.device;
        let mask_atlas = &mut systems.texture.mask_atlas;
        let color_atlas = &mut systems.texture.color_atlas;
        let mut scaler = scale_context
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .normalized_coords(normalized_coords)
            .build();

        // Iterates over the glyphs in the GlyphRun
        for glyph in glyph_run.glyphs() {
            let glyph_x = run_x + glyph.x;
            let glyph_y = run_y - glyph.y;
            run_x += glyph.advance;

            self.render_glyph(
                queue,
                device,
                mask_atlas,
                color_atlas,
                mesh,
                &mut scaler,
                font.index,
                font_size,
                color,
                glyph,
                glyph_x,
                glyph_y,
            );
        }

        // Draw decorations: underline & strikethrough
        let run_metrics = run.metrics();
        if let Some(decoration) = &style.underline {
            let offset = decoration.offset.unwrap_or(run_metrics.underline_offset);
            let size = decoration.size.unwrap_or(run_metrics.underline_size);
            Self::render_decoration(
                mesh,
                start_position,
                glyph_run,
                decoration.brush,
                offset,
                size,
            );
        }
        if let Some(decoration) = &style.strikethrough {
            let offset = decoration
                .offset
                .unwrap_or(run_metrics.strikethrough_offset);
            let size = decoration.size.unwrap_or(run_metrics.strikethrough_size);
            Self::render_decoration(
                mesh,
                start_position,
                glyph_run,
                decoration.brush,
                offset,
                size,
            );
        }
    }

    fn render_decoration(
        mesh: &mut Mesh<Vertex>,
        start_position: Point2D<f32>,
        glyph_run: &GlyphRun<'_, ColorBrush>,
        brush: ColorBrush,
        offset: f32,
        width: f32,
    ) {
        let y = glyph_run.baseline() - offset;
        mesh.append(&Mesh::new_color_quad(
            Box2D::new(
                start_position + Size2D::new(glyph_run.offset(), y),
                start_position + Size2D::new(glyph_run.offset() + glyph_run.advance(), y + width),
            ),
            C::apply(VertexKind::Color(brush.color)),
        ));
    }

    fn try_glyph_cache<F: AtlasFormat>(
        mesh: &mut Mesh<Vertex>,
        cache_key: CacheKey,
        atlas: &mut LayeredAtlas<F, CacheKey, TextureData>,
        atlas_keys: &mut Vec<Arc<AllocatedTexture<F, TextureData>>>,
        position: Point2D<f32>,
        vertex_kind: VertexKind,
    ) -> Result<(), ()> {
        let glyph = atlas.is_allocated(cache_key).ok_or(())?;
        let area: Box2D<f32> = match glyph.data {
            TextureData::Text(data) => Box2D::from_origin_and_size(
                Point2D::new(
                    position.x as i32 + data.placement_left,
                    position.y as i32 - data.placement_top,
                )
                .cast(),
                Size2D::new(data.width, data.height).cast(),
            ),
            _ => unreachable!("glyph can only have associated data of type TextureData::Text"),
        };
        mesh.append(&Self::glyph_to_mesh(
            area,
            &glyph,
            &atlas,
            C::apply(vertex_kind),
        ));
        atlas_keys.push(glyph);
        Ok(())
    }

    fn render_glyph(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        mask_atlas: &mut LayeredAtlas<atlas::formats::Mask, CacheKey, TextureData>,
        color_atlas: &mut LayeredAtlas<atlas::formats::Rgba8, CacheKey, TextureData>,
        mesh: &mut Mesh<Vertex>,
        scaler: &mut Scaler<'_>,
        font_index: u32,
        font_size: f32,
        brush: ColorBrush,
        glyph: Glyph,
        glyph_x: f32,
        glyph_y: f32,
    ) -> Result<(), ()> {
        // TODO: try get glyphs from atlas first before rendering

        let cache_key = CacheKey::Text(GlyphCacheKey {
            font_index,
            glyph_id: glyph.id,
            font_size_bits: font_size.to_bits(),
        });
        let position = Point2D::new(glyph_x, glyph_y);

        if let Ok(_) = Self::try_glyph_cache(
            mesh,
            cache_key.clone(),
            mask_atlas,
            &mut self.atlas_keys.mask_glyphs,
            position,
            VertexKind::MaskTexture(brush.color),
        ) {
            return Ok(());
        };
        if let Ok(_) = Self::try_glyph_cache(
            mesh,
            cache_key.clone(),
            color_atlas,
            &mut self.atlas_keys.color_glyphs,
            position,
            VertexKind::ColorTexture,
        ) {
            return Ok(());
        };

        // Compute the fractional offset
        // You'll likely want to quantize this in a real renderer
        let offset = Vector::new(glyph_x.fract(), glyph_y.fract());

        // Render the glyph using swash
        let rendered_glyph = Render::new(
            // Select our source order
            &[
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
            ],
        )
        // Select the simple alpha (non-subpixel) format
        .format(Format::Alpha)
        // Apply the fractional offset
        .offset(offset)
        // Render the image
        .render(scaler, glyph.id)
        .unwrap();

        let glyph_width = rendered_glyph.placement.width;
        let glyph_height = rendered_glyph.placement.height;
        let glyph_x = (glyph_x.floor() as i32 + rendered_glyph.placement.left) as u32;
        let glyph_y = (glyph_y.floor() as i32 - rendered_glyph.placement.top) as u32;

        let glyph_area = Box2D::from_origin_and_size(
            Point2D::new(glyph_x as f32, glyph_y as f32),
            Size2D::new(glyph_width as f32, glyph_height as f32),
        );

        let texture = UnallocatedTexture::new(
            rendered_glyph.data.as_ref(),
            rendered_glyph.placement.width,
            rendered_glyph.placement.height,
        );

        match rendered_glyph.content {
            Content::SubpixelMask => unimplemented!(),
            Content::Mask => {
                let allocated_glyph = mask_atlas
                    .allocate(
                        &device,
                        &queue,
                        texture,
                        Some(cache_key),
                        TextureData::Text(TextData::from_swash(&rendered_glyph)),
                    )
                    .map_err(|_| ())?;
                mesh.append(&Self::glyph_to_mesh(
                    glyph_area,
                    &allocated_glyph,
                    &mask_atlas,
                    C::apply(VertexKind::MaskTexture(brush.color)),
                ));
                self.atlas_keys.mask_glyphs.push(allocated_glyph);
            }
            Content::Color => {
                let allocated_glyph = color_atlas
                    .allocate(
                        &device,
                        &queue,
                        texture,
                        Some(cache_key),
                        TextureData::Text(TextData::from_swash(&rendered_glyph)),
                    )
                    .map_err(|_| ())?;
                mesh.append(&Self::glyph_to_mesh(
                    glyph_area,
                    &allocated_glyph,
                    &color_atlas,
                    C::apply(VertexKind::ColorTexture),
                ));
                self.atlas_keys.color_glyphs.push(allocated_glyph);
            }
        };
        Ok(())
    }
}

impl<C: ApplyCoordinates> Drawable for Text<C> {
    fn render(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
        if let Some(ref cache) = self.render_cache {
            return cache;
        };
        let start_position = self.area.min.round();

        let layout = self.prepare_layout(systems);

        let cursor = Cursor::from_byte_index(&layout, 3, parley::Affinity::Downstream);

        let mut result: Mesh<Vertex> = Mesh::empty();

        // Reset the atlas keys
        self.atlas_keys = Default::default();

        for line in layout.lines() {
            for item in line.items() {
                match item {
                    PositionedLayoutItem::GlyphRun(glyph_run) => {
                        self.render_glyph_run(systems, &glyph_run, start_position, &mut result);
                    }
                    PositionedLayoutItem::InlineBox(inline_box) => {
                        result.append(&Mesh::new_color_quad(
                            Box2D::new(
                                start_position + Size2D::new(inline_box.x, inline_box.y),
                                start_position
                                    + Size2D::new(
                                        inline_box.x + inline_box.width,
                                        inline_box.y + inline_box.height,
                                    ),
                            ),
                            C::apply(VertexKind::Color(self.options.color.premultiply())),
                        ));
                    }
                }
            }
        }

        self.layout = Some(layout);
        self.render_cache = Some(result.clone());
        self.render_cache.as_ref().unwrap()
    }

    fn bounding_box(&self) -> Box2D<f32> {
        self.area
    }

    fn is_dirty(&self) -> bool {
        self.render_cache.is_none()
    }
}
