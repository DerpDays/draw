use crate::{ElementId, Tree, tree::Node};
use euclid::default::{Box2D, Point2D};
use taffy::Layout;

#[inline]
pub fn box_from_layout(layout: &Layout) -> Box2D<f32> {
    Box2D::new(
        Point2D::new(layout.location.x, layout.location.y),
        Point2D::new(
            layout.location.x + layout.size.width,
            layout.location.y + layout.size.height,
        ),
    )
}

impl Tree {
    pub fn get_abs_layout(&self, node: ElementId) -> &Layout {
        self.get(node).get_abs_layout()
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
            let elem = self.get_mut(node_id);
            let relative_layout = elem.get_relative_final_layout();

            let abs_location = parent_abs_location
                + taffy::Point::<f32> {
                    x: relative_layout.location.x,
                    y: relative_layout.location.y,
                };

            elem.set_abs_layout(Layout {
                location: abs_location,
                ..*relative_layout
            });

            // Add children to the stack
            for child in self.children(node_id).to_vec() {
                stack.push((child, abs_location));
            }
        }
    }

    // Returns an iterator of the hitboxes that the given point is inside, based on render order.
    pub fn hit_layout<'a>(&'a self, point: Point2D<f32>) -> impl Iterator<Item = ElementId> + 'a {
        self.render_order
            .render_order()
            .iter()
            .rev()
            .filter_map(move |id| {
                if box_from_layout(self.get_abs_layout(*id)).contains_inclusive(point) {
                    return Some(*id);
                }
                None
            })
    }
}
