use std::{collections::HashMap, hash::Hash};

use graphics_v2::Primitive;

use crate::{
    arena::Key,
    primitives::Render,
    vertex::Vertex,
    GraphicsContext,
    PrimitiveCache,
    VertexArenaMarker,
};

struct CacheEntry {
    previous_elem: graphics_v2::Primitive,
    cache: Option<PrimitiveCache>,
    vertex_alloc: Key<VertexArenaMarker>,
    indices: Vec<u32>,
}

#[derive(Default)]
pub struct GuiCache<T: Hash + Eq> {
    hashmap: HashMap<T, CacheEntry>,
}
impl<T: Hash + Eq> GuiCache<T> {
    pub fn insert(&mut self, ctx: &mut GraphicsContext<Vertex>, elem_id: T, primitive: Primitive) {
        let mut cache = None;
        let mesh = primitive.to_mesh(ctx, &mut cache);
        let entry = CacheEntry {
            previous_elem: primitive,
            cache,
            vertex_alloc: ctx.insert(bytemuck::cast_slice(mesh.vertices.as_slice())),
            indices: mesh.indices,
        };
        self.hashmap.insert(elem_id, entry);
    }
    pub fn update(&mut self, ctx: &mut GraphicsContext<Vertex>, elem_id: T, primitive: Primitive) {
        if let Some(entry) = self.hashmap.get_mut(&elem_id) {
            if entry.previous_elem != primitive {
                let mesh = primitive.to_mesh(ctx, &mut entry.cache);
                // SAFETY: we replace entry.vertex_alloc straight after
                let key = std::mem::replace(&mut entry.vertex_alloc, unsafe { Key::empty_key() });
                entry.vertex_alloc = ctx.vertex_arena.update(
                    &ctx.device,
                    &ctx.queue,
                    key,
                    bytemuck::cast_slice(mesh.vertices.as_slice()),
                );
                entry.indices = mesh.indices;
            }
        } else {
            self.insert(ctx, elem_id, primitive);
        }
    }
    pub fn render_order(&self, elem_ids: &[T]) -> Vec<u32> {
        elem_ids
            .iter()
            .flat_map(|id| {
                let entry = self.hashmap.get(id).expect("failed to get cache entry for element in render order, did you initialise the graphics context?");
                entry
                    .indices
                    .iter()
                    .map(|x| *x + (entry.vertex_alloc.byte_index() / size_of::<Vertex>()) as u32)
            })
            .collect::<Vec<_>>()
    }
}
