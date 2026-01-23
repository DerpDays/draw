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
    tree::{Element, Node, Widget, builder::ElementBuilder},
    zindex::ZIndexOrdering,
};

pub mod prelude {
    pub use crate::{
        events::{EventContext, EventPhase, *},
        zindex::ZIndexProperties,
    };
    pub use color;
    pub use graphics_v2;
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

pub trait GuiRenderer: MeasureCtx {
    type Renderer;

    fn update_cached(&mut self, elem_id: ElementId, primitive: graphics_v2::Primitive);
    fn remove_cached(&mut self, elem_id: ElementId);
}

pub trait MeasureCtx {
    fn measure_text(&mut self, text: graphics_v2::primitives::TextMeasure) -> taffy::Size<f32>;
}

pub struct Tree<R: GuiRenderer> {
    pub owner: RootHandle,
    pub manager: TreeManager,

    root_node: ElementId,
    alloc: SlotMap<ElementId, Element>,
    capture: Capture,
    size: Size<AvailableSpace>,

    render_order: ZIndexOrdering,
    pub layout_tree: LayoutTree,

    renderer: Rc<RefCell<R>>,
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

impl<R: GuiRenderer> Tree<R> {
    #[profiling::function]
    pub fn build<F, W>(
        renderer: Rc<RefCell<R>>,
        size: Size<AvailableSpace>,
        manager: TreeManager,
        f: F,
    ) -> Self
    where
        F: FnOnce() -> ElementBuilder<W> + 'static,
        W: Widget + 'static,
    {
        let mut alloc = SlotMap::with_key();
        // let mut node_parents = SecondaryMap::new();

        // we can set this to default since we can confirm
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

            renderer,
        };
        tree.render_order = ZIndexOrdering::new(&tree);
        tree.compute_root_layout();

