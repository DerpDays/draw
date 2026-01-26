use std::sync::Arc;
use thiserror::Error;

use atlas::{AllocatedTexture, AtlasFormat, LayeredAtlas, UnallocatedTexture};
use color::{AlphaColor, LinearSrgb, PremulColor, Srgb};
use euclid::default::{Box2D, Point2D, Size2D};
use graphics::{
    make_positive_box,
    primitives::{Text, TextLayoutOptions},
};
use parley::{
    Alignment,
    AlignmentOptions,
    FontStack,
    Glyph,
    GlyphRun,
    Layout,
    PositionedLayoutItem,
    StyleProperty,
};
use swash::{
    FontRef,
    scale::{Render, Scaler, Source, StrikeWith, image::Content},
    zeno::{Format, Vector},
};

use crate::{
    CacheKey,
    ColorBrush,
    GlyphCacheKey,
    GraphicsContext,
    Mesh,
    PrimitiveCache,
    TextData,
    TextureData,
    TextureState,
    shaders::text::{TextVertex, VertexKind},
};

pub fn render_text(
    ctx: &mut GraphicsContext,
    text: &Text,
    cache: &mut Option<PrimitiveCache>,
) -> Mesh<TextVertex> {
    let area = make_positive_box(Box2D::from_origin_and_size(text.origin, text.size));

    let start_position = area.min.round();

    let layout = prepare_text_layout(
        ctx,
        &text.text,
        text.color,
        &text.text_layout,
        Some(text.size.width),
    );

    // let cursor = Cursor::from_byte_index(&layout, 3, parley::Affinity::Downstream);

    let mut mesh: Mesh<TextVertex> = Mesh::empty();

    // Reset the atlas keys
    let mut new_cache = PrimitiveCache {
        // Assume most text outputs as mask textures, so allocate with that in mind.
        mask_textures: Vec::with_capacity(text.text.len()),
        color_textures: Vec::new(),
    };

    log::info!("rendering text");
    for line in layout.lines() {
        log::info!("rendering line {:?}", line.is_empty());
        for item in line.items() {
            log::info!("rendering item");
            match item {
                PositionedLayoutItem::GlyphRun(glyph_run) => {
                    // renderer.render_glyph_run(&glyph_run, start_position);

                    GlyphRunRenderer::new(
                        &mut mesh,
                        &mut new_cache,
                        ctx,
                        &glyph_run,
                        start_position,
                    )
                    .render();
                }
                PositionedLayoutItem::InlineBox(inline_box) => {
                    mesh.append(
                        &TextVertex::new_solid_rect(
                            [
                                start_position.x + inline_box.x,
                                start_position.y + inline_box.y,
                            ],
                            [
                                start_position.x + inline_box.x + inline_box.width,
                                start_position.y + inline_box.y + inline_box.height,
                            ],
                            text.color.convert().premultiply(),
                        ),
                        vec![0, 1, 2, 0, 2, 3],
                    );
                }
            }
        }
    }

    *cache = Some(new_cache);
    mesh
}

pub fn prepare_text_layout(
    ctx: &mut GraphicsContext,
    text: &String,
    color: AlphaColor<Srgb>,
    options: &TextLayoutOptions,
    max_width: Option<f32>,
) -> Layout<ColorBrush> {
    let mut builder =
        ctx.text_state
            .layout_ctx
            .ranged_builder(&mut ctx.text_state.font_ctx, text, 1.25, true);

    // Set default text colour styles (set foreground text color)
    let color_brush = ColorBrush {
        color: color.convert().premultiply(),
    };
    let brush_style = StyleProperty::Brush(color_brush);
    let font_stack = FontStack::Single(options.font_family.clone().into());
    builder.push_default(brush_style);
    builder.push_default(font_stack);
    builder.push_default(StyleProperty::LineHeight(options.line_height.into()));
    builder.push_default(StyleProperty::FontSize(options.font_size));
    builder.push_default(StyleProperty::FontWeight(options.font_weight.into()));
    builder.push_default(StyleProperty::OverflowWrap(options.overflow_wrap.into()));

    let mut layout: Layout<ColorBrush> = builder.build(text);
    layout.break_all_lines(max_width);
    layout.align(max_width, Alignment::Start, AlignmentOptions::default());
    layout
}

