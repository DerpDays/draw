use taffy::{
    CacheTree,
    Layout,
    LayoutBlockContainer,
    LayoutFlexboxContainer,
    LayoutGridContainer,
    LayoutPartialTree,
    NodeId,
    PrintTree,
    RoundTree,
    Style,
    TraversePartialTree,
    TraverseTree,
};

use crate::{ElementId, GuiRenderer, MeasureCtx, Tree, tree::Node};

pub struct ChildIter<'a>(std::slice::Iter<'a, ElementId>);
impl<'a> Iterator for ChildIter<'a> {
    type Item = NodeId;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().cloned().map(NodeId::from)
    }
}
impl<R: GuiRenderer> TraversePartialTree for Tree<R> {
    type ChildIter<'a>
        = ChildIter<'a>
    where
        R: 'a;
    fn child_ids<'a>(&'a self, node_id: NodeId) -> Self::ChildIter<'a> {
        ChildIter(
            self.alloc
                .get(node_id.into())
                .expect("called child_ids for a node not in the tree")
                .children()
                .iter(),
        )
    }

    fn child_count(&self, node_id: NodeId) -> usize {
        self.alloc
            .get(node_id.into())
            .expect("called child_count for a node not in the tree")
            .children()
            .len()
    }

    fn get_child_id(&self, node_id: NodeId, index: usize) -> NodeId {
        let elem = self
            .alloc
            .get(node_id.into())
            .expect("called get_child_id for a node not in the tree");
        (*elem
            .children()
            .get(index)
            .expect("could not get child_id {index:?}"))
        .into()
    }
}

impl<R: GuiRenderer> TraverseTree for Tree<R> {}

impl<R: GuiRenderer> LayoutPartialTree for Tree<R> {
    type CoreContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
        self.alloc
            .get(node_id.into())
            .expect("called get_core_container_style for a node not in the tree")
            .get_style()
    }

    fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.alloc
            .get_mut(node_id.into())
            .expect("called set_unrounded_layout for a node not in the tree")
            .set_unrounded_layout(*layout);
    }

    fn resolve_calc_value(&self, _val: *const (), _basis: f32) -> f32 {
        0.0
    }

    #[inline(always)]
    fn compute_child_layout(
        &mut self,
        node: NodeId,
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
                .get(node.into())
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

                    {
                        // SAFETY:
                        // The renderer is borrowed only while computing a leaf layout.
                        // `compute_leaf_layout` calls the measure closure synchronously, and leaf
                        // measurement does not have access to the tree, and hence can't re-borrow.
                        let mut renderer = tree.renderer.borrow_mut();
                        let measure_ctx: &mut dyn MeasureCtx = &mut *renderer;
                        let measure_function = |known_dimensions, available_space| {
                            tree.alloc
                                .get_mut(node.into())
                                .expect("tried to measure a node not in the tree")
                                .measure(
                                    measure_ctx,
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
            }
        })
    }
}

/// SAFETY: we have a mutable reference to self, and therefore, a mutable reference to the
/// tree, ensuring that nothing else has access to the inner element tree.
///
/// As long as the node_id provided is a valid node belonging to this tree (which has the
/// inner tree pinned), this is safe.
impl<R: GuiRenderer> CacheTree for Tree<R> {
    fn cache_get(
        &self,
        node_id: NodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
    ) -> Option<taffy::LayoutOutput> {
        self.alloc
            .get(node_id.into())
            .expect("called cache_get for a node not in the tree")
            .layout_cache()
            .get(known_dimensions, available_space, run_mode)
    }

    fn cache_store(
        &mut self,
        node_id: NodeId,
        known_dimensions: taffy::Size<Option<f32>>,
        available_space: taffy::Size<taffy::AvailableSpace>,
        run_mode: taffy::RunMode,
        layout_output: taffy::LayoutOutput,
    ) {
        self.alloc
            .get_mut(node_id.into())
            .expect("called cache_store for a node not in the tree")
            .layout_cache_mut()
            .store(known_dimensions, available_space, run_mode, layout_output);
    }

    fn cache_clear(&mut self, node_id: NodeId) {
        self.alloc
            .get_mut(node_id.into())
            .expect("called cache_clear for a node not in the tree")
            .layout_cache_mut()
            .clear();
    }
}

impl<R: GuiRenderer> LayoutBlockContainer for Tree<R> {
    type BlockContainerStyle<'a>
        = Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = Style
    where
        Self: 'a;

    #[inline(always)]
    fn get_block_container_style(&self, node_id: NodeId) -> Self::BlockContainerStyle<'_> {
        self.alloc
            .get(node_id.into())
            .expect("called get_block_container_style for a node not in the tree")
            .get_style()
    }

    #[inline(always)]
    fn get_block_child_style(&self, child_node_id: NodeId) -> Self::BlockItemStyle<'_> {
        self.alloc
            .get(child_node_id.into())
            .expect("called get_block_child_style for a node not in the tree")
            .get_style()
    }
}

impl<R: GuiRenderer> LayoutFlexboxContainer for Tree<R> {
    type FlexboxContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type FlexboxItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
        self.alloc
            .get(node_id.into())
            .expect("called get_flexbox_container_style for a node not in the tree")
            .get_style()
    }

    fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
        self.alloc
            .get(child_node_id.into())
            .expect("called get_flexbox_child_style for a node not in the tree")
            .get_style()
    }
}

impl<R: GuiRenderer> LayoutGridContainer for Tree<R> {
    type GridContainerStyle<'a>
        = Style
    where
        Self: 'a;

    type GridItemStyle<'a>
        = Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: NodeId) -> Self::GridContainerStyle<'_> {
        self.alloc
            .get(node_id.into())
            .expect("called get_grid_container_style for a node not in the tree")
            .get_style()
    }

    fn get_grid_child_style(&self, child_node_id: NodeId) -> Self::GridItemStyle<'_> {
        self.alloc
            .get(child_node_id.into())
            .expect("called get_grid_child_style for a node not in the tree")
            .get_style()
    }
}

impl<R: GuiRenderer> RoundTree for Tree<R> {
    fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
        *self
            .alloc
            .get(node_id.into())
            .expect("called get_unrounded_layout for a node not in the tree")
            .get_unrounded_layout()
    }

    fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
        self.alloc
            .get_mut(node_id.into())
            .expect("called set_final_layout for a node not in the tree")
            .set_final_layout(*layout);
    }
}

impl<R: GuiRenderer> PrintTree for Tree<R> {
    fn get_debug_label(&self, node_id: NodeId) -> &'static str {
        self.alloc
            .get(node_id.into())
            .expect("called get_debug_label for a node not in the tree")
            .debug_label()
    }

    fn get_final_layout(&self, node_id: NodeId) -> Layout {
        *self
            .alloc
            .get(node_id.into())
            .expect("called get_final_layout for a node not in the tree")
            .get_final_layout()
    }
}
