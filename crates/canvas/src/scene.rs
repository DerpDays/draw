use anyhow::Result;
use euclid::default::Point2D;
use graphics::Drawable;
use graphics::Primitive;
use graphics::Systems;
use graphics::Vertex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use graphics::{CanvasCoordinates, Mesh};

type C = CanvasCoordinates;
/// A trait that describes a scene containing shapes, with serialization support.
/// The scene is responsible for shape management, layer ordering, and converting to/from a serializable state.
#[derive(Serialize, Deserialize, Default)]
pub struct Scene {
    nodes: HashMap<u32, Primitive<C>>,
    ordering: Vec<u32>,
    pub next_node_id: u32,
}
// TODO: remove allow(unused)
#[allow(unused)]
impl Scene {
    pub fn new(nodes: HashMap<u32, Primitive<C>>, ordering: Vec<u32>) -> Self {
        Self {
            nodes,
            ordering,
            next_node_id: 0,
        }
    }
    /// Add a new shape to the scene; returns its unique ID.
    pub fn add_node(&mut self, node: Primitive<C>) {
        self.nodes.insert(self.next_node_id, node);
        if !self.ordering.contains(&self.next_node_id) {
            self.ordering.push(self.next_node_id);
        } else {
            unreachable!("next_node_id was not incremented properly!");
        }
        self.next_node_id += 1;
    }

    /// Remove a shape by its ID.
    pub fn remove_node(&mut self, id: u32) {
        _ = self.nodes.remove(&id);
        if let Some(position) = self.ordering.iter().position(|x| *x == id) {
            _ = self.ordering.remove(position);
        };
    }

    /// Update a shape's color.
    pub fn get_node(&self, id: u32) -> Option<&Primitive<C>> {
        self.nodes.get(&id)
    }
    pub fn get_node_mut(&mut self, id: u32) -> Option<&mut Primitive<C>> {
        self.nodes.get_mut(&id)
    }
    pub fn get_node_at_position<'a>(
        &'a mut self,
        point: Point2D<f32>,
    ) -> Option<&'a mut Primitive<C>> {
        for id in self.ordering.iter().rev() {
            if let Some(node) = self.nodes.get(id)
                && node.bounding_box().contains(point)
            {
                return self.get_node_mut(*id);
            }
        }
        None
    }

    pub fn get_node_id_at_position<'a>(&'a mut self, point: Point2D<f32>) -> Option<u32> {
        for id in self.ordering.iter().rev() {
            if let Some(node) = self.nodes.get(id)
                && node.bounding_box().contains(point)
            {
                return Some(*id);
            }
        }
        None
    }

    /// Reorder a shape to a new z-index.
    pub fn node_to_layer(&mut self, id: u32, target_layer: usize) {
        if let Some(pos) = self.ordering.iter().position(|&x| x == id) {
            self.ordering.remove(pos);
            let new_layer = if target_layer >= self.ordering.len() {
                self.ordering.len()
            } else if target_layer > pos {
                target_layer - 1
            } else {
                target_layer
            };
            self.ordering.insert(new_layer, id);
        }
    }

    /// Get shapes in render (layer) order.
    pub fn tessellate(&mut self, systems: &mut Systems) -> Mesh<Vertex> {
        let keys = self.ordering.clone();

        let (vertex_count, index_count) = keys
            .iter()
            .filter_map(|key| {
                self.nodes.get_mut(key).map(|node| {
                    let mesh = node.render(systems);
                    (mesh.vertices.len(), mesh.indices.len())
                })
            })
            .fold((0, 0), |(va, ia), (vc, ic)| (va + vc, ia + ic));

        let mut result = Mesh {
            vertices: Vec::with_capacity(vertex_count),
            indices: Vec::with_capacity(index_count),
        };

        for key in keys {
            if let Some(node) = self.nodes.get_mut(&key) {
                result.append(node.render(systems));
            }
        }

        result
    }

    /// Serialize the scene.
    pub fn serialize(&self) -> Result<String> {
        todo!()
    }

    /// Deserialize a scene.
    pub fn deserialize(serialized: &str) -> Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}