#[derive(Debug, Error)]
pub enum GlyphRunError {
    #[error("failed to rasterise glyph using swash")]
    FailedToRasterise,
    #[error("failed to allocate glyph to atlas")]
    AtlasAllocationFailure(String),
}

struct GlyphRunRenderer<'a> {
    mesh: &'a mut Mesh<TextVertex>,
    cache: &'a mut PrimitiveCache,

    scaler: Scaler<'a>,
    texture_state: &'a mut TextureState,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,

    glyph_run: &'a GlyphRun<'a, ColorBrush>,
    start_position: Point2D<f32>,

    font_index: u32,
    font_size: f32,
}

impl<'a> GlyphRunRenderer<'a> {
    pub fn new(
        mesh: &'a mut Mesh<TextVertex>,
        cache: &'a mut PrimitiveCache,
        ctx: &'a mut GraphicsContext,
        // scale_ctx: &'a mut ScaleContext,
        // texture_state: &'a mut ScaleContext,
        glyph_run: &'a GlyphRun<'a, ColorBrush>,
        start_position: Point2D<f32>,
    ) -> Self {
        let &mut GraphicsContext {
            ref device,
            ref queue,
            ref mut texture_state,
            ref mut text_state,
            ..
        }: &mut GraphicsContext = ctx;
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
        let scaler = text_state
            .scale_ctx
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .normalized_coords(normalized_coords)
            .build();
        Self {
            mesh,
            cache,

            scaler,
            texture_state,
            device,
            queue,

            glyph_run,
            start_position,

            font_index: font.index,
            font_size,
        }
    }

    fn render(&mut self) {
        let color = self.glyph_run.style().brush.color;
        log::info!("rendering glyph run {color:?}");
        // Resolve properties of the GlyphRun
        let mut run_x = self.glyph_run.offset() + self.start_position.x;
        let run_y = self.glyph_run.baseline() + self.start_position.y;
        let style = self.glyph_run.style();

        // Iterates over the glyphs in the GlyphRun
        for glyph in self.glyph_run.glyphs() {
            let glyph_x = run_x + glyph.x;
            let glyph_y = run_y - glyph.y;
            run_x += glyph.advance;

            if let Err(e) = self.render_glyph(glyph, glyph_x, glyph_y) {
                log::error!("failed to render glyph in run: {e}");
            };
        }

        // Draw decorations: underline & strikethrough
        let run_metrics = self.glyph_run.run().metrics();
        if let Some(decoration) = &style.underline {
            log::info!("decoration");
            let offset = decoration.offset.unwrap_or(run_metrics.underline_offset);
            let size = decoration.size.unwrap_or(run_metrics.underline_size);
            self.render_decoration(offset, size);
        }
        if let Some(decoration) = &style.strikethrough {
            log::info!("decoration");
            let offset = decoration
                .offset
                .unwrap_or(run_metrics.strikethrough_offset);
            let size = decoration.size.unwrap_or(run_metrics.strikethrough_size);
            self.render_decoration(offset, size);
        }
    }

    fn glyph_to_mesh<T: AtlasFormat>(
        area: Box2D<f32>,
        glyph: &Arc<AllocatedTexture<T, TextureData>>,
        atlas: &LayeredAtlas<T, CacheKey, TextureData>,
        fill: PremulColor<LinearSrgb>,
        kind: VertexKind,
    ) -> Mesh<TextVertex> {
        let texture_mesh = glyph.to_mesh(area, atlas);
        Mesh {
            vertices: texture_mesh
                .vertices
                .into_iter()
                .map(|vert| TextVertex::from_texture_vertex(vert, fill, kind))
                .collect(),
            indices: texture_mesh.indices,
        }
    }

