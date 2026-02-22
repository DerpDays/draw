use crate::{ElementId, Tree, tree::Node};
use euclid::default::{Box2D, Point2D};
use taffy::{Layout, style::Overflow};

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

    /// Updates absolute positions and clip rects starting from a specific node.
    /// This should be called after we recompute the relative layout e.g.
    /// using [`Tree::compute_root_layout`] or similar.
    pub fn update_abs_subtree(
        &mut self,
        node_id: ElementId,
        parent_abs_location: taffy::Point<f32>,
        parent_clip: Box2D<f32>,
    ) {
        let mut stack = vec![(node_id, parent_abs_location, parent_clip)];

        while let Some((node_id, parent_abs_location, incoming_clip)) = stack.pop() {
            let elem = self.get_mut(node_id);
            let relative_layout = elem.get_relative_final_layout();

            let style = elem.get_style_clone();

            let abs_location = parent_abs_location
                + taffy::Point::<f32> {
                    x: relative_layout.location.x,
                    y: relative_layout.location.y,
                };

            let width = relative_layout.size.width;
            let height = relative_layout.size.height;

            elem.set_abs_layout(Layout {
                location: abs_location,
                ..*relative_layout
            });
            elem.set_clip_rect(incoming_clip);

            let should_clip_children =
                style.overflow.x != Overflow::Visible || style.overflow.y != Overflow::Visible;

            let child_clip = if should_clip_children {
                // TODO: overflow only in the right dimension
                let my_geometry = Box2D::new(
                    Point2D::new(abs_location.x, abs_location.y),
                    Point2D::new(abs_location.x + width, abs_location.y + height),
                );
                incoming_clip
                    .intersection(&my_geometry)
                    .unwrap_or_else(Box2D::zero)
            } else {
                // If overflow is visible, children are NOT confined to this element's geometry.
                // They are only confined to the nearest ancestor that WASN'T visible.
                incoming_clip
            };

            // 5. Recurse
            // We iterate children in default order. Z-index sorting happens later in rendering.
            for child in self.children(node_id).to_vec() {
                stack.push((child, abs_location, child_clip));
            }
        }
    }

    pub fn hit_layout<'a>(&'a self, point: Point2D<f32>) -> impl Iterator<Item = ElementId> + 'a {
        self.render_order
            .render_order()
            .iter()
            .rev()
            .filter_map(move |id| {
                let elem = self.get(*id);
                let abs_layout = elem.get_abs_layout();
                let geometry = box_from_layout(abs_layout);

                // Hit logic:
                // 1. Point must be inside the element's own geometry.
                // 2. Point must be inside the visible region (clip_rect) allowed by ancestors.
                if geometry.contains_inclusive(point)
                    && elem.get_clip_rect().contains_inclusive(point)
                {
                    return Some(*id);
                }
                None
            })
    }
}
