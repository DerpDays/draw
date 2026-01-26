use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock, Weak},
};

use euclid::default::Point2D;
use sycamore_reactive::{RootHandle, create_root};

use input::{KeyboardEvent, MouseEvent, MouseEventKind};
use slotmap::{Key, SlotMap};
use taffy::{AvailableSpace, CacheTree, Size};

pub mod events;
mod layout_tree;
pub mod reexports;
mod taffy_impl;
pub mod tree;
pub mod widgets;
mod zindex;

use crate::{
    events::{BlurEvent, EventContext, EventPhase},
    layout_tree::LayoutTree,
    taffy_impl::TaffyTree,
    tree::{Element, Node, Widget, builder::ElementBuilder},
    zindex::ZIndexOrdering,
};

pub mod prelude {
    pub use crate::{
        events::{EventContext, EventPhase, *},
        zindex::ZIndexProperties,
    };
    pub use color;
    pub use graphics;
    pub use sycamore_reactive::*;
    pub use taffy::prelude::*;
}

slotmap::new_key_type! { pub struct ElementId; }
impl From<ElementId> for taffy::NodeId {
    fn from(value: ElementId) -> Self {
        Self::new(value.0.as_ffi())
    }
}
impl From<taffy::NodeId> for ElementId {
    fn from(value: taffy::NodeId) -> Self {
        ElementId(slotmap::KeyData::from_ffi(value.into()))
    }
}

pub trait GuiRenderer {
    type Renderer;

    fn update_cached(&mut self, elem_id: ElementId, primitive: graphics::Primitive);
    fn remove_cached(&mut self, elem_id: ElementId);
}

pub trait MeasureCtx {
    fn measure_text(&mut self, text: graphics::primitives::TextMeasure) -> taffy::Size<f32>;
}

pub struct Tree {
    pub owner: RootHandle,
    pub manager: TreeManager,

    root_node: ElementId,
    alloc: SlotMap<ElementId, Element>,
    capture: Capture,
    size: Size<AvailableSpace>,

    render_order: ZIndexOrdering,
    pub layout_tree: LayoutTree,
}

/// State about the tree's current keyboard focus and mouse capture.
#[derive(Copy, Clone, Debug, Default)]
pub struct Capture {
    /// The node currently capturing mouse events.
    mouse_capture: Option<ElementId>,
    /// The node currently with keyboard focus.
    kb_focus: Option<ElementId>,

    /// Last mouse node
    last_entered_node: Option<ElementId>,
}

impl Tree {
    /// You should compute the root layout atleast once before attempting to render this tree.
    ///
    /// See: [`Tree::compute_root_layout`]
    #[profiling::function]
    pub fn build<F, W>(
        size: Size<AvailableSpace>,
        // measure_ctx: &mut dyn MeasureCtx,
        manager: TreeManager,
        f: F,
    ) -> Self
    where
        F: FnOnce() -> ElementBuilder<W> + 'static,
        W: Widget + 'static,
    {
        let mut alloc = SlotMap::with_key();

        // SAFETY: This is unset from null straight after.
        let mut root_node: ElementId = ElementId::null();
        let owner = manager.with(|| {
            create_root(|| {
                root_node = f().build(&mut alloc, None);
            })
        });
        debug_assert!(!root_node.is_null(), "root node cannot be null");
        log::trace!("initial tree has been built");

        let render_order = ZIndexOrdering::default();
        let layout_tree = LayoutTree::default();

        let mut tree = Self {
            owner,
            manager,

            root_node,
            alloc,

            capture: Capture::default(),
            size,
            render_order,
            layout_tree,
        };
        tree.render_order = ZIndexOrdering::new(&tree);

        tree
    }

    pub fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.owner.run_in(|| self.manager.with(f))
    }
}

