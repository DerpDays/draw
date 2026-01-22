#[cfg(feature = "gui")]
use std::collections::HashMap;

use graphics_v2::Primitive;
use gui_v2::{ElementId, GuiRenderer, MeasureCtx};

use crate::{
    GraphicsContext,
    arena::Key,
    primitives::{DrawType, PrimitiveMesh, PrimitiveToMesh, ToDrawType},
    shaders::{basic_shape::BasicShapeState, text::TextState},
};

pub use viewport::{ViewportBinds, ViewportTransform};

mod viewport;

pub mod basic_shape;
pub mod text;
mod texture;

struct CacheEntry {
    previous_elem: graphics_v2::Primitive,
    cache: Option<crate::PrimitiveCache>,
    alloc: Alloc,
}

pub struct AllocMesh<K, I> {
    vertices: Key<K>,
    indices: Key<I>,
}

pub enum Alloc {
    BasicShape(AllocMesh<basic_shape::VertexArenaMarker, basic_shape::IndexArenaMarker>),
    BasicShapeMultisample(AllocMesh<basic_shape::VertexArenaMarker, basic_shape::IndexArenaMarker>),
    Text(AllocMesh<text::VertexArenaMarker, text::IndexArenaMarker>),
    Empty,
}

pub struct WgpuRenderer {
    pub ctx: GraphicsContext,
    // #[cfg(feature = "gui")]
    // pub gui_cache: GuiCache<ElementId>,
    #[cfg(feature = "gui")]
    gui_cache: HashMap<ElementId, CacheEntry>,

    pub viewport: ViewportBinds,

    basic_shapes: basic_shape::BasicShapeState,
    text_state: text::TextState,
    // text: crate::arena::Arena<TextVertexArena>,
    // texture: HashMap<wgpu::Texture, wgpu::BindGroup>,
}

impl WgpuRenderer {
    pub fn new(ctx: GraphicsContext, render_targets: &[Option<wgpu::ColorTargetState>]) -> Self {
        let viewport = ViewportBinds::new(&ctx, ViewportTransform::new(1920., 1080.));
        let basic_shapes = BasicShapeState::new(&ctx, &viewport, render_targets);
        let text_state = TextState::new(&ctx, &viewport, render_targets);
        Self {
            ctx,
            #[cfg(feature = "gui")]
            gui_cache: Default::default(),
            viewport,

            basic_shapes,
            text_state,
            // text,
            // texture: HashMap::default(),
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
    #[profiling::function]
    fn swap_pipeline(&mut self, renderer: &WgpuRenderer, draw_type: DrawType) -> DrawType {
        if Some(draw_type) != self.last_draw_type {
            match draw_type {
                DrawType::BasicShape => {
                    renderer
                        .basic_shapes
                        .swap_pipeline(&mut self.render_pass, &renderer.viewport);
                }
                DrawType::BasicShapeMultisample => todo!(),
                DrawType::Text => {
                    renderer
                        .text_state
                        .swap_pipeline(&mut self.render_pass, &renderer.viewport);
                }
                DrawType::Texture => todo!(),
            }
            self.last_draw_type = Some(draw_type);
        };
        draw_type
    }
}
impl RendererPass<'_> {
    pub fn draw_primitive(&mut self, renderer: &WgpuRenderer, primitive: Primitive) {
        self.swap_pipeline(renderer, primitive.to_draw_type());
    }
    #[cfg(feature = "gui")]
    #[profiling::function]
    pub fn draw_element(&mut self, renderer: &WgpuRenderer, elem: ElementId) {
        let Some(elem) = renderer.gui_cache.get(&elem) else {
            log::warn!("tried to draw a gui element without a cache entry");
            return;
        };

        match &elem.alloc {
            Alloc::BasicShape(alloc) => {
                self.swap_pipeline(renderer, DrawType::BasicShape);
                renderer
                    .basic_shapes
                    .render_mesh_alloc(&mut self.render_pass, alloc);
            }
            Alloc::BasicShapeMultisample(alloc) => {
                self.swap_pipeline(renderer, DrawType::BasicShapeMultisample);
            }
            Alloc::Text(alloc) => {
                self.swap_pipeline(renderer, DrawType::Text);
                renderer
                    .text_state
                    .render_mesh_alloc(&mut self.render_pass, alloc);
            }
            Alloc::Empty => {
                log::warn!("tried to render an empty element");
            }
        }
        // renderer.gui_cache.get()
    }
}

#[cfg(feature = "gui")]
impl GuiRenderer for WgpuRenderer {
    type Renderer = GraphicsContext;

