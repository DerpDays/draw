#[cfg(feature = "gui")]
use std::collections::HashMap;

use graphics_v2::Primitive;
use gui_v2::{ElementId, GuiRenderer, MeasureCtx};

use crate::{
    GraphicsContext,
    gui::GuiCache,
    primitives::{DrawType, ToDrawType},
};
pub use viewport::{ViewportBinds, ViewportTransform};

mod viewport;

mod basic_shape;
pub use basic_shape::{BasicShapeVertex, BasicShapeVertexArena, BasicShapes};
mod text;
pub use text::TextVertex;
mod texture;

pub struct TextVertexArena;

struct CacheEntry {
    previous_elem: graphics_v2::Primitive,
    cache: Option<crate::PrimitiveCache>,
    alloc: Alloc,
}

pub enum Alloc {
    BasicShape {
        vertices: crate::arena::Key<crate::shaders::BasicShapeVertexArena>,
        indices: Vec<u32>,
    },
    BasicShapeMultisample {
        vertices: crate::arena::Key<crate::shaders::BasicShapeVertexArena>,
        indices: Vec<u32>,
    },
}

pub struct WgpuRenderer {
    pub ctx: GraphicsContext,
    // #[cfg(feature = "gui")]
    // pub gui_cache: GuiCache<ElementId>,
    #[cfg(feature = "gui")]
    gui_cache: HashMap<ElementId, CacheEntry>,

    viewport: ViewportBinds,

    basic_shapes: basic_shape::BasicShapes,
    // text: crate::arena::Arena<TextVertexArena>,
    // texture: HashMap<wgpu::Texture, wgpu::BindGroup>,
}

impl WgpuRenderer {
    pub fn new(ctx: GraphicsContext) -> Self {
        let viewport = ViewportBinds::new(&ctx, ViewportTransform::new(1920., 1080.));
        let basic_shapes = BasicShapes::new(&ctx, &viewport);
        // let text = Arena::new(&ctx.device, wgpu::BufferUsages::all());
        Self {
            ctx,
            #[cfg(feature = "gui")]
            gui_cache: GuiCache::default(),
            viewport,

            basic_shapes,
            // text,
            // texture: HashMap::default(),
        }
    }

    fn begin_render_pass<'a>(
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

    fn submit_encoder(&self, encoder: wgpu::CommandEncoder) {
        self.ctx.queue.submit([encoder.finish()]);
    }
}

struct RendererPass<'a> {
    render_pass: wgpu::RenderPass<'a>,
    last_draw_type: Option<DrawType>,
}

impl RendererPass<'_> {
    fn prepare_render_pass(&mut self, renderer: &WgpuRenderer, primitive: &Primitive) -> DrawType {
        let draw_type = primitive.to_draw_type();
        if Some(draw_type) != self.last_draw_type {
            match draw_type {
                DrawType::BasicShape => {
                    self.render_pass
                        .set_pipeline(&renderer.basic_shapes.pipeline);
                    self.render_pass.set_vertex_buffer(
                        0,
                        renderer.basic_shapes.vertex_arena.inner_buffer().slice(..),
                    );
                }
                DrawType::BasicShapeMultisample => todo!(),
                DrawType::Text => todo!(),
                DrawType::Texture => todo!(),
            }
        };
        draw_type
    }
}
impl RendererPass<'_> {
    pub fn draw_primitive(&mut self, renderer: &WgpuRenderer, primitive: Primitive) {
        self.prepare_render_pass(renderer, &primitive);
    }
    #[cfg(feature = "gui")]
    pub fn draw_element(&mut self, renderer: &WgpuRenderer, elem: ElementId) {
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
            // if entry.previous_elem != primitive {
            //     let mesh = primitive.to_mesh(ctx, &mut entry.cache);
            //     log::info!("mesh is: {:?} for primitive {primitive:?}", mesh.vertices);
            //     // SAFETY: we replace entry.vertex_alloc straight after
            //     let key = std::mem::replace(&mut entry.vertex_alloc, unsafe { Key::empty_key() });
            //     entry.vertex_alloc = ctx.vertex_arena.update(
            //         &ctx.device,
            //         &ctx.queue,
            //         key,
            //         bytemuck::cast_slice(mesh.vertices.as_slice()),
            //     );
            //     entry.indices = mesh.indices;
            //     entry.previous_elem = primitive;
            // }
        } else {
            log::info!("inserting primitive into cache: {primitive:?}");
            // let mut cache = None;
            match primitive.to_draw_type() {
                DrawType::BasicShape => {}
                DrawType::BasicShapeMultisample => todo!(),
                DrawType::Text => todo!(),
                DrawType::Texture => todo!(),
            }
            // let mesh = primitive.to_mesh(ctx, &mut cache);
            // log::info!("mesh is: {:?} for primitive {primitive:?}", mesh.vertices);
            // let entry = CacheEntry {
            //     previous_elem: primitive,
            //     cache,
            //     vertex_alloc: ctx.insert(bytemuck::cast_slice(mesh.vertices.as_slice())),
            //     indices: mesh.indices,
            // };
            // self.hashmap.insert(elem_id, entry);
        }
    }

    fn remove_cached(&mut self, elem_id: ElementId) {
        self.gui_cache.remove(&mut self.ctx, elem_id);
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