impl Tree {
    pub fn nodes(&self) -> Vec<ElementId> {
        let mut stack = vec![self.root_node];
        let mut visited = vec![];

        while let Some(node) = stack.pop() {
            visited.push(node);
            stack.extend(self.get(node).children());
        }
        visited
    }
    /// Changes the size used for root size calculations and clears all node layout cache.
    ///
    /// It is important to note that this will not request a redraw, it is up to you,
    /// the callee to do this (if you want).
    #[profiling::function]
    pub fn resize(&mut self, measure_ctx: &mut dyn MeasureCtx, size: Size<AvailableSpace>) {
        self.size = size;
        for node in self.nodes() {
            log::trace!("clearing cache for node: {node:?}");
            self.clear_node_layout(measure_ctx, node);
        }
        self.compute_root_layout(measure_ctx);
    }
    pub fn root_node(&self) -> ElementId {
        self.root_node
    }

    pub fn get(&self, node_id: ElementId) -> &Element {
        self.alloc
            .get(node_id)
            .expect("called `Tree::get` for a node not in the tree")
    }
    pub fn get_mut(&mut self, node_id: ElementId) -> &Element {
        self.alloc
            .get_mut(node_id)
            .expect("called `Tree::get_mut` for a node not in the tree")
    }

    pub fn children(&self, node_id: ElementId) -> &[ElementId] {
        self.alloc
            .get(node_id)
            .expect("called `Tree::children` for a node not in the tree")
            .children()
    }
    pub fn parent(&self, node_id: ElementId) -> Option<ElementId> {
        self.get(node_id).parent_id()
    }

    pub fn compute_root_layout(&mut self, measure_ctx: &mut dyn MeasureCtx) {
        self.compute_layout(self.root_node(), self.size, measure_ctx);
        let root_node = self.root_node;
        self.update_abs_subtree(root_node, taffy::Point::ZERO);
    }

    #[profiling::function]
    #[inline(always)]
    fn compute_layout(
        &mut self,
        node_id: ElementId,
        available_space: taffy::Size<taffy::AvailableSpace>,
        measure_ctx: &mut dyn MeasureCtx,
    ) {
        let mut tree = TaffyTree {
            alloc: &mut self.alloc,
            measure_ctx,
        };
        taffy::compute_root_layout(&mut tree, node_id.into(), available_space);
        taffy::round_layout(&mut tree, node_id.into());
    }

    #[inline(always)]
    pub fn clear_node_layout(&mut self, measure_ctx: &mut dyn MeasureCtx, node: ElementId) {
        let mut tree = TaffyTree {
            alloc: &mut self.alloc,
            measure_ctx,
        };
        tree.cache_clear(node.into());
    }

    #[profiling::function]
    fn clear_node_layout_upwards(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        start_node: ElementId,
    ) {
        let mut current = Some(start_node);
        while let Some(node) = current {
            self.clear_node_layout(measure_ctx, node);
            current = self.parent(node);
        }
    }