    fn update_cached(&mut self, elem_id: ElementId, primitive: graphics_v2::Primitive) {
        log::info!("updating primitive: {primitive:?}");
        // if already existing
        if let Some(entry) = self.gui_cache.get_mut(&elem_id) {
            if entry.previous_elem != primitive {
                let mesh = primitive.to_mesh(&mut self.ctx, &mut entry.cache);
                'a: {
                    match mesh {
                        PrimitiveMesh::BasicShape(mesh) => {
                            log::info!("mesh is: {:?} for basicshape {primitive:?}", mesh.vertices);
                            let previous_alloc = std::mem::replace(&mut entry.alloc, Alloc::Empty);

                            if mesh.vertices.is_empty() {
                                // if we are converting to an empty allocation, clear the keys from the
                                if let Alloc::BasicShape(alloc) = previous_alloc {
                                    self.basic_shapes.remove_mesh(alloc);
                                }
                                break 'a;
                            }
                            // If the previous allocation was not empty, update the exisiting
                            // allocation
                            if let Alloc::BasicShape(alloc) = previous_alloc {
                                entry.alloc = Alloc::BasicShape(
                                    self.basic_shapes.update_mesh(&self.ctx, alloc, mesh),
                                );
                            // Otherwise if it was previously empty, create a new allocation
                            } else {
                                entry.alloc = Alloc::BasicShape(
                                    self.basic_shapes.insert_mesh(&self.ctx, mesh),
                                );
                            }
                        }
                        PrimitiveMesh::BasicShapeMultisample(_) => todo!(),
                        PrimitiveMesh::Text(mesh) => {
                            log::info!("mesh is: {:?} for text {primitive:?}", mesh.vertices);
                            let previous_alloc = std::mem::replace(&mut entry.alloc, Alloc::Empty);

                            if mesh.vertices.is_empty() {
                                // if we are converting to an empty allocation, clear the keys from the
                                if let Alloc::Text(alloc) = previous_alloc {
                                    self.text_state.remove_mesh(alloc);
                                }
                                break 'a;
                            }
                            // If the previous allocation was not empty, update the exisiting
                            // allocation
                            if let Alloc::Text(alloc) = previous_alloc {
                                entry.alloc = Alloc::Text(
                                    self.text_state.update_mesh(&self.ctx, alloc, mesh),
                                );
                            // Otherwise if it was previously empty, create a new allocation
                            } else {
                                entry.alloc =
                                    Alloc::Text(self.text_state.insert_mesh(&self.ctx, mesh));
                            }
                        }
                        PrimitiveMesh::Texture(_) => todo!(),
                    }
                }

                entry.previous_elem = primitive;
            }
        } else {
            log::info!("inserting primitive into cache: {primitive:?}");
            let mut cache = None;
            let mesh = primitive.to_mesh(&mut self.ctx, &mut cache);
            // log::info!("mesh is: {:?} for primitive {primitive:?}", mesh.vertices);
            let entry = CacheEntry {
                previous_elem: primitive,
                cache,
                alloc: match mesh {
                    PrimitiveMesh::BasicShape(mesh) => {
                        Alloc::BasicShape(self.basic_shapes.insert_mesh(&self.ctx, mesh))
                    }
                    PrimitiveMesh::BasicShapeMultisample(mesh) => {
                        Alloc::BasicShapeMultisample(self.basic_shapes.insert_mesh(&self.ctx, mesh))
                    }
                    PrimitiveMesh::Text(mesh) => {
                        Alloc::Text(self.text_state.insert_mesh(&self.ctx, mesh))
                    }
                    PrimitiveMesh::Texture(mesh) => todo!(),
                },
            };
            self.gui_cache.insert(elem_id, entry);
        }
    }

    fn remove_cached(&mut self, elem_id: ElementId) {
        if let Some(entry) = self.gui_cache.remove(&elem_id) {
            match entry.alloc {
                Alloc::BasicShape(alloc) => {
                    self.basic_shapes.remove_mesh(alloc);
                }
                Alloc::BasicShapeMultisample(alloc) => {
                    self.basic_shapes.remove_mesh(alloc);
                }
                Alloc::Text(alloc) => self.text_state.remove_mesh(alloc),
                Alloc::Empty => {}
            }
        }
    }
}

#[cfg(feature = "gui")]
impl MeasureCtx for WgpuRenderer {
    fn measure_text(
        &mut self,
        text: graphics_v2::primitives::TextMeasure,
    ) -> gui_v2::reexports::taffy::Size<f32> {
        let layout = crate::primitives::prepare_text_layout(
            &mut self.ctx,
            &text.text,
            color::AlphaColor::BLACK,
            &text.text_layout,
            text.max_width.or(match text.available_space_width {
                graphics_v2::primitives::AvailableSpace::Definite(x) => Some(x),
                _ => None,
            }),
        );
        let width = match text.available_space_width {
            graphics_v2::primitives::AvailableSpace::Definite(_) => layout.width(),
            graphics_v2::primitives::AvailableSpace::MinContent => {
                layout.calculate_content_widths().min + 1.
            }
            graphics_v2::primitives::AvailableSpace::MaxContent => {
                layout.calculate_content_widths().max + 1.
            }
        };
        gui_v2::reexports::taffy::Size {
            width,
            height: layout.height(),
        }
    }
}
