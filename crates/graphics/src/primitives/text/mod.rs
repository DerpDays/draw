use std::{marker::PhantomData, sync::Arc};

use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Size2D, Vector2D};
use parley::{
    swash::{
        scale::{image::Content, Render, Scaler, Source, StrikeWith},
        zeno::{Format, Vector},
        FontRef,
    },
    Alignment, AlignmentOptions, Cursor, FontFamily, FontStack, Glyph, GlyphRun, Layout,
    LineHeight, PositionedLayoutItem, StyleProperty,
};
use serde::{Deserialize, Serialize};

use crate::{
    make_positive_box,
    systems::{CacheKey, ColorBrush, GlyphCacheKey, TextData, TextureData},
    ApplyCoordinates, Drawable, Mesh, Systems, Vertex, VertexKind,
};
use atlas::{
    formats::{Mask, Rgba8},
    AllocatedTexture, AtlasFormat, LayeredAtlas, UnallocatedTexture,
};

mod options;
pub use options::{FontStretch, FontStyle, FontWeight};

#[derive(Clone, Debug, Default)]
pub struct TextureAtlasKeys {
    mask_glyphs: Vec<Arc<AllocatedTexture<Mask, TextureData>>>,
    color_glyphs: Vec<Arc<AllocatedTexture<Rgba8, TextureData>>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Options {
    pub color: PremulColor<Srgb>,

    pub font_size: f32,
    pub line_height: f32,

    pub font_family: options::FontFamily,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub font_stretch: FontStretch,

    pub alignment: options::Alignment,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            color: PremulColor::new([1., 0., 0., 1.]),
            line_height: 28.,
            font_size: 16.,
            font_family: options::FontFamily::Name("JetBrainsMono Nerd Font".to_string()),
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            font_stretch: FontStretch::Normal,
            alignment: options::Alignment::Left,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
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

    fn glyph_to_mesh<T: AtlasFormat>(
        area: Box2D<f32>,
        glyph: &Arc<AllocatedTexture<T, TextureData>>,
        atlas: &LayeredAtlas<T, CacheKey, TextureData>,
        kind: VertexKind,
    ) -> Mesh<Vertex> {
        Mesh::from_texture_mesh(glyph.to_mesh(area, atlas), C::apply(kind))
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
        self.layout = None;
        self.render_cache = None;
    }

    pub fn clear_cache(&mut self) {
        self.render_cache = None;
    }

    pub fn update_rect(&mut self, area: Box2D<f32>) {
        self.render_cache = None;
        self.layout = None;
        self.area = make_positive_box(area);
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
            color: self.options.color,
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
            tracing::info!("rendering glyph!!!!!");
            tracing::info!("mesh size before: {}", mesh.indices.len());

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
            tracing::info!("mesh size after: {}", mesh.indices.len());
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
    ) -> Option<()> {
        // TODO: try get glyphs from atlas first before rendering

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
        tracing::info!(
            "glyph rendering with area: {glyph_area:?} aka: {glyph_width}x{glyph_height}"
        );
        tracing::info!("glyph data len: {:?}", rendered_glyph.data.len());

        let texture = UnallocatedTexture::new(
            rendered_glyph.data.as_ref(),
            rendered_glyph.placement.width,
            rendered_glyph.placement.height,
        );
        let cache_key = CacheKey::Text(GlyphCacheKey {
            font_index,
            glyph_id: glyph.id,
            font_size_bits: font_size.to_bits(),
        });

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
                    .ok()?;
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
                    .ok()?;
                mesh.append(&Self::glyph_to_mesh(
                    glyph_area,
                    &allocated_glyph,
                    &color_atlas,
                    C::apply(VertexKind::ColorTexture),
                ));
                self.atlas_keys.color_glyphs.push(allocated_glyph);
            }
        };
        Some(())
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

        // Selection
        // TODO: handle completion
        // Cursor

        let geom = cursor.geometry(&layout, 2.);
        let clusters = cursor.visual_clusters(&layout);

        if let Some(clus) = clusters[1] {
            result.append(&Mesh::new_color_quad(
                Box2D::new(
                    start_position + Size2D::new(geom.x0 as f32, geom.y0 as f32),
                    start_position + Size2D::new(geom.x0 as f32 + clus.advance(), geom.y1 as f32),
                ),
                C::apply(VertexKind::Color(PremulColor::new([1., 1., 1., 0.3]))),
            ));
        } else if let Some(clus) = clusters[0] {
            result.append(&Mesh::new_color_quad(
                Box2D::new(
                    start_position + Size2D::new(geom.x0 as f32, geom.y0 as f32),
                    start_position + Size2D::new(geom.x0 as f32 + clus.advance(), geom.y1 as f32),
                ),
                C::apply(VertexKind::Color(PremulColor::new([1., 1., 1., 0.3]))),
            ));
        }

        for line in layout.lines() {
            tracing::info!("render glyph line");
            // Iterate over GlyphRun's within each line
            for item in line.items() {
                tracing::info!("render glyph item");
                match item {
                    PositionedLayoutItem::GlyphRun(glyph_run) => {
                        tracing::info!("render glyph run");
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
                            C::apply(VertexKind::Color(self.options.color)),
                        ));
                    }
                }
            }
        }

        self.layout = Some(layout);
        // Update the stored glyph allocations, so that unused allocations can be dropped
        self.render_cache = Some(result.clone());
        tracing::info!(
            "self is the following: {self:#?}, with area min: {:?}",
            self.area.min
        );
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