    // #[profiling::function]
    // fn clear_tree_layout(&mut self, measure_ctx: &mut dyn MeasureCtx, node: ElementId) {
    //     // Clear layout cache for this node and all ancestors
    //     let mut current = Some(node);
    //     while let Some(node_id) = current {
    //         self.clear_node_layout(measure_ctx, node_id);
    //         current = self.parent(node_id);
    //         self.manager.relayout(node);
    //     }
    //     // Mark render order as needing recomputation
    //     self.manager.recompute_render_order();
    // }
}
impl Tree {
    #[profiling::function]
    pub fn on_mouse(&mut self, event: MouseEvent) -> Option<()> {
        self.owner.clone().run_in(|| {
            self.manager.clone().with(|| {
                // If the mouse event is an actual enter/exit event, reset the tree to its default state.
                match event.kind {
                    MouseEventKind::Leave | MouseEventKind::Enter => {
                        log::trace!("got a {:?} event, resetting capture.", event.kind);
                        self.capture.mouse_capture = None;
                        if let Some(prev) = self.capture.last_entered_node.take() {
                            // Send leave events to all nodes that were previously entered by going up the
                            // tree from the last entered node.
                            let mut current = Some(prev);
                            while let Some(inner) = current {
                                let ctx = EventContext::non_bubbling(
                                    MouseEvent::leave(event.position),
                                    inner,
                                );
                                self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx));
                                current = self.parent(inner);
                            }
                        }
                        return Some(());
                    }
                    _ => {}
                };

                // If the mouse is currently captured, directly send the event to the captured node.
                if let Some(node) = self.capture.mouse_capture {
                    log::trace!("sending direct mouse event since mouse is captured");
                    let mut ctx = EventContext::direct(event, node);
                    self.alloc.get_mut(node).unwrap().mouse_event(&mut ctx);
                    self.handle_event_dispatch_cleanup(ctx);
                    self.capture.last_entered_node = Some(node);
                    return Some(());
                }

                let node = self.hit_layout(event.position).next()?;
                log::trace!("got hit for {node:?}");

                // If we previously hit a node in our last mouse event, check if the new hit is the same,
                // if so we will not modify any events.
                if let Some(prev) = self.capture.last_entered_node {
                    if prev != node {
                        let t1 = self.has_ancestor(node, prev);
                        let t2 = self.has_ancestor(prev, node);
                        let transition = if t1 {
                            // if the previous node is an ancestor of the new hit, send an enter event to all
                            // nodes from the ancestor onwards.
                            Some((MouseEventKind::Enter, prev, node))
                        } else if t2 {
                            // if the new hit is the ancestor of the previous node, send leave events to all
                            // nodes inbetween.
                            Some((MouseEventKind::Leave, node, prev))
                        } else {
                            // send leave events to the previous node until their ancestor matches an
                            // ancestor of our current node, then send enter events to the new node
                            None
                        };

                        match transition {
                            Some((kind, ancestor, target)) => {
                                log::info!("sending {kind:?} events");
                                self.dispatch_event_chain(ancestor, target, kind, event);
                            }
                            None => {
                                // TODO: improve this
                                let mut node1 = prev;
                                let mut node2 = node;

                                let mut depth1 = self.get_node_depth(node1);
                                let mut depth2 = self.get_node_depth(node2);

                                // move the deeper node up until both nodes are at the same level
                                while depth1 > depth2 {
                                    node1 = self.parent(node1).unwrap();
                                    depth1 -= 1;
                                }
                                while depth2 > depth1 {
                                    node2 = self.parent(node2).unwrap();
                                    depth2 -= 1;
                                }

                                // move both up until they meet
                                while node1 != node2 {
                                    node1 = self.parent(node1).unwrap();
                                    node2 = self.parent(node2).unwrap();
                                }

                                self.dispatch_event_chain(
                                    node1,
                                    prev,
                                    MouseEventKind::Leave,
                                    event,
                                );
                                self.dispatch_event_chain(
                                    node1,
                                    node,
                                    MouseEventKind::Enter,
                                    event,
                                );
                            }
                        }
                    }
                } else {
                    // if we are entering the UITree for the first time, send a mouse enter event to all
                    // direct ancestors of the hit.
                    let mut current = Some(node);
                    while let Some(inner) = current {
                        let ctx =
                            EventContext::non_bubbling(MouseEvent::enter(event.position), inner);
                        self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx));

                        current = self.parent(inner);
                    }
                }

                let ctx = EventContext::bubbling(event, node);
                self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx));
                self.capture.last_entered_node = Some(node);
                Some(())
            })
        })
    }

    #[profiling::function]
    pub fn on_keyboard(&mut self, event: KeyboardEvent) -> Option<()> {
        // Only handle keyboard events if a node currently has keyboard focus.
        log::trace!("checking kb_focus {:?}", self.capture.kb_focus);
        let node = self.capture.kb_focus?;

        log::trace!("sending direct keyboard event.");
        let mut ctx = EventContext::direct(event, node);
        self.alloc.get_mut(node).unwrap().keyboard_event(&mut ctx);
        Some(())
    }

    // dispatch an event that doesnt bubble to each of the nodes from a target node to a parent.
    #[profiling::function]
    fn dispatch_event_chain(
        &mut self,
        ancestor: ElementId,
        mut target: ElementId,
        kind: MouseEventKind,
        event: MouseEvent,
    ) {
        while target != ancestor {
            let ctx = EventContext::non_bubbling(MouseEvent { kind, ..event }, target);
            self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx));

            match self.parent(target) {
                Some(parent) => target = parent,
                None => break, // Safety: prevents infinite loop if hierarchy is broken
            }
        }
    }

    #[profiling::function]
    fn dispatch_generic_event<E>(
        &mut self,
        mut ctx: EventContext<E>,
        mut handler: impl FnMut(&mut dyn Node, &mut EventContext<E>),
    ) {
        let mut current_node = ctx.target_node();
        let mut ancestors = vec![];

        while let Some(parent) = self.parent(current_node) {
            ancestors.push(parent);
            current_node = parent;
        }

        ctx.set_phase(EventPhase::Capturing);
        for node_id in ancestors.iter().rev() {
            ctx.set_current_node(*node_id);
            let node = self.alloc.get_mut(*node_id).unwrap();
            handler(node, &mut ctx);
            if !ctx.is_propagating() {
                return self.handle_event_dispatch_cleanup(ctx);
            }
        }

        let target_node = ctx.target_node();
        ctx.set_current_node(target_node);
        ctx.set_phase(EventPhase::AtTarget);

        let node = self.alloc.get_mut(target_node).unwrap();
        handler(node, &mut ctx);

        if !ctx.is_propagating() {
            return self.handle_event_dispatch_cleanup(ctx);
        }

        if ctx.bubbles() {
            ctx.set_phase(EventPhase::Bubbling);
            for node_id in ancestors.iter() {
                ctx.set_current_node(*node_id);
                let node = self.alloc.get_mut(*node_id).unwrap();
                handler(node, &mut ctx);
                if !ctx.is_propagating() {
                    return self.handle_event_dispatch_cleanup(ctx);
                }
            }
        }

        self.handle_event_dispatch_cleanup(ctx);
    }
    fn handle_event_dispatch_cleanup<E>(&mut self, mut ctx: EventContext<E>) {
        self.handle_mouse_capture(&mut ctx);
        self.handle_kb_focus(&mut ctx);
    }
    #[inline]
    const fn handle_mouse_capture<E>(&mut self, ctx: &mut EventContext<E>) {
        if let Some(capture) = ctx.is_requesting_mouse_capture() {
            self.capture.mouse_capture = Some(capture);
        } else if ctx.is_requesting_mouse_release() {
            self.capture.mouse_capture = None;
        }
    }

    #[inline]
    fn handle_kb_focus<E>(&mut self, ctx: &mut EventContext<E>) {
        if let Some(capture) = ctx.is_requesting_kb_focus_capture() {
            if let Some(prev) = self.capture.kb_focus {
                self.dispatch_generic_event(EventContext::direct(BlurEvent, prev), |node, ctx| {
                    node.blur_event(ctx)
                });
            }
            self.capture.kb_focus = Some(capture);

            self.dispatch_generic_event(EventContext::direct(BlurEvent, capture), |node, ctx| {
                node.blur_event(ctx)
            });
        } else if ctx.is_requesting_kb_focus_release() {
            if let Some(prev) = self.capture.kb_focus {
                self.dispatch_generic_event(EventContext::direct(BlurEvent, prev), |node, ctx| {
                    node.blur_event(ctx)
                });
            }
            self.capture.kb_focus = None;
        }
    }

    fn get_node_depth(&self, target_node: ElementId) -> usize {
        let mut current = Some(target_node);
        let mut depth = 0;
        while let Some(node) = current {
            current = self.parent(node);
            depth += 1;
        }
        depth
    }

    pub fn has_ancestor(&self, node: ElementId, ancestor: ElementId) -> bool {
        let mut current = node;
        while let Some(parent) = self.parent(current) {
            if parent == ancestor {
                return true;
            }
            current = parent;
        }
        false
    }

    #[profiling::function]
    pub fn process_changes<R: GuiRenderer + MeasureCtx>(&mut self, renderer: &mut R) -> Option<()> {
        let mut structure_changed = false;
        let mut layout_changed = false;

        // 1. Handle Structure Changes (Add/Remove)
        let mut child_ops = self.manager.take_child_operations();
        if !child_ops.is_empty() {
            structure_changed = true;
            layout_changed = true; // Adding/removing usually shifts layout

            for op in child_ops.drain(..) {
                match op {
                    ChildOperation::AddChild {
                        parent,
                        builder,
                        callback,
                    } => {
                        let child = self.add_child_direct(renderer, parent, builder);
                        // Add to LayoutTree map immediately (with dummy pos) or wait for full pass
                        if let Some(cb) = callback {
                            cb(child);
                        }
                    }
                    ChildOperation::RemoveChild {
                        parent,
                        child,
                        scope_dispose,
                    } => {
                        if let Some(dispose) = scope_dispose {
                            dispose();
                        }
                        self.remove_child_direct(renderer, parent, child);
                        // Remove from LayoutTree map
                        self.remove_abs_layout(child);
                    }
                }
            }
        }

        // 2. Check if Z-Order explicit flag was set (e.g. style change on z-index prop)
        if self.manager.take_structure_dirty() {
            structure_changed = true;
        }

        // 3. Recompute Render Order only if structure changed
        if structure_changed {
            // This is still O(N) but only runs on add/remove or z-index change
            self.render_order = ZIndexOrdering::new(self);
        }

        // 4. Handle Layout Changes
        let dirty_nodes = self.manager.take_dirty_layout_nodes();
        if layout_changed || !dirty_nodes.is_empty() {
            // A. Clear Taffy Cache for dirty nodes
            for node in &dirty_nodes {
                // Optimization: Only clear up to the point where layout boundaries stop propagating
                // For now, clearing ancestor chain is safe.
                self.clear_node_layout_upwards(renderer, *node);
            }

            // B. Run Taffy Layout Algo
            // This updates the 'relative' layout in self.alloc
            self.compute_root_layout(renderer);

            // C. Update Absolute Positions in LayoutTree
            // Optimization: If we knew exactly which subtree changed, we could pass that.
            // For now, updating the whole absolute tree is still much faster than re-allocating it.
            // A better approach: find common ancestor of dirty nodes, but Root is safest fallback.
            self.update_abs_subtree(self.root_node(), taffy::Point::ZERO);
        }

        if structure_changed || layout_changed || !dirty_nodes.is_empty() {
            Some(())
        } else {
            None
        }
    }

    #[profiling::function]
    fn add_child_direct(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        parent: ElementId,
        builder: Box<dyn tree::builder::ErasedBuilder>,
    ) -> ElementId {
        // Build the node within the reactive context
        // The scope management is handled in ReactiveChildren widget
        let child_id = self.owner.clone().run_in(|| {
            self.manager
                .clone()
                .with(|| builder.build(&mut self.alloc, Some(parent)))
        });

        // Add to parent's children list
        if let Some(parent_elem) = self.alloc.get_mut(parent) {
            parent_elem.children.push(child_id);
        }

        // Invalidate parent's layout cache and trigger relayout
        self.clear_node_layout_upwards(measure_ctx, parent);

        child_id
    }

    #[profiling::function]
    fn remove_child_direct<R: GuiRenderer + MeasureCtx>(
        &mut self,
        renderer: &mut R,
        parent: ElementId,
        child: ElementId,
    ) {
        // Validate parent-child relationship
        if self.parent(child) != Some(parent) {
            log::warn!(
                "Attempted to remove child {:?} from parent {:?}, but parent relationship doesn't match",
                child,
                parent
            );
            return;
        }

        // Collect all descendants to remove (including the child itself)
        let mut to_remove = vec![child];
        let mut stack = vec![child];

        while let Some(current) = stack.pop() {
            if let Some(elem) = self.alloc.get(current) {
                for &grandchild in elem.children() {
                    to_remove.push(grandchild);
                    stack.push(grandchild);
                }
            }
        }

        // Remove from parent's children list
        if let Some(parent_elem) = self.alloc.get_mut(parent) {
            parent_elem.children.retain(|&id| id != child);
        }

        // Clean up focus/capture state
        for &removed_node in &to_remove {
            if self.capture.mouse_capture == Some(removed_node) {
                self.capture.mouse_capture = None;
            }
            if self.capture.kb_focus == Some(removed_node) {
                self.capture.kb_focus = None;
            }
            if self.capture.last_entered_node == Some(removed_node) {
                let mut current = self.capture.last_entered_node;
                while let Some(parent) = current {
                    if !to_remove.contains(&parent) {
                        self.capture.last_entered_node = Some(parent);
                        break;
                    }

                    let ctx =
                        EventContext::non_bubbling(MouseEvent::leave(Point2D::zero()), parent);
                    self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx));
                    current = self.parent(parent);
                }
            }
        }

        // Remove from alloc and renderer cache
        for node_id in &to_remove {
            renderer.remove_cached(*node_id);
            self.alloc.remove(*node_id);
            self.remove_abs_layout(*node_id);
        }

        // Invalidate parent's layout
        self.clear_node_layout_upwards(renderer, parent);
    }

    #[profiling::function]
    pub fn render_order<R: GuiRenderer>(&mut self, renderer: &mut R) -> Vec<ElementId> {
        self.owner.clone().run_in(|| {
            self.manager.clone().with(|| {
                let render_order = self.render_order.render_order().to_vec();
                for node in &render_order {
                    let layout = self.get_abs_layout(*node).unwrap().abs_layout;
                    let elem = self
                        .alloc
                        .get_mut(*node)
                        .expect("tried to render a node not in the tree");

                    // TODO: do not re-render everything
                    if let Some(primitive) = elem.render(&layout) {
                        renderer.update_cached(*node, primitive);
                    }
                }
                render_order
            })
        })
    }
}