        tree
    }
}

impl<R: GuiRenderer> Tree<R> {
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
    pub fn resize(&mut self, size: Size<AvailableSpace>) {
        self.size = size;
        for node in self.nodes() {
            log::info!("clearing cache for node {node:?}!");
            self.cache_clear(node.into());
        }
        self.compute_root_layout();
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

    pub fn compute_root_layout(&mut self) {
        self.compute_layout(self.root_node(), self.size);
        self.layout_tree = LayoutTree::new(self);
    }

    #[profiling::function]
    #[inline(always)]
    fn compute_layout(
        &mut self,
        node_id: ElementId,
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) {
        taffy::compute_root_layout(self, node_id.into(), available_space);
        taffy::round_layout(self, node_id.into());
    }

    #[profiling::function]
    pub fn on_mouse(&mut self, event: MouseEvent) -> Option<()> {
        self.owner.clone().run_in(|| {
            self.manager.clone().with(|| {
                // If the mouse event is an actual enter/exit event, reset the tree to its default state.
                match event.kind {
                    MouseEventKind::Leave | MouseEventKind::Enter => {
                        log::info!("Got an enter or leave event, resetting capture.");
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
                                log::info!("calling parent 0!!");
                                current = self.parent(inner);
                                log::info!("done parent 0!!");
                            }
                        }
                        return Some(());
                    }
                    _ => {}
                };

                // If the mouse is currently captured, directly send the event to the captured node.
                if let Some(node) = self.capture.mouse_capture {
                    log::info!("Sending direct mouse event since mouse is captured!");
                    let mut ctx = EventContext::direct(event, node);
                    self.alloc.get_mut(node).unwrap().mouse_event(&mut ctx);
                    self.handle_event_dispatch_cleanup(ctx);
                    self.capture.last_entered_node = Some(node);
                    return Some(());
                }

                let node = self.layout_tree.hit(event.position).next()?;
                log::info!("hit {node:?}");
                log::warn!("parent: {:?}", self.parent(node));

                // If we previously hit a node in our last mouse event, check if the new hit is the same,
                // if so we will not modify any events.
                if let Some(prev) = self.capture.last_entered_node {
                    if prev != node {
                        log::info!("failed on t1");
                        let t1 = self.has_ancestor(node, prev);
                        log::info!("failed on t2");
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

                                log::info!("node depth");
                                let mut depth1 = self.get_node_depth(node1);
                                let mut depth2 = self.get_node_depth(node2);

                                // move the deeper node up until both nodes are at the same level
                                while depth1 > depth2 {
                                    log::info!("calling parent 1!!");
                                    node1 = self.parent(node1).unwrap();
                                    log::info!("done parent 1!!");
                                    depth1 -= 1;
                                }
                                while depth2 > depth1 {
                                    log::info!("calling parent 2!!");
                                    node2 = self.parent(node2).unwrap();
                                    log::info!("done parent 2!!");
                                    depth2 -= 1;
                                }

                                // move both up until they meet
                                while node1 != node2 {
                                    log::info!("calling parent 3!!");
                                    node1 = self.parent(node1).unwrap();
                                    node2 = self.parent(node2).unwrap();
                                    log::info!("done parent 3!!");
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

                        log::info!("calling parent 4!!");
                        current = self.parent(inner);
                        log::info!("done parent 4!!");
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

        log::trace!("there is a focused node.");

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
    pub fn process_changes(&mut self) -> Option<()> {
        // Process child operations first
        let mut child_ops = self.manager.take_child_operations();
        let child_ops_empty = child_ops.is_empty();
        for op in child_ops.drain(..) {
            match op {
                ChildOperation::AddChild {
                    parent,
                    builder,
                    callback,
                } => {
                    let child_id = self.add_child_direct(parent, builder);
                    log::info!("added child: {child_id:?}");
                    if let Some(cb) = callback {
                        cb(child_id);
                    }
                }
                ChildOperation::RemoveChild {
                    parent,
                    child,
                    scope_dispose,
                } => {
                    // Dispose scope first if provided
                    if let Some(dispose) = scope_dispose {
                        dispose();
                    }
                    self.remove_child_direct(parent, child);
                    log::info!("removed child: {child:?}");
                }
            }
        }

        if !child_ops_empty {
            self.manager.recompute_render_order()
        }

        let nodes = self.manager.take_relayout_nodes();
        if nodes.is_empty() && child_ops.is_empty() {
            return None;
        }

        for node in nodes.clone() {
            let mut current = Some(node);
            while let Some(current_node) = current {
                self.cache_clear(current_node.into());
                current = self.parent(current_node);
            }
        }

        if self.manager.take_compute_render_order() {
            self.render_order = ZIndexOrdering::new(self)
        }
        // TODO: Don't recompute the entire layout tree every time.
        self.compute_root_layout();

        Some(())
    }

    #[profiling::function]
    fn add_child_direct(
        &mut self,
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
        self.invalidate_node(parent);

        child_id
    }

    #[profiling::function]
    fn remove_child_direct(&mut self, parent: ElementId, child: ElementId) {
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
            self.renderer.borrow_mut().remove_cached(*node_id);
            self.alloc.remove(*node_id);
        }

        // Invalidate parent's layout
        self.invalidate_node(parent);
    }

    #[profiling::function]
    fn invalidate_node(&mut self, node: ElementId) {
        // Clear layout cache for this node and all ancestors
        let mut current = Some(node);
        while let Some(node_id) = current {
            self.cache_clear(node_id.into());
            self.manager.relayout(node_id);
            current = self.parent(node_id);
        }
        // Mark render order as needing recomputation
        self.manager.recompute_render_order();
    }

    #[profiling::function]
    pub fn render_order(&mut self) -> Vec<ElementId> {
        self.owner.clone().run_in(|| {
            self.manager.clone().with(|| {
                let mut actual_order = Vec::with_capacity(self.render_order.render_order().len());
                for node in self.render_order.render_order() {
                    let layout = self.layout_tree.get_layout(*node).unwrap().abs_layout;
                    let elem = self
                        .alloc
                        .get_mut(*node)
                        .expect("tried to render a node not in the tree");

                    // TODO: do not re-render everything
                    if elem.get_style().display != taffy::Display::None
                        && let Some(primitive) = elem.render(&layout)
                    {
                        self.renderer
                            .borrow_mut()
                            .update_cached(elem.node_id(), primitive);
                        actual_order.push(*node);
                    }
                }
                actual_order
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
    relayout_nodes: Vec<ElementId>,
    compute_render_order: bool,
    child_operations: Vec<ChildOperation>,
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

            relayout_nodes: vec![],
            compute_render_order: false,
            child_operations: vec![],
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
    pub fn relayout(&self, elem_id: ElementId) {
        let nodes = &mut self.get_unwrap().relayout_nodes;
        nodes.push(elem_id);
    }

    pub fn take_relayout_nodes(&self) -> Vec<ElementId> {
        let nodes = &mut self.get_unwrap().relayout_nodes;
        std::mem::take(nodes)
    }
    pub fn recompute_render_order(&self) {
        self.get_unwrap().compute_render_order = true
    }

    pub fn take_compute_render_order(&self) -> bool {
        std::mem::replace(&mut self.get_unwrap().compute_render_order, false)
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
