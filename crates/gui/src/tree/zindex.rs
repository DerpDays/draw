use std::collections::HashMap;

use taffy::NodeId;

use crate::tree::UITree;

/// Metadata about a gui tree node's z-index
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct ZIndexProperties {
    /// The z-index set for this node
    pub z_index: usize,
    /// Whether to isolate the child nodes z-index from the rest of the siblings/ancestors.
    ///
    /// When this is set to true, the node is rendered last in its z-layer for its current z
    /// context.
    pub isolate_z: bool,
}

impl ZIndexProperties {
    pub const DEFAULT: Self = Self {
        z_index: 0,
        isolate_z: false,
    };

    pub const fn new(z_index: usize, isolate_z: bool) -> Self {
        Self { z_index, isolate_z }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZIndexOrdering {
    map: HashMap<NodeId, usize>,
    render_order: Vec<NodeId>,
}

enum NodeGrouping {
    Node(NodeId),
    IsolatedContext(Vec<NodeId>),
}

impl ZIndexOrdering {
    pub fn node_z_indexing<T>(tree: &UITree<T>, node: NodeId) -> ZIndexProperties {
        tree.get_zindex_properties(node)
    }

    pub fn empty(root: NodeId) -> Self {
        let mut map = HashMap::new();
        map.insert(root, 0);
        let render_order = vec![root];
        Self { map, render_order }
    }

    pub fn new<T>(tree: &UITree<T>) -> Self {
        let mut map = HashMap::new();
        let render_order = Self::sort_stacking_context(tree, tree.root_node());
        for (idx, node) in render_order.iter().enumerate() {
            map.insert(*node, idx);
        }
        Self { map, render_order }
    }

    #[inline]
    pub fn get_node_render_idx(&self, node: NodeId) -> Option<usize> {
        self.map.get(&node).map(|x| *x)
    }
    #[inline]
    pub fn render_order(&self) -> &Vec<NodeId> {
        &self.render_order
    }

    fn sort_stacking_context<T>(tree: &UITree<T>, root: NodeId) -> Vec<NodeId> {
        let mut sorted = vec![];
        let mut stack = tree.children(root);
        stack.reverse();

        while let Some(node) = stack.pop() {
            let z_indexing = tree.get_zindex_properties(node);
            if z_indexing.isolate_z {
                sorted.push((
                    NodeGrouping::IsolatedContext(Self::sort_stacking_context(tree, node)),
                    z_indexing.z_index,
                ));
            } else {
                sorted.push((NodeGrouping::Node(node), z_indexing.z_index));
                for child in tree.children(node).iter().rev() {
                    stack.push(*child)
                }
            }
        }
        sorted.sort_by_key(|(_, z_index)| *z_index);

        let mut result = Vec::with_capacity(sorted.len());
        result.push(root);

        for (grouping, _) in sorted {
            match grouping {
                NodeGrouping::Node(node) => result.push(node),
                NodeGrouping::IsolatedContext(node_ids) => result.extend(node_ids),
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use euclid::default::Size2D;
    use taffy::{NodeId, Style};

    use super::{ZIndexOrdering, ZIndexProperties};
    use crate::tree::UITree;

    #[derive(Copy, Clone, Default)]
    pub struct Widget;

    fn create_ui() -> UITree<Widget> {
        UITree::new(Size2D::new(1024., 1024.))
    }
    fn add_dummy_node(
        tree: &mut UITree<Widget>,
        parent: NodeId,
        order: ZIndexProperties,
    ) -> NodeId {
        let node = tree.new_leaf_with_z(Widget, Style::DEFAULT, order);
        tree.add_child(parent, node);
        node
    }

    #[test]
    fn nested_isolation_order() {
        // root (zIndex: 0, isolation: true)
        // ├── node-1 (zIndex: 0, isolation: false)
        // │   └── node-1-1 (zIndex: 0, isolation: true)
        // │       ├── node-1-1-1 (zIndex: 0, isolation: true)
        // │   └── node-1-2 (zIndex: 2, isolation: true)
        // ├── node-2 (zIndex: 0, isolation: true)
        // ├── node-3 (zIndex: 2, isolation: false)
        //
        // This should create a rendering order of:
        // root, node-1, node-1-1, node-1-1-1, node-2, node-1-2, node-3,
        let mut ui = create_ui();
        let root_node = ui.root_node;

        let node_1 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(0, false));
        let node_1_1 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(0, true));
        let node_1_1_1 = add_dummy_node(&mut ui, node_1_1, ZIndexProperties::new(0, true));
        let node_1_2 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(2, true));
        let node_2 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(0, true));
        let node_3 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(2, false));

        let z = ZIndexOrdering::new(&ui);

        assert_eq!(
            z.get_node_render_idx(root_node),
            Some(0),
            "root_node failed"
        );
        assert_eq!(z.get_node_render_idx(node_1), Some(1), "node_1 failed");
        assert_eq!(z.get_node_render_idx(node_1_1), Some(2), "node_1_1 failed");
        assert_eq!(
            z.get_node_render_idx(node_1_1_1),
            Some(3),
            "node_1_1_1 failed"
        );
        assert_eq!(z.get_node_render_idx(node_2), Some(4), "node_2 failed");
        assert_eq!(z.get_node_render_idx(node_1_2), Some(5), "node_1_2 failed");
        assert_eq!(z.get_node_render_idx(node_3), Some(6), "node_3 failed");
    }

