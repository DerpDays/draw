use std::collections::BTreeMap;

use euclid::default::{Box2D, Point2D};

use rstar::{PointDistance, RTree, RTreeObject, AABB};
use taffy::{Layout, NodeId};

use crate::tree::UITree;

pub struct Spatial {
    rtree: RTree<LayoutNode>,
    render_order: Vec<NodeId>,
}
pub struct Linear {
    bounding_boxes: Vec<(NodeId, Box2D<f32>)>,
}

pub struct LayoutTree<T> {
    map: BTreeMap<u64, LayoutNode>,
    bounding: T,
}

impl<T> LayoutTree<T> {
    pub fn get_layout(&self, node: NodeId) -> Option<LayoutNode> {
        self.map.get(&node.into()).cloned()
    }
}

pub trait HittableLayout {
    fn empty(root: NodeId) -> Self;
    fn new<T>(tree: &UITree<T>) -> Self;
    /// Returns an iterator of NodeId's at a given position in z-index order.
    fn hit(&self, point: Point2D<f32>) -> impl Iterator<Item = NodeId>;
}

impl HittableLayout for LayoutTree<Linear> {
    fn empty(root: NodeId) -> Self {
        let mut map = BTreeMap::new();
        let root_layout_node = LayoutNode {
            node: root,
            abs_layout: Layout::new(),
        };
        map.insert(root.into(), root_layout_node);
        let bounding_boxes = vec![(root, box_from_layout(root_layout_node.abs_layout))];
        Self {
            map,
            bounding: Linear { bounding_boxes },
        }
    }

    fn new<T>(tree: &UITree<T>) -> Self {
        let mut map = BTreeMap::new();
        let mut stack = vec![(tree.root_node(), taffy::Point::ZERO)];
        let mut bounding_boxes: Vec<(NodeId, Box2D<f32>)> =
            Vec::with_capacity(tree.render_order.render_order().len());
        while let Some((node, parent_origin)) = stack.pop() {
            let mut layout = tree.relative_layout(node);
            layout.location = layout.location + parent_origin;

            // Push children in reverse order to process them first (top-most elements)
            let children = tree.children(node);
            for child in children.iter().rev() {
                stack.push((*child, layout.location));
            }
            map.insert(
                node.into(),
                LayoutNode {
                    node,
                    abs_layout: layout,
                },
            );
        }
        for node in tree.render_order.render_order() {
            bounding_boxes.push((
                *node,
                box_from_layout(map.get(&(*node).into()).unwrap().abs_layout),
            ));
        }
        Self {
            map,
            bounding: Linear { bounding_boxes },
        }
    }
    fn hit(&self, point: Point2D<f32>) -> impl Iterator<Item = NodeId> {
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

impl HittableLayout for LayoutTree<Spatial> {
    fn empty(root: NodeId) -> Self {
        let mut map = BTreeMap::new();
        let root_layout_node = LayoutNode {
            node: root,
            abs_layout: Layout::new(),
        };
        map.insert(root.into(), root_layout_node);
        let spatial_tree = RTree::bulk_load(vec![root_layout_node]);
        Self {
            map,
            bounding: Spatial {
                rtree: spatial_tree,
                render_order: vec![root],
            },
        }
    }

    fn new<T>(tree: &UITree<T>) -> Self {
        let mut map = BTreeMap::new();
        let mut stack = vec![(tree.root_node(), taffy::Point::ZERO)];
        let mut bounding_boxes: Vec<(NodeId, Box2D<f32>)> =
            Vec::with_capacity(tree.render_order.render_order().len());
        while let Some((node, parent_origin)) = stack.pop() {
            let mut layout = tree.relative_layout(node);
            layout.location = layout.location + parent_origin;

            // Push children in reverse order to process them first (top-most elements)
            let children = tree.children(node);
            for child in children.iter().rev() {
                stack.push((*child, layout.location));
            }
            map.insert(
                node.into(),
                LayoutNode {
                    node,
                    abs_layout: layout,
                },
            );
        }
        for node in tree.render_order.render_order() {
            bounding_boxes.push((
                *node,
                box_from_layout(map.get(&(*node).into()).unwrap().abs_layout),
            ));
        }
        let spatial_tree = RTree::bulk_load(map.values().cloned().collect());
        Self {
            map,
            bounding: Spatial {
                rtree: spatial_tree,
                render_order: tree.render_order.render_order().clone(),
            },
        }
    }
    fn hit(&self, point: Point2D<f32>) -> impl Iterator<Item = NodeId> {
        let mut nodes = self
            .bounding
            .rtree
            .locate_all_at_point(&point.to_array())
            .collect::<Vec<_>>();
        nodes.sort_unstable_by_key(|x| {
            self.bounding
                .render_order
                .iter()
                .find_map(|item| {
                    if x.node.eq(item) {
                        Some(u64::from(x.node))
                    } else {
                        None
                    }
                })
                .unwrap_or(0)
        });
        nodes.into_iter().rev().map(|x| x.node)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutNode {
    pub node: NodeId,
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

impl RTreeObject for LayoutNode {
    type Envelope = AABB<[f32; 2]>;

    #[inline]
    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.abs_layout.location.x, self.abs_layout.location.y],
            [
                self.abs_layout.location.x + self.abs_layout.size.width,
                self.abs_layout.location.y + self.abs_layout.size.height,
            ],
        )
    }
}

impl PointDistance for LayoutNode {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let min = [self.abs_layout.location.x, self.abs_layout.location.y];
        let max = [
            self.abs_layout.location.x + self.abs_layout.size.width,
            self.abs_layout.location.y + self.abs_layout.size.height,
        ];

        let dx = if point[0] < min[0] {
            min[0] - point[0]
        } else if point[0] > max[0] {
            point[0] - max[0]
        } else {
            0.
        };

        let dy = if point[1] < min[1] {
            min[1] - point[1]
        } else if point[1] > max[1] {
            point[1] - max[1]
        } else {
            0.
        };

        (dx * dx) + (dy * dy)
    }

    #[inline]
    fn contains_point(&self, point: &[f32; 2]) -> bool {
        let min_x = self.abs_layout.location.x;
        let min_y = self.abs_layout.location.y;
        let max_x = self.abs_layout.location.x + self.abs_layout.size.width;
        let max_y = self.abs_layout.location.y + self.abs_layout.size.height;

        (min_x <= point[0]) & (point[0] <= max_x) & (min_y <= point[1]) & (point[1] <= max_y)
    }
}
