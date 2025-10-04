use std::{
    collections::HashMap,
    pin::Pin,
    sync::{atomic::Ordering, Arc, Mutex, RwLock},
    time::Duration,
};

use graphics::{Mesh, Systems, Vertex};
use input::{MouseEvent, MouseEventKind};
use reactive_graph::owner::Owner;
use slotmap::{SecondaryMap, SlotMap};
use taffy::{AvailableSpace, CacheTree, Size};

pub mod events;
mod layout_tree;
mod taffy_impl;
pub mod tree;
pub mod widgets;
mod zindex;

use crate::{
    events::{BlurEvent, EventContext, EventPhase},
    layout_tree::LayoutTree,
    tree::{Element, ElementBuilder, Node, Widget},
    zindex::ZIndexOrdering,
};

pub mod prelude {
    pub use crate::events::*;
    pub use crate::events::{EventContext, EventPhase};
    pub use taffy::prelude::*;
}

pub mod reexports {
    pub use any_spawner;
    pub use reactive_graph;
    pub use reactive_stores;
    pub use taffy;
}

slotmap::new_key_type! { pub struct ElementId; }

pub struct Tree {
    pub owner: Owner,
    pub manager: TreeManager,

    root_node: ElementId,
    alloc: SlotMap<ElementId, Element>,
    node_parents: SecondaryMap<ElementId, Option<ElementId>>,

    capture: Capture,
    size: Size<AvailableSpace>,

    // node_info: HashMap<DynNodeId, NodeInfo>,
    render_order: ZIndexOrdering,
    pub layout_tree: LayoutTree,
}

/// State about the tree's current keyboard focus and mouse capture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capture {
    /// The node currently capturing mouse events.
    mouse_capture: Option<ElementId>,
    /// The node currently with keyboard focus.
    kb_focus: Option<ElementId>,

    /// Last mouse node
    last_entered_node: Option<ElementId>,
}

impl Tree {
    pub fn build<F, W>(size: Size<AvailableSpace>, manager: TreeManager, f: F) -> Self
    where
        F: FnOnce() -> ElementBuilder<W> + 'static,
        W: Widget + 'static,
    {
        let owner = Owner::new();

        let mut alloc = SlotMap::with_key();
        let mut node_parents = SecondaryMap::new();

        let root_node = manager.with(|| owner.with(|| f().build(&mut alloc)));
        Self::populate_parents_map(root_node, &alloc, &mut node_parents);

        let render_order = ZIndexOrdering::default();
        let layout_tree = LayoutTree::default();

        let mut tree = Self {
            owner,
            manager,

            root_node,
            alloc,
            node_parents,

            capture: Capture::default(),
            size,
            render_order,
            layout_tree,
        };
        tree.render_order = ZIndexOrdering::new(&tree);
        tree.compute_root_layout();

        tree
    }
    fn populate_parents_map(
        root_node: ElementId,
        alloc: &SlotMap<ElementId, Element>,
        node_parents: &mut SecondaryMap<ElementId, Option<ElementId>>,
    ) {
        let mut stack: Vec<(ElementId, Option<ElementId>)> = Vec::with_capacity(alloc.len());
        stack.push((root_node, None)); // start with root, which has no parent

        while let Some((node_id, parent_id)) = stack.pop() {
            // record parent for this node
            node_parents.insert(node_id, parent_id);

            // push all children with current node as their parent
            for &child_id in alloc[node_id].children().iter() {
                stack.push((child_id, Some(node_id)));
            }
        }
    }
}

impl Tree {
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

    pub fn children(&self, node_id: ElementId) -> &Vec<ElementId> {
        self.alloc
            .get(node_id)
            .expect("called `Tree::children` for a node not in the tree")
            .children()
    }
    pub fn parent(&self, node_id: ElementId) -> Option<ElementId> {
        self.node_parents
            .get(node_id)
            .expect("called `Tree::parent` for a node not in the tree")
            .clone()
    }
    pub fn compute_root_layout(&mut self) {
        self.compute_layout(self.root_node(), self.size);
        self.layout_tree = LayoutTree::new(&self);
    }

    #[inline(always)]
    fn compute_layout(
        &mut self,
        node_id: ElementId,
        available_space: taffy::Size<taffy::AvailableSpace>,
    ) {
        taffy::compute_root_layout(self, node_id, available_space);
        taffy::round_layout(self, node_id);
    }

