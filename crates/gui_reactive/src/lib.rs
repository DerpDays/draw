use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

use input::{MouseEvent, MouseEventKind};
use reactive_graph::owner::Owner;
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
    tree::{DynNode, DynNodeId, IntoView, Node, NodeForEach, NodeVisitor},
    zindex::ZIndexOrdering,
};

pub struct Tree<T: Node + 'static> {
    pub owner: Owner,
    pub manager: TreeManager,
    pub inner: Pin<Box<T>>,
    capture: Capture,

    size: Size<AvailableSpace>,

    node_info: HashMap<DynNodeId, NodeInfo>,
    render_order: ZIndexOrdering,
    pub layout_tree: LayoutTree,
}

#[derive(Debug)]
struct NodeInfo {
    parent: Option<DynNodeId>,
    children: Vec<DynNodeId>,
}

struct NodeMapBuilder(HashMap<DynNodeId, NodeInfo>);
impl NodeMapBuilder {
    fn build<T: Node + 'static>(tree: &T) -> HashMap<DynNodeId, NodeInfo> {
        let mut hashmap = HashMap::with_capacity(1000);
        // Topmost node has no parent.
        hashmap.insert(
            tree.node_id(),
            NodeInfo {
                parent: None,
                children: vec![],
            },
        );
        let mut res = Self(hashmap);
        tree.children().for_each_recursive_parent(&mut res, tree);
        res.0
    }
}

impl NodeVisitor for NodeMapBuilder {
    fn visit_with_parent<P: DynNode + 'static, C: DynNode + 'static>(
        &mut self,
        parent: &P,
        child: &C,
    ) {
        self.0.insert(
            child.node_id(),
            NodeInfo {
                parent: Some(parent.node_id()),
                children: vec![],
            },
        );
        if let Some(info) = self.0.get_mut(&parent.node_id()) {
            info.children.push(child.node_id());
        }
    }
}
/// State about the tree's current keyboard focus and mouse capture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capture {
    /// The node currently capturing mouse events.
    mouse_capture: Option<DynNodeId>,
    /// The node currently with keyboard focus.
    kb_focus: Option<DynNodeId>,

    /// Last mouse node
    last_entered_node: Option<DynNodeId>,
}

impl<T: IntoView> Tree<T> {
    pub fn build<F>(size: Size<AvailableSpace>, manager: TreeManager, f: F) -> Self
    where
        F: FnOnce() -> T + 'static,
    {
        let owner = Owner::new();
        let view = Box::pin(manager.with(|| owner.with(move || f().into_view()).inner));

        let node_info = NodeMapBuilder::build(view.as_ref().get_ref());

        let render_order = ZIndexOrdering::default();
        let layout_tree = LayoutTree::default();

        let mut tree = Self {
            owner,
            manager,
            inner: view,
            capture: Capture::default(),

            size,
            node_info,
            render_order,
            layout_tree,
        };
        tree.render_order = ZIndexOrdering::new(&tree);
        tree.layout_tree = LayoutTree::new(&tree);

        tree
    }
}

impl<T: Node> Tree<T> {
    pub fn root_node(&self) -> DynNodeId {
        self.inner.node_id()
    }

    pub fn children(&self, node_id: DynNodeId) -> Vec<DynNodeId> {
        self.node_info
            .get(&node_id)
            .expect("didn't call child_ids for a node in the tree")
            .children
            .clone()
    }
    pub fn parent(&self, node_id: DynNodeId) -> Option<DynNodeId> {
        self.node_info.get(&node_id).map_or(None, |x| x.parent)
    }
    pub fn compute_root_layout(&mut self) {
        self.compute_layout(self.inner.node_id(), self.size);
        self.layout_tree = LayoutTree::new(&self);
    }

    #[inline(always)]
    fn compute_layout(
        &mut self,
        node_id: DynNodeId,
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
                if let Some(mut node) = self.capture.mouse_capture {
                    tracing::info!("Sending direct mouse event since mouse is captured!");
                    let mut ctx = EventContext::direct(event, node);
                    unsafe { node.as_mut() }.mouse_event(&mut ctx);
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
        ancestor: DynNodeId,
        mut target: DynNodeId,
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
        mut handler: impl FnMut(&mut dyn DynNode, &mut EventContext<E>),
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
            let mut node_id = node_id.clone();
            let node = unsafe { node_id.as_mut() };
            handler(node, &mut ctx);
            if !ctx.is_propagating() {
                return self.handle_event_dispatch_cleanup(ctx);
            }
        }

        let mut target_node = ctx.target_node();
        ctx.set_current_node(target_node);
        ctx.set_phase(EventPhase::AtTarget);
        let node = unsafe { target_node.as_mut() };
        handler(node, &mut ctx);

        if !ctx.is_propagating() {
            return self.handle_event_dispatch_cleanup(ctx);
        }

        if ctx.bubbles() {
            ctx.set_phase(EventPhase::Bubbling);
            for node_id in ancestors.iter() {
                ctx.set_current_node(*node_id);
                let mut node_id = node_id.clone();
                let node = unsafe { node_id.as_mut() };
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

    fn get_node_depth(&self, target_node: DynNodeId) -> usize {
        let mut current = Some(target_node);
        let mut depth = 0;
        while let Some(node) = current {
            current = self.parent(node);
            depth += 1;
        }
        depth
    }

    pub fn has_ancestor(&self, node: DynNodeId, ancestor: DynNodeId) -> bool {
        let mut current = node;
        while let Some(parent) = self.parent(current) {
            if parent == ancestor {
                return true;
            }
            current = parent;
        }
        return false;
    }

    pub fn relayout_nodes(&mut self) {
        let nodes = self.manager.take_relayout_nodes();
        for node in nodes {
            let mut current = Some(node);
            while let Some(current_node) = current {
                self.cache_clear(current_node);
                current = self.parent(current_node);
            }
        }
        // TODO: Don't recompute the entire layout tree every time.
        self.compute_root_layout();
    }
}

static CURRENT_MANAGER: RwLock<Option<TreeManager>> = RwLock::new(None);

#[derive(Clone)]
pub struct TreeManager(Arc<Mutex<TreeManagerInner>>);

struct TreeManagerInner {
    redraw_now_fn: fn(),
    redraw_duration_fn: fn(duration: Duration),

    relayout_nodes: Vec<DynNodeId>,
}

impl TreeManager {
    pub fn new(redraw_now_fn: fn(), redraw_duration_fn: fn(Duration)) -> Self {
        Self(Arc::new(Mutex::new(TreeManagerInner {
            redraw_now_fn,
            redraw_duration_fn,
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
        tracing::info!("set mgr");
        let val = f();
        tracing::info!("resetting");
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
    pub fn relayout(&self, node: DynNodeId) {
        let nodes = &mut self.get_unwrap().relayout_nodes;
        nodes.push(node);
    }
    pub fn take_relayout_nodes(&self) -> Vec<DynNodeId> {
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
