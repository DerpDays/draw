#[cfg(feature = "gui")]
use std::collections::HashMap;

#[cfg(feature = "gui")]
use euclid::default::Box2D;
#[cfg(feature = "gui")]
use gui::{ElementId, GuiRenderer, MeasureCtx};

use crate::{
    GraphicsContext,
    ShapedLayoutCache,
    ShapedLayoutKey,
    TextLayoutKey,
    arena::Key,
    primitives::{DrawType, PrimitiveMesh, PrimitiveToMesh},
    shaders::{generic::GenericRenderer, texture::TextureState},
};

pub use viewport::{ViewportBinds, ViewportTransform};

mod viewport;

pub mod generic;
pub mod texture;

pub struct CacheEntry {
    previous_elem: Option<graphics::Primitive>,
    cache: Option<crate::PrimitiveCache>,
    alloc: Alloc,
}

pub struct AllocMesh<K, I> {
    vertices: Key<K>,
    indices: Key<I>,
}

pub enum Alloc {
    Generic(AllocMesh<generic::VertexArenaMarker, generic::IndexArenaMarker>),
    Texture((Key<texture::VertexArenaMarker>, wgpu::BindGroup)),
    Empty,
}

pub struct WgpuRenderer {
    pub ctx: GraphicsContext,
    #[cfg(feature = "gui")]
    pub gui_cache: HashMap<ElementId, CacheEntry>,

    pub viewport: ViewportBinds,

    pub generic_renderer: GenericRenderer,
    pub texture_state: TextureState,
}

impl WgpuRenderer {
    pub fn new(ctx: GraphicsContext, render_targets: &[Option<wgpu::ColorTargetState>]) -> Self {
        let viewport = ViewportBinds::new(&ctx, ViewportTransform::new(1920., 1080.));
        // let basic_shapes = BasicShapeState::new(&ctx, &viewport, render_targets);
        // let text_state = TextState::new(&ctx, &viewport, render_targets);
        let generic_renderer = GenericRenderer::new(&ctx, &viewport, render_targets);
        let texture_state = TextureState::new(&ctx, &viewport, render_targets);
        Self {
            ctx,
            #[cfg(feature = "gui")]
            gui_cache: Default::default(),
            viewport,

            generic_renderer,
            // basic_shapes,
            // text_state,
            texture_state,
        }
    }

    pub fn begin_render_pass<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        output: wgpu::TextureView,
        ops: wgpu::Operations<wgpu::Color>,
    ) -> RendererPass<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wgpu_renderer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output,
                depth_slice: None,
                resolve_target: None,
                ops,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        RendererPass {
            render_pass,
            last_draw_type: None,
        }
    }

    pub fn submit_encoder(&self, encoder: wgpu::CommandEncoder) {
        self.ctx.queue.submit([encoder.finish()]);
    }
}

pub struct RendererPass<'a> {
    render_pass: wgpu::RenderPass<'a>,
    last_draw_type: Option<DrawType>,
}

impl RendererPass<'_> {
    #[cfg(feature = "gui")]
    #[profiling::function]
    pub fn draw_element(
        &mut self,
        renderer: &WgpuRenderer,
        elem: ElementId,
        scissor_rect: Box2D<u32>,
    ) {
        let Some(elem) = renderer.gui_cache.get(&elem) else {
            return;
        };

        match &elem.alloc {
            Alloc::Generic(alloc) => {
                renderer
                    .generic_renderer
                    .swap_pipeline(&mut self.render_pass, &renderer.viewport);
                self.last_draw_type = Some(DrawType::Generic);

                self.render_pass.set_scissor_rect(
                    scissor_rect.min.x,
                    scissor_rect.min.y,
                    scissor_rect.width(),
                    scissor_rect.height(),
                );
                renderer
                    .generic_renderer
                    .render_mesh_alloc(&mut self.render_pass, alloc);
            }
            Alloc::Texture((key, bind_group)) => {
                renderer
                    .texture_state
                    .swap_pipeline(&mut self.render_pass, &renderer.viewport);
                self.last_draw_type = Some(DrawType::Texture);

                self.render_pass.set_scissor_rect(
                    scissor_rect.min.x,
                    scissor_rect.min.y,
                    scissor_rect.width(),
                    scissor_rect.height(),
                );
                renderer
                    .texture_state
                    .render_bind_group(&mut self.render_pass, key, bind_group);
            }
            Alloc::Empty => {
                log::trace!("tried to render an empty element");
            }
        }
    }
}