    /// After this you should call [`any_spawner::Executor::tick()`] to ensure the reactive state
    /// is updated.
    pub fn on_mouse(&mut self, event: MouseEvent) -> Option<()> {
        self.owner.clone().with(|| {
            self.manager.clone().with(|| {
                // If the mouse event is an actual enter/exit event, reset the tree to its default state.
                match event.kind {
                    MouseEventKind::Leave | MouseEventKind::Enter => {
                        tracing::info!("Got an enter or leave event, resetting capture.");
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
                    tracing::info!("Sending direct mouse event since mouse is captured!");
                    let mut ctx = EventContext::direct(event, node);
                    self.alloc.get_mut(node).unwrap().mouse_event(&mut ctx);
                    self.handle_event_dispatch_cleanup(ctx);
                    self.capture.last_entered_node = Some(node);
                    return Some(());
                }

                tracing::info!("attempting to hit");
                let node = self.layout_tree.hit(event.position).next()?;

                tracing::info!("got hit");
                // If we previously hit a node in our last mouse event, check if the new hit is the same,
                // if so we will not modify any events.
                if let Some(prev) = self.capture.last_entered_node {
                    if prev != node {
                        let transition = if self.has_ancestor(node, prev) {
                            // if the previous node is an ancestor of the new hit, send an enter event to all
                            // nodes from the ancestor onwards.
                            Some((MouseEventKind::Enter, prev, node))
                        } else if self.has_ancestor(prev, node) {
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
                                tracing::info!("sending {kind:?} events");
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

    // dispatch an event that doesnt bubble to each of the nodes from a target node to a parent.
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
                tracing::info!("pushed new blur event");
                self.dispatch_generic_event(EventContext::direct(BlurEvent, prev), |node, ctx| {
                    node.blur_event(ctx)
                });
            }
            self.capture.kb_focus = Some(capture);
            tracing::info!("pushed new focus event");

            self.dispatch_generic_event(EventContext::direct(BlurEvent, capture), |node, ctx| {
                node.blur_event(ctx)
            });
        } else if ctx.is_requesting_kb_focus_release() {
            if let Some(prev) = self.capture.kb_focus {
                tracing::info!("pushed new blur event");
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
        return false;
    }

    pub fn relayout_nodes(&mut self) -> Option<()> {
        let nodes = self.manager.take_relayout_nodes();
        if nodes.is_empty() {
            return None;
        }
        for node in nodes.clone() {
            let mut current = Some(node);
            while let Some(current_node) = current {
                self.cache_clear(current_node);
                current = self.parent(current_node);
            }
        }
        // TODO: Don't recompute the entire layout tree every time.
        self.compute_root_layout();

        Some(())
    }

    pub fn render(&mut self, systems: &mut Systems) -> Mesh<Vertex> {
        self.owner.clone().with(|| {
            self.manager.clone().with(|| {
                let mut mesh = Mesh::empty();
                for node in self.render_order.render_order() {
                    let layout = self.layout_tree.get_layout(*node).unwrap().abs_layout;
                    self.alloc
                        .get_mut(*node)
                        .expect("tried to render a node not in the tree")
                        .render(&mut mesh, systems, &layout);
                }
                mesh
            })
        })
    }
}

static CURRENT_MANAGER: RwLock<Option<TreeManager>> = RwLock::new(None);

#[derive(Clone)]
pub struct TreeManager(Arc<Mutex<TreeManagerInner>>);

struct TreeManagerInner {
    redraw_now_fn: Arc<dyn Fn() + Send + Sync>,
    redraw_duration_fn: Arc<dyn Fn(Duration) + Send + Sync>,

    relayout_nodes: Vec<ElementId>,
}

impl TreeManager {
    pub fn new<N, D>(redraw_now_fn: N, redraw_duration_fn: D) -> Self
    where
        N: Fn() + Send + Sync + 'static,
        D: Fn(Duration) + Send + Sync + 'static,
    {
        Self(Arc::new(Mutex::new(TreeManagerInner {
            redraw_now_fn: Arc::new(redraw_now_fn),
            redraw_duration_fn: Arc::new(redraw_duration_fn),
            relayout_nodes: vec![],
        })))
    }
    /// Set the global handler for the entire process.
    pub fn set_global(handler: TreeManager) {
        *CURRENT_MANAGER.write().unwrap() = Some(handler);
    }

    pub fn global() -> TreeManager {
        CURRENT_MANAGER
            .read()
            .unwrap()
            .clone()
            .expect("tried to get global manager while none is currently set")
    }

    pub fn global_warn_none() -> Option<TreeManager> {
        let mgr = CURRENT_MANAGER.read().unwrap().clone();
        if mgr.is_none() {
            tracing::warn!("Tried to get the current tree manager while none was set!");
        }
        mgr
    }

    /// Temporarily set this handler as the global one while running `f`,
    /// then restore the previous handler. Returns whatever `f` returns.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Store the previous value
        let prev = CURRENT_MANAGER.read().unwrap().clone();
        *CURRENT_MANAGER.write().unwrap() = Some(self.clone());
        let val = f();
        *CURRENT_MANAGER.write().unwrap() = prev;
        val
    }

    fn get_unwrap(&self) -> std::sync::MutexGuard<'_, TreeManagerInner> {
        self.0
            .lock()
            .expect("Can only access TreeManager from one thread at a time.")
    }
}

impl TreeManager {
    pub fn relayout(&self, elem_id: ElementId) {
        let nodes = &mut self.get_unwrap().relayout_nodes;
        nodes.push(elem_id);
    }
    pub fn take_relayout_nodes(&self) -> Vec<ElementId> {
        let mut nodes = &mut self.get_unwrap().relayout_nodes;
        std::mem::replace(&mut nodes, vec![])
    }

    pub fn now(&self) {
        (self.get_unwrap().redraw_now_fn)();
    }
    pub fn duration(&self, duration: Duration) {
        (self.get_unwrap().redraw_duration_fn)(duration);
    }
}

// Non clone so that we always only have one root manager, animation handles themselves can be
// cloned.
struct AnimationManager {
    handle: Arc<()>,
}

impl AnimationManager {
    #[inline(always)]
    fn new() -> Self {
        Self {
            handle: Arc::new(()),
        }
    }

    #[inline(always)]
    fn active_handles(&self) -> usize {
        Arc::strong_count(&self.handle) - 1
    }

    #[inline(always)]
    fn is_animating(&self) -> bool {
        Arc::strong_count(&self.handle) > 0
    }

    fn new_handle(&self) -> AnimationHandle {
        AnimationHandle(self.handle.clone())
    }
}

#[derive(Clone)]
struct AnimationHandle(Arc<()>);
