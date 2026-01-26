use crate::{ElementId, Tree, tree::Node};
use euclid::default::{Box2D, Point2D};
use std::collections::HashMap;
use taffy::Layout;

#[derive(Debug, Default)]
pub struct LayoutTree {
    map: HashMap<ElementId, LayoutNode>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct LayoutNode {
    pub node: ElementId,
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

impl Tree {
    pub fn get_abs_layout(&self, node: ElementId) -> Option<LayoutNode> {
        self.layout_tree.map.get(&node).cloned()
    }

    /// Updates absolute positions starting from a specific node.
    /// This should be called after we recompute the relative layout e.g.
    /// using [`Tree::compute_root_layout`] or similar.
    pub fn update_abs_subtree(
        &mut self,
        node_id: ElementId,
        parent_abs_location: taffy::Point<f32>,
    ) {
        let mut stack = vec![(node_id, parent_abs_location)];
        while let Some((node_id, parent_abs_location)) = stack.pop() {
            let elem = self.get(node_id);
            let relative_layout = elem.get_final_layout();

            let abs_location = parent_abs_location
                + taffy::Point::<f32> {
                    x: relative_layout.location.x,
                    y: relative_layout.location.y,
                };

            self.layout_tree.map.insert(
                node_id,
                LayoutNode {
                    node: node_id,
                    abs_layout: Layout {
                        location: abs_location,
                        ..*relative_layout
                    },
                },
            );

            // Add children to the stack
            for child in self.children(node_id).to_vec() {
                stack.push((child, abs_location));
            }
        }
    }

    // clean up removed nodes
    pub fn remove_abs_layout(&mut self, node: ElementId) {
        self.layout_tree.map.remove(&node);
    }

    // Returns an iterator of the hitboxes that the given point is inside, based on render order.
    pub fn hit_layout<'a>(&'a self, point: Point2D<f32>) -> impl Iterator<Item = ElementId> + 'a {
        self.render_order
            .render_order()
            .iter()
            .rev()
            .filter_map(move |id| {
                if let Some(layout_node) = self.layout_tree.map.get(id)
                    && box_from_layout(layout_node.abs_layout).contains_inclusive(point)
                {
                    return Some(*id);
                }
                None
            })
    }
}