thread_local! {
    static CURRENT_MANAGER: RwLock<Option<TreeManager>> = const { RwLock::new(None) };
}

#[derive(Clone)]
pub struct TreeManager(Rc<RefCell<TreeManagerInner>>);

pub(crate) enum ChildOperation {
    AddChild {
        parent: ElementId,
        builder: Box<dyn tree::builder::ErasedBuilder>,
        callback: Option<Box<dyn FnOnce(ElementId)>>,
    },
    RemoveChild {
        parent: ElementId,
        child: ElementId,
        scope_dispose: Option<Box<dyn FnOnce()>>,
    },
}

struct TreeManagerInner {
    redraw_now_fn: Arc<dyn Fn() + Send + Sync>,
    new_handle_fn: Arc<dyn Fn() -> AnimationHandle + Send + Sync>,
    child_operations: Vec<ChildOperation>,

    layout_dirty_nodes: Vec<ElementId>,
    /// Whether the z-index has changed, or a node has been added/removed,
    /// fundamentally changing the UI structure.
    structure_dirty: bool,
}

impl TreeManager {
    // pub fn new<N, D>(redraw_now_fn: N, redraw_duration_fn: D) -> Self
    pub fn new<N, H>(redraw_now_fn: N, new_handle_fn: H) -> Self
    where
        N: Fn() + Send + Sync + 'static,
        H: Fn() -> AnimationHandle + Send + Sync + 'static,
    {
        Self(Rc::new(RefCell::new(TreeManagerInner {
            redraw_now_fn: Arc::new(redraw_now_fn),
            new_handle_fn: Arc::new(new_handle_fn),

            child_operations: vec![],

            layout_dirty_nodes: vec![],
            structure_dirty: false,
        })))
    }
    /// Set the global handler for the entire process.
    pub fn set_global(handler: TreeManager) {
        CURRENT_MANAGER.with(|mgr| {
            *mgr.write().unwrap() = Some(handler);
        })
    }

