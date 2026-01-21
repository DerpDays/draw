use std::{collections::HashMap, hash::Hash};

use graphics_v2::Primitive;

use crate::{
    GraphicsContext,
    PrimitiveCache,
    arena::Key,
    primitives::{DrawType, Render, ToDrawType},
    vertex::Vertex,
};

struct CacheEntry {
    previous_elem: graphics_v2::Primitive,
    cache: Option<PrimitiveCache>,
    alloc: Alloc,
}

pub enum Alloc {
    BasicShape {
        vertices: Key<crate::shaders::BasicShapeVertexArena>,
        indices: Vec<u32>,
    },
    BasicShapeMultisample {
        vertices: Key<crate::shaders::BasicShapeVertexArena>,
        indices: Vec<u32>,
    },
}

#[derive(Default)]
pub struct GuiCache<T: Hash + Eq> {
    hashmap: HashMap<T, CacheEntry>,
}
impl<T: Hash + Eq> GuiCache<T> {
    pub fn insert(&mut self, ctx: &mut GraphicsContext, elem_id: T, primitive: Primitive) {
        log::info!("inserting primitive into cache: {primitive:?}");
        let mut cache = None;
        match primitive.to_draw_type() {
            DrawType::BasicShape => {}
            DrawType::BasicShapeMultisample => todo!(),
            DrawType::Text => todo!(),
            DrawType::Texture => todo!(),
        }
        // let mesh = primitive.to_mesh(ctx, &mut cache);
        log::info!("mesh is: {:?} for primitive {primitive:?}", mesh.vertices);
        // let entry = CacheEntry {
        //     previous_elem: primitive,
        //     cache,
        //     vertex_alloc: ctx.insert(bytemuck::cast_slice(mesh.vertices.as_slice())),
        //     indices: mesh.indices,
        // };
        // self.hashmap.insert(elem_id, entry);
    }
    pub fn update(&mut self, ctx: &mut GraphicsContext, elem_id: T, primitive: Primitive) {
        // log::info!("updating primitive: {primitive:?}");
        // if let Some(entry) = self.hashmap.get_mut(&elem_id) {
        //     if entry.previous_elem != primitive {
        //         let mesh = primitive.to_mesh(ctx, &mut entry.cache);
        //         log::info!("mesh is: {:?} for primitive {primitive:?}", mesh.vertices);
        //         // SAFETY: we replace entry.vertex_alloc straight after
        //         let key = std::mem::replace(&mut entry.vertex_alloc, unsafe { Key::empty_key() });
        //         entry.vertex_alloc = ctx.vertex_arena.update(
        //             &ctx.device,
        //             &ctx.queue,
        //             key,
        //             bytemuck::cast_slice(mesh.vertices.as_slice()),
        //         );
        //         entry.indices = mesh.indices;
        //         entry.previous_elem = primitive;
        //     }
        // } else {
        //     self.insert(ctx, elem_id, primitive);
        // }
    }

    pub fn remove(&mut self, ctx: &mut GraphicsContext, elem_id: T) {
        // if let Some(entry) = self.hashmap.remove(&elem_id) {
        //     ctx.vertex_arena.remove(entry.vertex_alloc);
        // }
    }

    // pub fn render_order(&self, elem_ids: &[T]) -> Vec<u32> {
    //     elem_ids
    //         .iter()
    //         .flat_map(|id| {
    //             let entry = self.hashmap.get(id).expect("failed to get cache entry for element in render order, did you initialise the graphics context?");
    //             entry
    //                 .indices
    //                 .iter()
    //                 .map(|x| *x + (entry.vertex_alloc.byte_index() / size_of::<Vertex>()) as u32)
    //         })
    //         .collect::<Vec<_>>()
    // }
}

pub use crate::shaders::WgpuRenderer;

// pub struct WgpuRenderer {
//     pub ctx: GraphicsContext,
//     pub cache: GuiCache<ElementId>,
// }
// impl GuiRenderer for WgpuRenderer {
//     type Renderer = GraphicsContext;
//
//     fn update_cached(&mut self, elem_id: ElementId, primitive: graphics_v2::Primitive) {
//         self.cache.update(&mut self.ctx, elem_id, primitive);
//     }
//
//     fn remove_cached(&mut self, elem_id: ElementId) {
//         self.cache.remove(&mut self.ctx, elem_id);
//     }
// }
// impl MeasureCtx for WgpuRenderer {
//     fn measure_text(
//         &mut self,
//         text: graphics_v2::primitives::TextMeasure,
//     ) -> gui_v2::reexports::taffy::Size<f32> {
//         let layout = crate::primitives::prepare_text_layout(
//             &mut self.ctx,
//             &text.text,
//             AlphaColor::BLACK,
//             &text.text_layout,
//             text.max_width.or(match text.available_space_width {
//                 graphics_v2::primitives::AvailableSpace::Definite(x) => Some(x),
//                 _ => None,
//             }),
//         );
//         let width = match text.available_space_width {
//             graphics_v2::primitives::AvailableSpace::Definite(_) => layout.width(),
//             graphics_v2::primitives::AvailableSpace::MinContent => {
//                 layout.calculate_content_widths().min + 1.
//             }
//             graphics_v2::primitives::AvailableSpace::MaxContent => {
//                 layout.calculate_content_widths().max + 1.
//             }
//         };
//         gui_v2::reexports::taffy::Size {
//             width,
//             height: layout.height(),
//         }
//     }
// }