#[cfg(feature = "gui")]
impl GuiRenderer for WgpuRenderer {
    type Renderer = GraphicsContext;

    #[inline(always)]
    #[profiling::function]
    fn update_cached(&mut self, elem_id: ElementId, primitive: graphics::Primitive) {
        // if already existing
        if let Some(entry) = self.gui_cache.get_mut(&elem_id) {
            if entry.previous_elem.as_ref() != Some(&primitive) {
                log::trace!("updating primitive: {primitive:?}");
                let mesh = primitive.to_mesh(&mut self.ctx, &mut entry.cache);
                match mesh {
                    PrimitiveMesh::Generic(mesh) => {
                        log::trace!("mesh is: {:?} for basicshape {primitive:?}", mesh.vertices);
                        let previous_alloc = std::mem::replace(&mut entry.alloc, Alloc::Empty);

                        if mesh.vertices.is_empty() {
                            // if we are converting to an empty allocation, clear the keys from the
                            if let Alloc::Generic(alloc) = previous_alloc {
                                self.generic_renderer.remove_mesh(alloc);
                            }
                        } else {
                            // If the previous allocation was not empty, update the existing
                            // allocation
                            if let Alloc::Generic(alloc) = previous_alloc {
                                entry.alloc = Alloc::Generic(
                                    self.generic_renderer.update_mesh(&self.ctx, alloc, mesh),
                                );
                            // Otherwise if it was previously empty, create a new allocation
                            } else {
                                entry.alloc = Alloc::Generic(
                                    self.generic_renderer.insert_mesh(&self.ctx, mesh),
                                );
                            }
                        }
                    }
                    PrimitiveMesh::Texture((vertices, bind_group)) => {
                        log::trace!("mesh updating texture primitive");
                        let previous_alloc = std::mem::replace(&mut entry.alloc, Alloc::Empty);

                        // If the previous allocation was not empty, update the existing
                        // allocation
                        if let Alloc::Texture((key, ..)) = previous_alloc {
                            entry.alloc = Alloc::Texture((
                                self.texture_state.update_vertices(&self.ctx, key, vertices),
                                bind_group,
                            ));
                        // Otherwise if it was previously empty, create a new allocation
                        } else {
                            panic!("we cannot have an empty texture primitive cache entry")
                        }
                    }
                }

                entry.previous_elem = Some(primitive);
            }
        } else {
            log::info!("inserting primitive into cache: {primitive:?}");
            let mut cache = None;
            let mesh = primitive.to_mesh(&mut self.ctx, &mut cache);
            let entry = CacheEntry {
                previous_elem: Some(primitive),
                cache,
                alloc: match mesh {
                    PrimitiveMesh::Generic(mesh) => {
                        if mesh.vertices.is_empty() {
                            Alloc::Empty
                        } else {
                            Alloc::Generic(self.generic_renderer.insert_mesh(&self.ctx, mesh))
                        }
                    }
                    PrimitiveMesh::Texture((vertices, bind_group)) => Alloc::Texture((
                        self.texture_state.insert_vertices(&self.ctx, vertices),
                        bind_group,
                    )),
                },
            };
            self.gui_cache.insert(elem_id, entry);
        }
    }

    fn remove_cached(&mut self, elem_id: ElementId) {
        if let Some(entry) = self.gui_cache.remove(&elem_id) {
            match entry.alloc {
                Alloc::Generic(alloc) => {
                    self.generic_renderer.remove_mesh(alloc);
                }
                Alloc::Texture((vertices, ..)) => {
                    self.texture_state.remove_vertices(vertices);
                }
                Alloc::Empty => {}
            }
        }
    }
}

