use taffy::{
    CacheTree, Layout, LayoutBlockContainer, LayoutFlexboxContainer, LayoutGridContainer,
    LayoutPartialTree, PrintTree, RoundTree, Style, TraversePartialTree, TraverseTree,
};

use crate::{
    tree::{DynNodeId, Node},
    Tree,
};

pub struct ChildIter<'a>(std::slice::Iter<'a, DynNodeId>);
impl Iterator for ChildIter<'_> {
    type Item = DynNodeId;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().cloned()
    }
}

impl<T: Node> TraversePartialTree<DynNodeId> for Tree<T> {
    type ChildIter<'a> = ChildIter<'a>;
    fn child_ids<'a>(&'a self, node_id: DynNodeId) -> Self::ChildIter<'a> {
        ChildIter(
            self.node_info
                .get(&node_id)
                .expect("didn't call child_ids for a node in the tree")
                .children
                .iter(),
        )
    }

    fn child_count(&self, node_id: DynNodeId) -> usize {
        self.node_info.get(&node_id).map_or(0, |x| x.children.len())
    }

    fn get_child_id(&self, node_id: DynNodeId, index: usize) -> DynNodeId {
        let info = self
            .node_info
            .get(&node_id)
            .expect("didn't call get_child_id for a node in the tree");
        info.children
            .get(index)
            .expect("could not get child_id {index:?}")
            .clone()
    }
}

impl<T: Node> TraverseTree<DynNodeId> for Tree<T> {}

impl<T: Node> LayoutPartialTree<DynNodeId> for Tree<T> {
    type CoreContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: DynNodeId) -> Self::CoreContainerStyle<'_> {
        unsafe { node_id.as_ref() }.get_style()
    }

    fn set_unrounded_layout(&mut self, mut node_id: DynNodeId, layout: &Layout) {
        unsafe { node_id.as_mut() }.set_unrounded_layout(*layout);
    }

    fn resolve_calc_value(&self, _val: *const (), _basis: f32) -> f32 {
        0.0
    }

    #[inline(always)]
    fn compute_child_layout(
        &mut self,
        node: DynNodeId,
        inputs: taffy::LayoutInput,
    ) -> taffy::LayoutOutput {
        // If RunMode is PerformHiddenLayout then this indicates that an ancestor node is `Display::None`
        // and thus that we should lay out this node using hidden layout regardless of it's own display style.
        if inputs.run_mode == taffy::RunMode::PerformHiddenLayout {
            return taffy::compute_hidden_layout(self, node);
        }

        // We run the following wrapped in "compute_cached_layout", which will check the cache for an entry matching the node and inputs and:
        //   - Return that entry if exists
        //   - Else call the passed closure (below) to compute the result
        //
        // If there was no cache match and a new result needs to be computed then that result will be added to the cache
        taffy::compute_cached_layout(self, node, inputs, |tree, mut node, inputs| {
            let style = unsafe { node.as_ref() }.get_style();
            let display_mode = style.display;
            let has_children = tree.child_count(node) > 0;

            // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
            match (display_mode, has_children) {
                (taffy::Display::None, _) => taffy::compute_hidden_layout(tree, node),
                (taffy::Display::Block, true) => taffy::compute_block_layout(tree, node, inputs),
                (taffy::Display::Flex, true) => taffy::compute_flexbox_layout(tree, node, inputs),
                (taffy::Display::Grid, true) => taffy::compute_grid_layout(tree, node, inputs),
                (_, false) => {
                    let style = unsafe { node.as_ref() }.get_style();
                    let style_clone = style.clone();
                    let measure_function = move |known_dimensions, available_space| {
                        unsafe { node.as_mut() }.measure(
                            known_dimensions,
                            available_space,
                            &style_clone,
                        )
                    };
                    // INFO: we do not use the calc (hence why style can be send), hence we return
                    // zero for the calc fn.
                    taffy::compute_leaf_layout(inputs, &style, |_, _| 0.0, measure_function)
                }
            }
        })
    }
}

/// SAFETY: we have a mutable reference to self, and therefore, a mutable reference to the
/// tree, ensuring that nothing else has access to the inner element tree.
///
/// As long as the node_id provided is a valid node belonging to this tree (which has the
/// inner tree pinned), this is safe.
impl<T: Node> CacheTree<DynNodeId> for Tree<T> {
    fn cache_get(
        &self,
        node_id: DynNodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
    ) -> Option<taffy::LayoutOutput> {
        unsafe { node_id.as_ref() }
            .layout_cache()
            .get(known_dimensions, available_space, run_mode)
    }

    fn cache_store(
        &mut self,
        mut node_id: DynNodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
        layout_output: taffy::LayoutOutput,
    ) {
        unsafe { node_id.as_mut() }.layout_cache_mut().store(
            known_dimensions,
            available_space,
            run_mode,
            layout_output,
        );
    }

    fn cache_clear(&mut self, mut node_id: DynNodeId) {
        unsafe { node_id.as_mut() }.layout_cache_mut().clear();
    }
}

impl<T: Node> LayoutBlockContainer<DynNodeId> for Tree<T> {
    type BlockContainerStyle<'a>
        = Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_block_container_style(&self, node_id: DynNodeId) -> Self::BlockContainerStyle<'_> {
        unsafe { node_id.as_ref() }.get_style()
    }

    #[inline(always)]
    fn get_block_child_style(&self, child_node_id: DynNodeId) -> Self::BlockItemStyle<'_> {
        unsafe { child_node_id.as_ref() }.get_style()
    }
}

impl<T: Node> LayoutFlexboxContainer<DynNodeId> for Tree<T> {
    type FlexboxContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: DynNodeId) -> Self::FlexboxContainerStyle<'_> {
        unsafe { node_id.as_ref() }.get_style()
    }

    fn get_flexbox_child_style(&self, child_node_id: DynNodeId) -> Self::FlexboxItemStyle<'_> {
        unsafe { child_node_id.as_ref() }.get_style()
    }
}

impl<T: Node> LayoutGridContainer<DynNodeId> for Tree<T> {
    type GridContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: DynNodeId) -> Self::GridContainerStyle<'_> {
        unsafe { node_id.as_ref() }.get_style()
    }

    fn get_grid_child_style(&self, child_node_id: DynNodeId) -> Self::GridItemStyle<'_> {
        unsafe { child_node_id.as_ref() }.get_style()
    }
}

impl<T: Node> RoundTree<DynNodeId> for Tree<T> {
    fn get_unrounded_layout(&self, node_id: DynNodeId) -> Layout {
        unsafe { node_id.as_ref() }.get_unrounded_layout().clone()
    }

    fn set_final_layout(&mut self, mut node_id: DynNodeId, layout: &Layout) {
        unsafe { node_id.as_mut() }.set_final_layout(layout.clone())
    }
}

impl<T: Node> PrintTree<DynNodeId> for Tree<T> {
    fn get_debug_label(&self, node_id: DynNodeId) -> &'static str {
        unsafe { node_id.as_ref() }.debug_label()
    }

    fn get_final_layout(&self, node_id: DynNodeId) -> Layout {
        unsafe { node_id.as_ref() }.get_final_layout().clone()
    }
}