    pub fn global() -> TreeManager {
        CURRENT_MANAGER.with(|mgr| {
            mgr.read()
                .unwrap()
                .clone()
                .expect("tried to get global manager while none is currently set")
        })
    }

    pub fn try_global() -> Option<TreeManager> {
        CURRENT_MANAGER.with(|mgr| mgr.read().unwrap().clone())
    }

    /// Temporarily set this handler as the global one while running `f`,
    /// then restore the previous handler. Returns whatever `f` returns.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Store the previous value
        let prev = Self::try_global();
        Self::set_global(self.clone());
        let val = f();
        CURRENT_MANAGER.with(|mgr| {
            *mgr.write().unwrap() = prev;
        });
        val
    }

    fn get_unwrap(&self) -> std::cell::RefMut<'_, TreeManagerInner> {
        self.0.borrow_mut()
    }
}

impl TreeManager {
    pub fn mark_layout_dirty(&self, elem_id: ElementId) {
        self.get_unwrap().layout_dirty_nodes.push(elem_id);
    }

    pub fn mark_structure_dirty(&self) {
        self.get_unwrap().structure_dirty = true;
    }

    pub fn take_dirty_layout_nodes(&self) -> Vec<ElementId> {
        let nodes = &mut self.get_unwrap().layout_dirty_nodes;
        std::mem::take(nodes)
    }
    pub fn take_structure_dirty(&mut self) -> bool {
        std::mem::take(&mut self.get_unwrap().structure_dirty)
    }