#[cfg(feature = "gui")]
impl MeasureCtx for WgpuRenderer {
    fn measure_text(
        &mut self,
        id: ElementId,
        text: String,
        text_layout: graphics::primitives::TextLayoutOptions,
        _: Option<graphics::primitives::TextSelection>,
        available_space_width: graphics::primitives::AvailableSpace,
        max_width: Option<f32>,
    ) -> gui::prelude::Size<f32> {
        let key = TextLayoutKey {
            text,
            available_space_width: available_space_width.clone(),
            options: text_layout,
        };

        let entry = self.gui_cache.entry(id).or_insert_with(|| CacheEntry {
            previous_elem: None,
            cache: Some(crate::PrimitiveCache::default()),
            alloc: Alloc::Empty,
        });

        // Tier 1: full cache hit (text + options + available_space unchanged)
        if let Some(cache) = &entry.cache
            && let Some(text_cache) = &cache.text_layout
            && text_cache.key == key
        {
            let layout = &entry.cache.as_ref().unwrap().text_layout.as_ref().unwrap().layout;
            let width = match available_space_width {
                graphics::primitives::AvailableSpace::Definite(_) => layout.width().ceil() + 1.,
                graphics::primitives::AvailableSpace::MinContent => {
                    layout.calculate_content_widths().min.ceil() + 1.
                }
                graphics::primitives::AvailableSpace::MaxContent => layout.width().ceil() + 1.,
            };
            return gui::prelude::Size { width, height: layout.height() };
        }

        let shaped_key = ShapedLayoutKey {
            text: key.text.clone(),
            options: key.options.clone(),
        };
        let break_width = max_width.or(match available_space_width {
            graphics::primitives::AvailableSpace::Definite(x) => Some(x),
            _ => None,
        });

        // Tier 2: shaped cache hit (text + options match, only width changed)
        let shaped_hit = entry
            .cache
            .as_ref()
            .and_then(|c| c.shaped_layout.as_ref())
            .map(|s| s.key == shaped_key)
            .unwrap_or(false);

        let layout = if shaped_hit {
            let mut layout = entry
                .cache
                .as_ref()
                .unwrap()
                .shaped_layout
                .as_ref()
                .unwrap()
                .layout
                .clone();
            layout.break_all_lines(break_width);
            layout
        } else {
            // Tier 3: full miss — shape + break
            crate::primitives::build_shaped_layout(
                &mut self.ctx,
                &key.text,
                color::AlphaColor::BLACK,
                &key.options,
                1.,
            )
        };

        // If we did a full miss, store the pre-break shaped layout and then break lines.
        let layout = if !shaped_hit {
            let shaped_cache = ShapedLayoutCache {
                key: shaped_key,
                layout: layout.clone(),
            };
            let mut broken = layout;
            broken.break_all_lines(break_width);
            // Store both caches
            if let Some(cache) = &mut entry.cache {
                cache.shaped_layout = Some(shaped_cache);
            } else {
                entry.cache = Some(crate::PrimitiveCache {
                    shaped_layout: Some(shaped_cache),
                    ..Default::default()
                });
            }
            broken
        } else {
            layout
        };

        let cache_entry = crate::TextLayoutCache { key, layout };
        if let Some(cache) = &mut entry.cache {
            cache.text_layout = Some(cache_entry);
        } else {
            entry.cache = Some(crate::PrimitiveCache {
                text_layout: Some(cache_entry),
                ..Default::default()
            });
        }
        let layout = &entry
            .cache
            .as_ref()
            .unwrap()
            .text_layout
            .as_ref()
            .unwrap()
            .layout;

        let width = match available_space_width {
            graphics::primitives::AvailableSpace::Definite(_) => layout.width().ceil() + 1.,
            graphics::primitives::AvailableSpace::MinContent => {
                layout.calculate_content_widths().min.ceil() + 1.
            }
            graphics::primitives::AvailableSpace::MaxContent => layout.width().ceil() + 1.,
        };

        gui::prelude::Size {
            width,
            height: layout.height(),
        }
    }
}
