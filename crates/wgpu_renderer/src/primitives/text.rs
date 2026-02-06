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
    scale::{Render, ScaleContext, Scaler, Source, StrikeWith, image::Content},
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
    shaders::generic::{Vertex, VertexKind},
};

#[profiling::function]
pub fn render_text(
    ctx: &mut GraphicsContext,
    text: &Text,
    cache: &mut Option<PrimitiveCache>,
) -> Mesh<Vertex> {
    let area = make_positive_box(Box2D::from_origin_and_size(text.origin, text.size));

    let start_position = area.min.round();

    let layout = prepare_text_layout(
        ctx,
        &text.text,
        text.color,
        &text.text_layout,
        Some(text.size.width),
        1.25,
    );

    // let cursor = Cursor::from_byte_index(&layout, 3, parley::Affinity::Downstream);

    let mut mesh: Mesh<Vertex> = Mesh::empty();

    // Reset the atlas keys
    let mut new_cache = PrimitiveCache {
        // Assume most text outputs as mask textures, so allocate with that in mind.
        mask_textures: Vec::with_capacity(text.text.len()),
        color_textures: Vec::new(),
    };

    let &mut GraphicsContext {
        ref device,
        ref queue,
        ref mut texture_state,
        ref mut text_state,
        ..
    }: &mut GraphicsContext = ctx;

    log::trace!("rendering text primitive");
    for line in layout.lines() {
        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let mut renderer = GlyphRunRenderer::new(
                        &mut mesh,
                        &mut new_cache,
                        device,
                        queue,
                        texture_state,
                        &glyph_run,
                        start_position,
                    );
                    renderer.render(&mut text_state.scale_ctx);
                }
                PositionedLayoutItem::InlineBox(inline_box) => {
                    mesh.append(
                        &Vertex::new_solid_rect(
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

#[profiling::function]
pub fn prepare_text_layout(
    ctx: &mut GraphicsContext,
    text: &str,
    color: AlphaColor<Srgb>,
    options: &TextLayoutOptions,
    max_width: Option<f32>,
    scale: f32,
) -> Layout<ColorBrush> {
    let style = parley::TextStyle {
        font_stack: FontStack::Single(options.font_family.clone().into()),
        font_size: options.font_size,
        font_width: options.font_width.into(),
        font_style: options.font_style.into(),
        font_weight: options.font_weight.into(),
        brush: ColorBrush {
            color: color.convert().premultiply(),
        },
        line_height: options.line_height.into(),
        word_break: options.word_break_strength.into(),
        overflow_wrap: options.overflow_wrap.into(),
        ..Default::default()
    };
    let mut builder =
        ctx.text_state
            .layout_ctx
            .tree_builder(&mut ctx.text_state.font_ctx, scale, true, &style);
    builder.set_white_space_mode(options.whitespace_collapse.into());
    builder.push_text(text);

    let mut layout: Layout<ColorBrush> = builder.build().0;
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
    mesh: &'a mut Mesh<Vertex>,
    cache: &'a mut PrimitiveCache,

    // scale_ctx: &'a mut ScaleContext,
    texture_state: &'a mut TextureState,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,

    glyph_run: &'a GlyphRun<'a, ColorBrush>,
    start_position: Point2D<f32>,

    font_index: u32,
    font_size: f32,
}

// 1. Define this helper enum
enum LazyScaler<'a> {
    // We have the context, ready to build a scaler if needed
    Uninitialized {
        ctx: &'a mut swash::scale::ScaleContext,
        font_ref: FontRef<'a>,
        font_size: f32,
        normalized_coords: &'a [i16],
    },
    // We have built the scaler
    Initialized(Box<Scaler<'a>>),
    // A temporary state required for safe memory swapping
    Poisoned,
}

impl<'a> LazyScaler<'a> {
    // Helper to get the scaler, transitioning state if necessary
    #[profiling::function]
    fn get(&mut self) -> &mut Scaler<'a> {
        // If we are still Uninitialized, we must transition
        if let LazyScaler::Uninitialized { .. } = self {
            // 1. Take the context out, leaving "Poisoned" in its place
            //    (This satisfies the borrow checker: we now OWN the reference temporarily)
            let old_state = std::mem::replace(self, LazyScaler::Poisoned);

            if let LazyScaler::Uninitialized {
                ctx,
                font_ref,
                font_size,
                normalized_coords,
            } = old_state
            {
                profiling::scope!("building font scaler");
                // 2. Build the scaler
                let scaler = ctx
                    .builder(font_ref)
                    .size(font_size)
                    .hint(true)
                    .normalized_coords(normalized_coords)
                    .build();

                // 3. Put the new scaler back into self
                *self = LazyScaler::Initialized(Box::new(scaler));
            }
        }

        // Return a mutable reference to the scaler
        match self {
            LazyScaler::Initialized(scaler) => scaler,
            _ => unreachable!("LazyScaler logic error: state should be Initialized"),
        }
    }
}

impl<'a> GlyphRunRenderer<'a> {
    #[profiling::function]
    pub fn new(
        mesh: &'a mut Mesh<Vertex>,
        cache: &'a mut PrimitiveCache,

        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        texture_state: &'a mut TextureState,

        glyph_run: &'a GlyphRun<'a, ColorBrush>,
        start_position: Point2D<f32>,
    ) -> Self {
        let run = glyph_run.run();

        Self {
            mesh,
            cache,

            texture_state,
            device,
            queue,

            glyph_run,
            start_position,

            font_index: run.font().index,
            font_size: run.font_size(),
        }
    }

    #[profiling::function]
    fn render(&mut self, scale_context: &'a mut ScaleContext) {
        log::trace!("rendering glyph run");
        let mut run_x = self.glyph_run.offset() + self.start_position.x;
        let run_y = self.glyph_run.baseline() + self.start_position.y;
        let style = self.glyph_run.style();

        let mut lazy_scaler = LazyScaler::Uninitialized {
            ctx: scale_context,
            font_ref: FontRef::from_index(
                self.glyph_run.run().font().data.as_ref(),
                self.font_index as usize,
            )
            .expect("failed to create font_ref"),
            font_size: self.font_size,
            normalized_coords: self.glyph_run.run().normalized_coords(),
        };

        for glyph in self.glyph_run.glyphs() {
            let glyph_x = run_x + glyph.x;
            let glyph_y = run_y - glyph.y;
            run_x += glyph.advance;

            let cache_key = CacheKey::Text(GlyphCacheKey {
                font_index: self.font_index,
                glyph_id: glyph.id,
                font_size_bits: self.font_size.to_bits(),
            });
            let position = Point2D::new(glyph_x, glyph_y);

            if self.try_glyph_cache(cache_key.clone(), position).is_some() {
                continue;
            }

            if let Err(e) =
                self.rasterize_glyph(glyph, glyph_x, glyph_y, cache_key, &mut lazy_scaler)
            {
                log::error!("failed to render glyph: {e}");
            }
        }

        // Draw decorations: underline & strikethrough
        let run_metrics = self.glyph_run.run().metrics();
        if let Some(decoration) = &style.underline {
            log::trace!("rendering glyph underline");
            let offset = decoration.offset.unwrap_or(run_metrics.underline_offset);
            let size = decoration.size.unwrap_or(run_metrics.underline_size);
            self.render_decoration(offset, size);
        }
        if let Some(decoration) = &style.strikethrough {
            log::trace!("rendering glyph strikethrough");
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
    ) -> Mesh<Vertex> {
        let texture_mesh = glyph.to_mesh(area, atlas);
        Mesh {
            vertices: texture_mesh
                .vertices
                .into_iter()
                .map(|vert| Vertex::from_texture_vertex(vert, fill, kind))
                .collect(),
            indices: texture_mesh.indices,
        }
    }

    #[profiling::function]
    fn try_generic_glyph_cache<F: AtlasFormat>(
        mesh: &mut Mesh<Vertex>,
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

    #[profiling::function]
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

    #[profiling::function]
    fn rasterize_glyph(
        &mut self,
        glyph: Glyph,
        glyph_x: f32,
        glyph_y: f32,
        cache_key: CacheKey,
        lazy_scaler: &mut LazyScaler<'a>,
    ) -> Result<(), GlyphRunError> {
        let offset = Vector::new(glyph_x.fract(), glyph_y.fract());

        let rendered_glyph = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(offset)
        .render(lazy_scaler.get(), glyph.id as u16)
        .ok_or(GlyphRunError::FailedToRasterise)?;

        let glyph_width = rendered_glyph.placement.width;
        let glyph_height = rendered_glyph.placement.height;
        let glyph_x_int = (glyph_x.floor() as i32 + rendered_glyph.placement.left) as u32;
        let glyph_y_int = (glyph_y.floor() as i32 - rendered_glyph.placement.top) as u32;

        let glyph_area = Box2D::from_origin_and_size(
            Point2D::new(glyph_x_int as f32, glyph_y_int as f32),
            Size2D::new(glyph_width as f32, glyph_height as f32),
        );

        let texture = UnallocatedTexture::new(
            rendered_glyph.data.as_ref(),
            rendered_glyph.placement.width,
            rendered_glyph.placement.height,
        );

        match rendered_glyph.content {
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
            _ => unimplemented!(),
        }
        Ok(())
    }

    fn render_decoration(&mut self, offset: f32, width: f32) {
        let y = self.glyph_run.baseline() - offset;

        self.mesh.append(
            &Vertex::new_solid_rect(
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