    pub fn now(&self) {
        (self.get_unwrap().redraw_now_fn)();
    }

    pub fn new_animation_handle(&self) -> AnimationHandle {
        let handle = (self.get_unwrap().new_handle_fn)();
        self.now();
        handle
    }

    pub fn queue_add_child<C>(
        &self,
        parent: ElementId,
        builder: Box<dyn tree::builder::ErasedBuilder>,
        callback: Option<C>,
    ) where
        C: FnOnce(ElementId) + 'static,
    {
        let ops = &mut self.get_unwrap().child_operations;
        ops.push(ChildOperation::AddChild {
            parent,
            builder,
            callback: callback.map(|c| Box::new(c) as Box<dyn FnOnce(ElementId)>),
        });
    }

    pub fn queue_remove_child<D>(
        &self,
        parent: ElementId,
        child: ElementId,
        scope_dispose: Option<D>,
    ) where
        D: FnOnce() + 'static,
    {
        let ops = &mut self.get_unwrap().child_operations;
        ops.push(ChildOperation::RemoveChild {
            parent,
            child,
            scope_dispose: scope_dispose.map(|d| Box::new(d) as Box<dyn FnOnce()>),
        });
    }

    pub(crate) fn take_child_operations(&self) -> Vec<ChildOperation> {
        std::mem::take(&mut self.get_unwrap().child_operations)
    }
}

// Non clone so that we always only have one root manager, animation handles themselves can be
// cloned.
#[derive(Default)]
pub struct AnimationManager {
    handle: Arc<()>,
}

impl AnimationManager {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            handle: Arc::new(()),
        }
    }

    #[inline(always)]
    pub fn active_handles(&self) -> usize {
        Arc::weak_count(&self.handle)
    }

    #[inline(always)]
    pub fn is_animating(&self) -> bool {
        Arc::weak_count(&self.handle) >= 1
    }

    #[inline(always)]
    pub fn new_handle(&self) -> AnimationHandle {
        AnimationHandle(Arc::downgrade(&self.handle))
    }
}

#[derive(Clone)]
pub struct AnimationHandle(#[allow(unused)] Weak<()>);
