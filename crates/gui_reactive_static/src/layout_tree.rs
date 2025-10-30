use std::collections::HashMap;

use euclid::default::{Box2D, Point2D};

use taffy::{Layout, PrintTree};

use crate::{
    tree::{DynNodeId, Node},
    Tree,
};

#[derive(Debug, Default)]
pub struct Linear {
    bounding_boxes: Vec<(DynNodeId, Box2D<f32>)>,
}

#[derive(Debug, Default)]
pub struct LayoutTree {
    map: HashMap<DynNodeId, LayoutNode>,
    bounding: Linear,
}

impl LayoutTree {
    pub fn get_layout(&self, node: DynNodeId) -> Option<LayoutNode> {
        self.map.get(&node).cloned()
    }
}

impl LayoutTree {
    pub fn new<T: Node>(tree: &Tree<T>) -> Self {
        let mut map = HashMap::new();
        let mut stack = vec![(tree.root_node(), taffy::Point::ZERO)];
        let mut bounding_boxes: Vec<(DynNodeId, Box2D<f32>)> = Vec::with_capacity(1000);
        while let Some((node, parent_origin)) = stack.pop() {
            let mut layout = tree.get_final_layout(node);
            layout.location = layout.location + parent_origin;

            // Push children in reverse order to process them first (top-most elements)
            let children = tree.children(node);
            for child in children.iter().rev() {
                stack.push((*child, layout.location));
            }
            map.insert(
                node,
                LayoutNode {
                    node,
                    abs_layout: layout,
                },
            );
        }
        for node in tree.render_order.render_order() {
            bounding_boxes.push((
                node.clone(),
                box_from_layout(map.get(node).unwrap().abs_layout),
            ));
        }
        Self {
            map,
            bounding: Linear { bounding_boxes },
        }
    }
    pub fn hit(&self, point: Point2D<f32>) -> impl Iterator<Item = DynNodeId> {
        self.bounding
            .bounding_boxes
            .iter()
            .rev()
            .filter_map(move |(node, bounding_box)| {
                bounding_box.contains_inclusive(point).then_some(*node)
            })
            .into_iter()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct LayoutNode {
    pub node: DynNodeId,
    pub abs_layout: Layout,
}

#[inline]
pub fn box_from_layout(layout: Layout) -> Box2D<f32> {
    Box2D::new(
        Point2D::new(layout.location.x, layout.location.y),
        Point2D::new(
            layout.location.x + layout.size.width,
            layout.location.y + layout.size.height,
        ),
    )
}