    fn try_generic_glyph_cache<F: AtlasFormat>(
        mesh: &mut Mesh<TextVertex>,
        cache_key: CacheKey,
        atlas: &mut LayeredAtlas<F, CacheKey, TextureData>,
        atlas_keys: &mut Vec<Arc<AllocatedTexture<F, TextureData>>>,
        position: Point2D<f32>,
        fill: PremulColor<LinearSrgb>,
        vertex_kind: VertexKind,
    ) -> Option<()> {
        let glyph = atlas.is_allocated(cache_key)?;
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
        mesh.append_mesh(Self::glyph_to_mesh(area, &glyph, atlas, fill, vertex_kind));
        atlas_keys.push(glyph);
        Some(())
    }

    fn try_glyph_cache(&mut self, cache_key: CacheKey, position: Point2D<f32>) -> Option<()> {
        let fill = self.glyph_run.style().brush.color;
        if Self::try_generic_glyph_cache(
            self.mesh,
            cache_key.clone(),
            &mut self.texture_state.mask_atlas,
            &mut self.cache.mask_textures,
            position,
            fill,
            VertexKind::MaskTexture,
        )
        .is_none()
        {
            Self::try_generic_glyph_cache(
                self.mesh,
                cache_key,
                &mut self.texture_state.color_atlas,
                &mut self.cache.color_textures,
                position,
                fill,
                VertexKind::ColorTexture,
            )?;
        };
        Some(())
    }

    fn render_glyph(
        &mut self,
        glyph: Glyph,
        glyph_x: f32,
        glyph_y: f32,
    ) -> Result<(), GlyphRunError> {
        // TODO: try get glyphs from atlas first before rendering

        let cache_key = CacheKey::Text(GlyphCacheKey {
            font_index: self.font_index,
            glyph_id: glyph.id,
            font_size_bits: self.font_size.to_bits(),
        });
        let position = Point2D::new(glyph_x, glyph_y);

        if self.try_glyph_cache(cache_key.clone(), position).is_some() {
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
        .render(&mut self.scaler, glyph.id as u16)
        .ok_or(GlyphRunError::FailedToRasterise)?;

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
                let allocated_glyph = self
                    .texture_state
                    .mask_atlas
                    .allocate(
                        self.device,
                        self.queue,
                        texture,
                        Some(cache_key),
                        TextureData::Text(TextData::from_swash(&rendered_glyph)),
                    )
                    .map_err(|e| GlyphRunError::AtlasAllocationFailure(e.to_string()))?;
                self.mesh.append_mesh(Self::glyph_to_mesh(
                    glyph_area,
                    &allocated_glyph,
                    &self.texture_state.mask_atlas,
                    self.glyph_run.style().brush.color,
                    VertexKind::MaskTexture,
                ));
                self.cache.mask_textures.push(allocated_glyph);
            }
            Content::Color => {
                let allocated_glyph = self
                    .texture_state
                    .color_atlas
                    .allocate(
                        self.device,
                        self.queue,
                        texture,
                        Some(cache_key),
                        TextureData::Text(TextData::from_swash(&rendered_glyph)),
                    )
                    .map_err(|e| GlyphRunError::AtlasAllocationFailure(e.to_string()))?;
                self.mesh.append_mesh(Self::glyph_to_mesh(
                    glyph_area,
                    &allocated_glyph,
                    &self.texture_state.color_atlas,
                    self.glyph_run.style().brush.color,
                    VertexKind::ColorTexture,
                ));
                self.cache.color_textures.push(allocated_glyph);
            }
        };
        Ok(())
    }

    fn render_decoration(&mut self, offset: f32, width: f32) {
        let y = self.glyph_run.baseline() - offset;

        self.mesh.append(
            &TextVertex::new_solid_rect(
                [
                    self.start_position.x + self.glyph_run.offset(),
                    self.start_position.y + y,
                ],
                [
                    self.start_position.x + self.glyph_run.offset() + self.glyph_run.advance(),
                    self.start_position.y + y + width,
                ],
                self.glyph_run.style().brush.color,
            ),
            vec![0, 1, 2, 0, 2, 3],
        );
    }
}
