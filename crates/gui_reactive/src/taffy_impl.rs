use taffy::{
    CacheTree,
    Layout,
    LayoutBlockContainer,
    LayoutFlexboxContainer,
    LayoutGridContainer,
    LayoutPartialTree,
    PrintTree,
    RoundTree,
    Style,
    TraversePartialTree,
    TraverseTree,
};

use crate::{ElementId, Tree, tree::Node};

pub struct ChildIter<'a>(std::slice::Iter<'a, ElementId>);
impl<'a> Iterator for ChildIter<'a> {
    type Item = ElementId;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().cloned()
    }
}
impl TraversePartialTree<ElementId> for Tree {
    type ChildIter<'a> = ChildIter<'a>;
    fn child_ids<'a>(&'a self, node_id: ElementId) -> Self::ChildIter<'a> {
        ChildIter(
            self.alloc
                .get(node_id)
                .expect("called child_ids for a node not in the tree")
                .children()
                .into_iter(),
        )
    }

    fn child_count(&self, node_id: ElementId) -> usize {
        self.alloc
            .get(node_id)
            .expect("called child_count for a node not in the tree")
            .children()
            .len()
    }

    fn get_child_id(&self, node_id: ElementId, index: usize) -> ElementId {
        let elem = self
            .alloc
            .get(node_id)
            .expect("called get_child_id for a node not in the tree");
        elem.children()
            .get(index)
            .expect("could not get child_id {index:?}")
            .clone()
    }
}

impl TraverseTree<ElementId> for Tree {}

impl LayoutPartialTree<ElementId> for Tree {
    type CoreContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: ElementId) -> Self::CoreContainerStyle<'_> {
        self.alloc
            .get(node_id)
            .expect("called get_core_container_style for a node not in the tree")
            .get_style()
    }

    fn set_unrounded_layout(&mut self, node_id: ElementId, layout: &Layout) {
        self.alloc
            .get_mut(node_id)
            .expect("called set_unrounded_layout for a node not in the tree")
            .set_unrounded_layout(*layout);
    }

    fn resolve_calc_value(&self, _val: *const (), _basis: f32) -> f32 {
        0.0
    }

    #[inline(always)]
    fn compute_child_layout(
        &mut self,
        node: ElementId,
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
        taffy::compute_cached_layout(self, node, inputs, |tree, node, inputs| {
            let style = tree
                .alloc
                .get(node)
                .expect("tried to get the style for a node not in the tree")
                .get_style();

            let display_mode = style.display;
            let has_children = tree.child_count(node) > 0;

            // Dispatch to a layout algorithm based on the node's display style and whether the node has children or not.
            match (display_mode, has_children) {
                (taffy::Display::None, _) => taffy::compute_hidden_layout(tree, node),
                (taffy::Display::Block, true) => taffy::compute_block_layout(tree, node, inputs),
                (taffy::Display::Flex, true) => taffy::compute_flexbox_layout(tree, node, inputs),
                (taffy::Display::Grid, true) => taffy::compute_grid_layout(tree, node, inputs),
                (_, false) => {
                    let style_clone = style.clone();

                    let measure_function = |known_dimensions, available_space| {
                        tree.alloc
                            .get_mut(node)
                            .expect("tried to measure a node not in the tree")
                            .measure(known_dimensions, available_space, &style_clone)
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
impl CacheTree<ElementId> for Tree {
    fn cache_get(
        &self,
        node_id: ElementId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
    ) -> Option<taffy::LayoutOutput> {
        self.alloc
            .get(node_id)
            .expect("called cache_get for a node not in the tree")
            .layout_cache()
            .get(known_dimensions, available_space, run_mode)
    }

    fn cache_store(
        &mut self,
        node_id: ElementId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
        layout_output: taffy::LayoutOutput,
    ) {
        self.alloc
            .get_mut(node_id)
            .expect("called cache_store for a node not in the tree")
            .layout_cache_mut()
            .store(known_dimensions, available_space, run_mode, layout_output);
    }

    fn cache_clear(&mut self, node_id: ElementId) {
        self.alloc
            .get_mut(node_id)
            .expect("called cache_clear for a node not in the tree")
            .layout_cache_mut()
            .clear();
    }
}

impl LayoutBlockContainer<ElementId> for Tree {
    type BlockContainerStyle<'a>
        = Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_block_container_style(&self, node_id: ElementId) -> Self::BlockContainerStyle<'_> {
        self.alloc
            .get(node_id)
            .expect("called get_block_container_style for a node not in the tree")
            .get_style()
    }

    #[inline(always)]
    fn get_block_child_style(&self, child_node_id: ElementId) -> Self::BlockItemStyle<'_> {
        self.alloc
            .get(child_node_id)
            .expect("called get_block_child_style for a node not in the tree")
            .get_style()
    }
}

impl LayoutFlexboxContainer<ElementId> for Tree {
    type FlexboxContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: ElementId) -> Self::FlexboxContainerStyle<'_> {
        self.alloc
            .get(node_id)
            .expect("called get_flexbox_container_style for a node not in the tree")
            .get_style()
    }

    fn get_flexbox_child_style(&self, child_node_id: ElementId) -> Self::FlexboxItemStyle<'_> {
        self.alloc
            .get(child_node_id)
            .expect("called get_flexbox_child_style for a node not in the tree")
            .get_style()
    }
}

impl LayoutGridContainer<ElementId> for Tree {
    type GridContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: ElementId) -> Self::GridContainerStyle<'_> {
        self.alloc
            .get(node_id)
            .expect("called get_grid_container_style for a node not in the tree")
            .get_style()
    }

    fn get_grid_child_style(&self, child_node_id: ElementId) -> Self::GridItemStyle<'_> {
        self.alloc
            .get(child_node_id)
            .expect("called get_grid_child_style for a node not in the tree")
            .get_style()
    }
}

impl RoundTree<ElementId> for Tree {
    fn get_unrounded_layout(&self, node_id: ElementId) -> Layout {
        self.alloc
            .get(node_id)
            .expect("called get_unrounded_layout for a node not in the tree")
            .get_unrounded_layout()
            .clone()
    }

    fn set_final_layout(&mut self, node_id: ElementId, layout: &Layout) {
        self.alloc
            .get_mut(node_id)
            .expect("called set_final_layout for a node not in the tree")
            .set_final_layout(*layout);
    }
}

impl PrintTree<ElementId> for Tree {
    fn get_debug_label(&self, node_id: ElementId) -> &'static str {
        self.alloc
            .get(node_id)
            .expect("called get_debug_label for a node not in the tree")
            .debug_label()
    }

    fn get_final_layout(&self, node_id: ElementId) -> Layout {
        self.alloc
            .get(node_id)
            .expect("called get_final_layout for a node not in the tree")
            .get_final_layout()
            .clone()
    }
}
