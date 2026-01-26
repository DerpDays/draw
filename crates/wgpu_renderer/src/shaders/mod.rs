#[cfg(feature = "gui")]
use std::collections::HashMap;
use std::marker::PhantomData;

use color::LinearSrgb;
use graphics::Primitive;
#[cfg(feature = "gui")]
use gui::{ElementId, GuiRenderer, MeasureCtx};

use crate::{
    GraphicsContext,
    RenderColorspace,
    arena::Key,
    primitives::{DrawType, PrimitiveMesh, PrimitiveToMesh, ToDrawType},
    shaders::{basic_shape::BasicShapeState, text::TextState, texture::TextureState},
};

pub use viewport::{ViewportBinds, ViewportTransform};

mod viewport;

pub mod basic_shape;
pub mod text;
pub mod texture;

pub struct CacheEntry {
    previous_elem: graphics::Primitive,
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
    Texture((Key<texture::VertexArenaMarker>, wgpu::BindGroup)),
    Empty,
}

pub struct WgpuRenderer<CS: RenderColorspace = LinearSrgb> {
    pub ctx: GraphicsContext<CS>,
    // #[cfg(feature = "gui")]
    // pub gui_cache: GuiCache<ElementId>,
    #[cfg(feature = "gui")]
    pub gui_cache: HashMap<ElementId, CacheEntry>,

    pub viewport: ViewportBinds,

    pub basic_shapes: BasicShapeState<CS>,
    pub text_state: TextState<CS>,
    pub texture_state: TextureState<CS>,
    // text: crate::arena::Arena<TextVertexArena>,
    // texture: HashMap<wgpu::Texture, wgpu::BindGroup>,
    _marker: PhantomData<CS>,
}

impl<CS: RenderColorspace> WgpuRenderer<CS> {
    pub fn new(
        ctx: GraphicsContext<CS>,
        render_targets: &[Option<wgpu::ColorTargetState>],
    ) -> Self {
        let viewport = ViewportBinds::new(&ctx, ViewportTransform::new(1920., 1080.));
        let basic_shapes = BasicShapeState::new(&ctx, &viewport, render_targets);
        let text_state = TextState::new(&ctx, &viewport, render_targets);
        let texture_state = TextureState::new(&ctx, &viewport, render_targets);
        Self {
            ctx,
            #[cfg(feature = "gui")]
            gui_cache: Default::default(),
            viewport,

            basic_shapes,
            text_state,
            texture_state,

            _marker: Default::default(),
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
                DrawType::Texture => {
                    renderer
                        .texture_state
                        .swap_pipeline(&mut self.render_pass, &renderer.viewport);
                }
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
            Alloc::Texture((key, bind_group)) => {
                self.swap_pipeline(renderer, DrawType::Texture);
                renderer
                    .texture_state
                    .render_bind_group(&mut self.render_pass, key, bind_group);
            }
            Alloc::Empty => {
                log::warn!("tried to render an empty element");
            }
        }
        // renderer.gui_cache.get()
    }
}

#[cfg(feature = "gui")]
impl<CS: RenderColorspace> GuiRenderer for WgpuRenderer<CS> {
    type Renderer = GraphicsContext<CS>;

    #[inline(always)]
    #[profiling::function]
    fn update_cached(&mut self, elem_id: ElementId, primitive: graphics::Primitive) {
        // if already existing
        if let Some(entry) = self.gui_cache.get_mut(&elem_id) {
            if entry.previous_elem != primitive {
                log::info!("updating primitive: {primitive:?}");
                let mesh = primitive.to_mesh::<CS>(&mut self.ctx, &mut entry.cache);
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
                            // If the previous allocation was not empty, update the existing
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
                            // If the previous allocation was not empty, update the existing
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
                        PrimitiveMesh::Texture((vertices, bind_group)) => {
                            log::info!("mesh updating texture primitive");
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
                }

                entry.previous_elem = primitive;
            }
        } else {
            log::info!("inserting primitive into cache: {primitive:?}");
            let mut cache = None;
            let mesh = primitive.to_mesh::<CS>(&mut self.ctx, &mut cache);
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
                        if mesh.vertices.is_empty() {
                            Alloc::Empty
                        } else {
                            Alloc::Text(self.text_state.insert_mesh(&self.ctx, mesh))
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
                Alloc::BasicShape(alloc) => {
                    self.basic_shapes.remove_mesh(alloc);
                }
                Alloc::BasicShapeMultisample(alloc) => {
                    self.basic_shapes.remove_mesh(alloc);
                }
                Alloc::Text(alloc) => self.text_state.remove_mesh(alloc),
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
        text: graphics::primitives::TextMeasure,
    ) -> gui::reexports::taffy::Size<f32> {
        let layout = crate::primitives::prepare_text_layout(
            &mut self.ctx,
            &text.text,
            color::AlphaColor::BLACK,
            &text.text_layout,
            text.max_width.or(match text.available_space_width {
                graphics::primitives::AvailableSpace::Definite(x) => Some(x),
                _ => None,
            }),
        );
        let width = match text.available_space_width {
            graphics::primitives::AvailableSpace::Definite(_) => layout.width(),
            graphics::primitives::AvailableSpace::MinContent => {
                layout.calculate_content_widths().min + 1.
            }
            graphics::primitives::AvailableSpace::MaxContent => {
                layout.calculate_content_widths().max + 1.
            }
        };
        gui::reexports::taffy::Size {
            width,
            height: layout.height(),
        }
    }
}