    #[test]
    fn deeply_nested() {
        // root (zIndex: 0, isolation: true)
        // ├── node-1 (zIndex: 3, isolation: true)
        // │   └── node-1-1 (zIndex: 0, isolation: true)
        // │       ├── node-1-1-1 (zIndex: 2, isolation: true)
        //
        // This should create a rendering order of:
        // root, node-1, node-1-1, node-1-1-1
        let mut ui = create_ui();
        let root_node = ui.root_node;

        let node_1 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, true));
        let node_1_1 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(0, true));
        let node_1_1_1 = add_dummy_node(&mut ui, node_1_1, ZIndexProperties::new(2, true));

        let z = ZIndexOrdering::new(&ui);

        for (i, node) in z.render_order.iter().enumerate() {
            println!("Render[{}]: {:?}", i, node);
        }

        assert_eq!(
            z.get_node_render_idx(root_node),
            Some(0),
            "root_node failed"
        );
        assert_eq!(z.get_node_render_idx(node_1), Some(1), "node_1 failed");
        assert_eq!(z.get_node_render_idx(node_1_1), Some(2), "node_1_1 failed");
        assert_eq!(
            z.get_node_render_idx(node_1_1_1),
            Some(3),
            "node_1_1_1 failed"
        );
    }

    #[test]
    fn isolation_ordering() {
        // root (zIndex: 0, isolation: true)
        // ├── node-1 (zIndex: 0, isolation: true)
        // ├── node-2 (zIndex: 1, isolation: true)
        // ├── node-3 (zIndex: 0, isolation: true)
        // ├── node-4 (zIndex: 2, isolation: true)
        // ├── node-5 (zIndex: 2, isolation: true)
        //
        // This should create a rendering order of:
        // root, node-1, node-3, node-2, node-4
        let mut ui = create_ui();
        let root_node = ui.root_node;

        let node_1 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(0, true));
        let node_2 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(1, true));
        let node_3 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(0, true));
        let node_4 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(2, true));
        let node_5 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(2, true));

        let z = ZIndexOrdering::new(&ui);

        println!("node_1: {:?}", z.get_node_render_idx(node_1));
        println!("node_2: {:?}", z.get_node_render_idx(node_2));
        println!("node_3: {:?}", z.get_node_render_idx(node_3));
        println!("node_4: {:?}", z.get_node_render_idx(node_4));
        println!("node_5: {:?}", z.get_node_render_idx(node_5));

        assert_eq!(
            z.get_node_render_idx(root_node),
            Some(0),
            "root_node failed"
        );
        assert_eq!(z.get_node_render_idx(node_1), Some(1), "node_1 failed");
        assert_eq!(z.get_node_render_idx(node_3), Some(2), "node_3 failed");
        assert_eq!(z.get_node_render_idx(node_2), Some(3), "node_2 failed");
        assert_eq!(z.get_node_render_idx(node_4), Some(4), "node_4 failed");
        assert_eq!(z.get_node_render_idx(node_5), Some(5), "node_5 failed");
    }

    #[test]
    fn flat_overlap() {
        // root (zIndex: 0, isolation: true)
        // ├── node-1 (zIndex: 3, isolation: false)
        // │   └── node-1-1 (zIndex: 0, isolation: false)
        // │       ├── node-1-1-1 (zIndex: 2, isolation: false)
        // │   └── node-1-2 (zIndex: 2, isolation: false)
        // │       ├── node-1-2-1 (zIndex: 1, isolation: false)
        // ├── node-2 (zIndex: 1, isolation: false)
        // │   └── node-2-1 (zIndex: 0, isolation: false)
        // ├── node-3 (zIndex: 3, isolation: false)
        //
        // This should create a rendering order of:
        // root, node-1-1, node-2-1, node-1-2-1, node-2, node-1-1-1, node-1-2, node-1, node-3
        let mut ui = create_ui();
        let root_node = ui.root_node;

        let node_1 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, false));
        let node_1_1 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(0, false));
        let node_1_1_1 = add_dummy_node(&mut ui, node_1_1, ZIndexProperties::new(2, false));
        let node_1_2 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(2, false));
        let node_1_2_1 = add_dummy_node(&mut ui, node_1_2, ZIndexProperties::new(1, false));
        let node_2 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(1, false));
        let node_2_1 = add_dummy_node(&mut ui, node_2, ZIndexProperties::new(0, false));
        let node_3 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, false));

        let z = ZIndexOrdering::new(&ui);

        println!("node_1_1: {:?}", z.get_node_render_idx(node_1_1));
        println!("node_2_1: {:?}", z.get_node_render_idx(node_2_1));
        println!("node_1_2_1: {:?}", z.get_node_render_idx(node_1_2_1));
        println!("node_2: {:?}", z.get_node_render_idx(node_2));
        println!("node_1_1_1: {:?}", z.get_node_render_idx(node_1_1_1));
        println!("node_1_2: {:?}", z.get_node_render_idx(node_1_2));
        println!("node_1: {:?}", z.get_node_render_idx(node_1));
        println!("node_3: {:?}", z.get_node_render_idx(node_3));

        assert_eq!(
            z.get_node_render_idx(root_node),
            Some(0),
            "root_node failed"
        );
        assert_eq!(z.get_node_render_idx(node_1_1), Some(1), "node_1_1 failed");
        assert_eq!(z.get_node_render_idx(node_2_1), Some(2), "node_2_1 failed");
        assert_eq!(
            z.get_node_render_idx(node_1_2_1),
            Some(3),
            "node_1_2_1 failed"
        );
        assert_eq!(z.get_node_render_idx(node_2), Some(4), "node_2 failed");
        assert_eq!(
            z.get_node_render_idx(node_1_1_1),
            Some(5),
            "node_1_1_1 failed"
        );
        assert_eq!(z.get_node_render_idx(node_1_2), Some(6), "node_1_2 failed");
        assert_eq!(z.get_node_render_idx(node_1), Some(7), "node_1 failed");
        assert_eq!(z.get_node_render_idx(node_3), Some(8), "node_3 failed");
    }

    #[test]
    fn comprehensive() {
        // root (zIndex: 0, isolation: true)
        // ├── node-1 (zIndex: 3, isolation: false)
        // │   ├── node-1-1 (zIndex: 1, isolation: false)
        // │   └── node-1-2 (zIndex: 5, isolation: true)
        // │       ├── node-1-2-1 (zIndex: 2, isolation: false)
        // │       └── node-1-2-2 (zIndex: 2, isolation: false)
        // ├── node-2 (zIndex: 3, isolation: true)
        // │   ├── node-2-1 (zIndex: 5, isolation: false)
        // │   └── node-2-2 (zIndex: 0, isolation: false)
        // │   └── node-2-3 (zIndex: 0, isolation: true)
        // │       ├── node-2-3-1 (zIndex: 2, isolation: true)
        // ├── node-3 (zIndex: 5, isolation: false)
        // └── node-4 (zIndex: 3, isolation: false)
        //
        // This should create a rendering order of:
        // root, node-1-1, node-1, node-2, node-2-2, node-2-3, node-2-3-1, node-2-1, node-4, node-1-2, node-1-2-1, node-1-2-2, node-3
        let mut ui = create_ui();
        let root_node = ui.root_node;

        let node_1 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, false));
        let node_1_1 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(1, false));
        let node_1_2 = add_dummy_node(&mut ui, node_1, ZIndexProperties::new(5, true));
        let node_1_2_1 = add_dummy_node(&mut ui, node_1_2, ZIndexProperties::new(2, false));
        let node_1_2_2 = add_dummy_node(&mut ui, node_1_2, ZIndexProperties::new(2, false));

        let node_2 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, true));
        let node_2_1 = add_dummy_node(&mut ui, node_2, ZIndexProperties::new(5, false));
        let node_2_2 = add_dummy_node(&mut ui, node_2, ZIndexProperties::new(0, false));
        let node_2_3 = add_dummy_node(&mut ui, node_2, ZIndexProperties::new(0, true));
        let node_2_3_1 = add_dummy_node(&mut ui, node_2_3, ZIndexProperties::new(2, true));

        let node_3 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(5, false));
        let node_4 = add_dummy_node(&mut ui, root_node, ZIndexProperties::new(3, false));

        let z = ZIndexOrdering::new(&ui);

        println!("node_1_1: {:?}", z.get_node_render_idx(node_1_1));
        println!("node_1: {:?}", z.get_node_render_idx(node_1));
        println!("node_2: {:?}", z.get_node_render_idx(node_2));
        println!("node_2_2: {:?}", z.get_node_render_idx(node_2_2));
        println!("node_2_3: {:?}", z.get_node_render_idx(node_2_3));
        println!("node_2_3_1: {:?}", z.get_node_render_idx(node_2_3_1));
        println!("node_2_1: {:?}", z.get_node_render_idx(node_2_1));
        println!("node_4: {:?}", z.get_node_render_idx(node_4));
        println!("node_1_2: {:?}", z.get_node_render_idx(node_1_2));
        println!("node_1_2_1: {:?}", z.get_node_render_idx(node_1_2_1));
        println!("node_1_2_2: {:?}", z.get_node_render_idx(node_1_2_2));
        println!("node_3: {:?}", z.get_node_render_idx(node_3));

        assert_eq!(
            z.get_node_render_idx(root_node),
            Some(0),
            "root_node failed"
        );
        assert_eq!(z.get_node_render_idx(node_1_1), Some(1), "node_1_1 failed");
        assert_eq!(z.get_node_render_idx(node_1), Some(2), "node_1 failed");
        assert_eq!(z.get_node_render_idx(node_2), Some(3), "node_2 failed");
        assert_eq!(z.get_node_render_idx(node_2_2), Some(4), "node_2_2 failed");
        assert_eq!(z.get_node_render_idx(node_2_3), Some(5), "node_2_3 failed");
        assert_eq!(
            z.get_node_render_idx(node_2_3_1),
            Some(6),
            "node_2_3_1 failed"
        );
        assert_eq!(z.get_node_render_idx(node_2_1), Some(7), "node_2_1 failed");
        assert_eq!(z.get_node_render_idx(node_4), Some(8), "node_4 failed");
        assert_eq!(z.get_node_render_idx(node_1_2), Some(9), "node_1_2 failed");
        assert_eq!(
            z.get_node_render_idx(node_1_2_1),
            Some(10),
            "node_1_2_1 failed"
        );
        assert_eq!(
            z.get_node_render_idx(node_1_2_2),
            Some(11),
            "node_1_2_2 failed"
        );
        assert_eq!(z.get_node_render_idx(node_3), Some(12), "node_3 failed");
    }
}
