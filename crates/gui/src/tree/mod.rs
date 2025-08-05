use euclid::default::{Box2D, Point2D, Size2D};
use graphics::{Drawable, Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent, MouseEventKind};
use taffy::{prelude::TaffyMaxContent, Layout, NodeId, Size, Style, TaffyTree};

mod layout;
mod zindex;

use layout::Linear;
pub use zindex::ZIndexProperties;

use crate::{
    events::{EventContext, EventPhase, EventResult, TreeEvent},
    tree::{
        layout::{HittableLayout, LayoutTree},
        zindex::ZIndexOrdering,
    },
    Element,
};

pub struct UITree<T> {
    pub viewport: Size2D<f32>,
    capture: Capture,

    inner: TaffyTree<TreeNode<T>>,
    root_node: NodeId,

    layout_tree: LayoutTree<Linear>,
    render_order: ZIndexOrdering,

    layout_dirty: bool,
    render_order_dirty: bool,
    prev_mouse_hit: Option<NodeId>,

    render_cache: Option<Mesh<Vertex>>,
}

/// State about the tree's current keyboard focus and mouse capture.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capture {
    /// The node currently capturing mouse events.
    mouse_capture: Option<NodeId>,
    /// The node currently with keyboard focus.
    kb_focus: Option<NodeId>,

    /// Last mouse node
    last_entered_node: Option<NodeId>,
}
impl Capture {
    pub const fn mouse_capture(&self) -> Option<NodeId> {
        self.mouse_capture
    }
    pub const fn kb_focus(&self) -> Option<NodeId> {
        self.kb_focus
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeNode<T> {
    /// The actual context represented by this node.
    pub inner: T,
    /// The z-index set for this node
    pub z_index: ZIndexProperties,
}

// TODO: follow this
#[derive(Clone, Copy, Debug, Default)]
pub enum Focusable {
    /// Indicates that the given node is not focusable.
    #[default]
    None,
    // Focusable, follows tree order
    Order,
    // Focusable, with an explicit ordering index
    Index(u64),
}

impl<T: Default> UITree<T> {
    pub fn new(size: Size2D<f32>) -> Self {
        let mut inner = TaffyTree::new();
        let root_node = inner
            .new_leaf_with_context(
                Style::DEFAULT,
                TreeNode {
                    inner: T::default(),
                    z_index: ZIndexProperties::DEFAULT,
                },
            )
            .expect("`TaffyTree::new_leaf` cannot fail");

        let layout_tree = LayoutTree::empty(root_node);
        let render_order = ZIndexOrdering::empty(root_node);

        Self {
            viewport: size,
            capture: Capture::default(),

            inner,
            root_node,

            layout_tree,
            render_order,

            layout_dirty: false,
            render_order_dirty: false,
            prev_mouse_hit: None,

            render_cache: None,
        }
    }
}
impl<T> UITree<T> {
    pub fn new_leaf(&mut self, widget: impl Into<T>, layout: Style) -> NodeId {
        self.inner
            .new_leaf_with_context(
                layout,
                TreeNode {
                    inner: widget.into(),
                    z_index: ZIndexProperties::DEFAULT,
                },
            )
            .expect("`TaffyTree::new_leaf_with_context` cannot fail")
    }
    pub fn new_leaf_with_z(
        &mut self,
        widget: impl Into<T>,
        layout: Style,
        z_index: ZIndexProperties,
    ) -> NodeId {
        self.inner
            .new_leaf_with_context(
                layout,
                TreeNode {
                    inner: widget.into(),
                    z_index,
                },
            )
            .expect("`TaffyTree::new_leaf_with_context` cannot fail")
    }

    /// Adds a node as a child of another node.
    ///
    /// # Panics
    ///
    /// Panics if an invalid parent [`NodeId`] is provided.
    ///
    /// Providing a node that already has a parent is undefined behaviour, and will likely lead to
    /// it not rendering correctly.
    pub fn add_child(&mut self, parent: NodeId, child: NodeId) {
        tracing::info!("adding child!!!!! parent: {parent:?} child: {child:?}");
        self.inner.add_child(parent, child).unwrap();
        self.layout_dirty = true;
        self.render_order_dirty = true;
    }

    /// Removes a node as a child of another node.
    ///
    /// # Panics
    ///
    /// Panics if an invalid parent or child [`NodeId`] is provided
    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) {
        // TODO: taffy's implementation can panic if given an invalid nodeid.
        self.inner.remove_child(parent, child).unwrap();
        self.layout_dirty = true;
    }

    /// Adds a node as a child of another node.
    pub fn set_style(&mut self, node: NodeId, style: Style) {
        self.inner.set_style(node, style).unwrap();
        self.layout_dirty = true;
        self.render_order_dirty = true;
    }

    /// Returns an iterator of NodeId's at a given position in z-index order.
    pub fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.inner
            .children(node)
            .expect("`TaffyTree::children` cannot fail")
    }
    /// Returns the parent of the given node if it exists.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.inner.parent(node)
    }

    pub fn get_zindex_properties(&self, node: NodeId) -> ZIndexProperties {
        self.inner
            .get_node_context(node)
            .expect("All nodes within a UITree must have a context.")
            .z_index
    }
    pub fn set_zindex_properties(&mut self, node: NodeId, properties: ZIndexProperties) {
        self.inner
            .get_node_context_mut(node)
            .expect("All nodes within a UITree must have a context.")
            .z_index = properties;
        self.render_order = ZIndexOrdering::new(&self);
    }

    pub fn get_node(&self, node: NodeId) -> &T {
        &self
            .inner
            .get_node_context(node)
            .expect("All nodes within a UITree must have a context.")
            .inner
    }
    pub fn get_node_mut(&mut self, node: NodeId) -> &mut T {
        &mut self
            .inner
            .get_node_context_mut(node)
            .expect("All nodes within a UITree must have a context.")
            .inner
    }

    /// Returns an iterator of NodeId's at a given position in z-index order.
    pub fn relative_layout(&self, node: NodeId) -> Layout {
        *self
            .inner
            .layout(node)
            .expect("`TaffyTree::layout` cannot fail")
    }

    pub fn has_ancestor(&self, node: NodeId, ancestor: NodeId) -> bool {
        let mut current = node;
        while let Some(parent) = self.parent(current) {
            if parent == ancestor {
                return true;
            }
            current = parent;
        }
        return false;
    }

    pub const fn root_node(&self) -> NodeId {
        self.root_node
    }

    pub const fn capture(&self) -> Capture {
        self.capture
    }
    pub fn print_tree(&mut self, root: NodeId) {
        self.inner.print_tree(root);
    }

    pub const fn update_viewport(&mut self, size: Size2D<f32>) {
        self.viewport = size;
        self.layout_dirty = true;
    }
}

impl<T: Element> UITree<T> {
    pub fn update_layout(&mut self, systems: &mut Systems) {
        self.inner
            .compute_layout_with_measure(
                self.root_node,
                Size::MAX_CONTENT,
                |known_dimensions, available_space, _, ctx, style| {
                    if let Size {
                        width: Some(width),
                        height: Some(height),
                    } = known_dimensions
                    {
                        return Size { width, height };
                    }

                    let ctx = ctx.expect("All nodes in the taffy tree have an inner ctx");
                    ctx.inner.measure(systems, available_space, style)
                },
            )
            .expect("failed to compute layout");

        let start = std::time::Instant::now();
        self.layout_tree = LayoutTree::new(&self);
        self.layout_dirty = false;
        tracing::trace!("Time taken to update layout TREE: {:?}", start.elapsed());
    }

    pub(crate) fn draw_node(&mut self, systems: &mut Systems, node: NodeId) -> &Mesh<Vertex> {
        let layout = self.layout_tree.get_layout(node).expect(
            "attempted to get the layout of a node not in the layout tree - this is a bug!!",
        );
        self.get_node_mut(node).render(systems, layout.abs_layout)
    }
}

impl<T: Element> UITree<T> {
    fn is_node_dirty(&self, node: NodeId) -> bool {
        self.render_cache.is_none() || {
            let mut stack = vec![node];
            let mut is_dirty = false;
            while let Some(node) = stack.pop() {
                is_dirty |= self.get_node(node).is_dirty();
                stack.extend(self.children(node));
            }
            is_dirty
        }
    }
}

impl<T: Element> UITree<T>
where
    T::Message: std::fmt::Debug,
{
    /// This applies a mouse event to the [`UITree`], if the mouse is not hitting any node, it
    /// returns [`None`].
    ///
    /// It takes a [`MouseEvent`] as an argument, this argument should directly correspond to the
    /// raw mouse event.
    ///
    /// Elements receive processed events (i.e. custom leave/enter based on the node currently
    /// with the mouse over.
    ///
    /// Returns the context of all events dispatched.
    pub fn mouse_event(&mut self, event: MouseEvent) -> Option<EventResult<T::Message>> {
        let mut result = EventResult::empty();
        // If the mouse event is an actual enter/exit event, reset the tree to its default state.
        match event.kind {
            MouseEventKind::Leave | MouseEventKind::Enter => {
                tracing::info!("sent leave events");
                self.capture.mouse_capture = None;
                if let Some(prev) = self.capture.last_entered_node.take() {
                    tracing::info!("looping from last entered node");
                    let mut current = Some(prev);
                    while let Some(inner) = current {
                        tracing::info!("inner loop for {inner:?}");
                        let ctx =
                            EventContext::new(MouseEvent::leave(event.position), false, inner);
                        result.combine(
                            self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx)),
                        );
                        current = self.parent(inner);
                    }
                }
                tracing::info!("result!!! {result:?}");
                return Some(result);
            }
            _ => {}
        };

        // If the mouse is currently captured, directly send the event to the captured node.
        if let Some(node) = self.capture.mouse_capture {
            tracing::info!("mouse is currently captured!!");
            let mut ctx = EventContext::direct(event, node);
            self.get_node_mut(node).mouse_event(&mut ctx);
            result.combine(self.handle_event_dispatch_cleanup(ctx));
            self.capture.last_entered_node = Some(node);
            return Some(result);
        }

        let node = self.layout_tree.hit(event.position).next()?;

        // If we previously hit a node in our last mouse event, check if the new hit is the same,
        // if so we will not modify any events.
        if let Some(prev) = self.prev_mouse_hit {
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
                        self.dispatch_event_chain(ancestor, target, kind, event, &mut result);
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
                            &mut result,
                        );
                        self.dispatch_event_chain(
                            node1,
                            node,
                            MouseEventKind::Enter,
                            event,
                            &mut result,
                        );
                    }
                }
            }
        } else {
            // if we are entering the UITree for the first time, send a mouse enter event to all
            // direct ancestors of the hit.
            let mut current = Some(node);
            while let Some(inner) = current {
                let ctx = EventContext::new(MouseEvent::enter(event.position), false, inner);
                result.combine(self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx)));
                current = self.parent(inner);
            }
        }

        self.prev_mouse_hit = Some(node);

        let ctx = EventContext::new(event, true, node);
        result.combine(self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx)));
        self.capture.last_entered_node = Some(node);
        Some(result)
    }

    fn handle_mouse_capture<E>(&mut self, ctx: &mut EventContext<E, T::Message>) {
        if let Some(capture) = ctx.is_requesting_mouse_capture() {
            tracing::info!("setting mouse capture!");
            self.capture.mouse_capture = Some(capture);
        } else if ctx.is_requesting_mouse_release() {
            tracing::info!("releasing mouse capture");
            self.capture.mouse_capture = None;
        }
    }

    /// dispatch an event that doesnt bubble to each of the nodes from a target node to a parent.
    fn dispatch_event_chain(
        &mut self,
        ancestor: NodeId,
        mut target: NodeId,
        kind: MouseEventKind,
        event: MouseEvent,
        result: &mut EventResult<T::Message>,
    ) {
        while target != ancestor {
            let ctx = EventContext::new(MouseEvent { kind, ..event }, false, target);
            result.combine(self.dispatch_generic_event(ctx, |node, ctx| node.mouse_event(ctx)));

            match self.parent(target) {
                Some(parent) => target = parent,
                None => break, // Safety: prevents infinite loop if hierarchy is broken
            }
        }
    }

    /// dispatch an event that doesnt bubble to each of the nodes from a target node to a parent.
    fn get_node_depth(&self, target_node: NodeId) -> usize {
        let mut current = Some(target_node);
        let mut depth = 0;
        while let Some(node) = current {
            current = self.parent(node);
            depth += 1;
        }
        depth
    }
    fn dispatch_tree_event(
        &mut self,
        target_node: NodeId,
        event: TreeEvent,
    ) -> EventResult<T::Message> {
        // TODO: change some of these to direct events
        match event {
            TreeEvent::MouseEvent(payload) => {
                let ctx = EventContext::new(payload, true, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.mouse_event(ctx);
                })
            }
            TreeEvent::KeyboardEvent(payload) => {
                let ctx = EventContext::new(payload, true, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.keyboard_event(ctx);
                })
            }
            TreeEvent::FocusEvent(payload) => {
                let ctx = EventContext::new(payload, false, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.focus_event(ctx);
                })
            }
            TreeEvent::BlurEvent(payload) => {
                let ctx = EventContext::new(payload, false, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.blur_event(ctx);
                })
            }
            TreeEvent::ChangeEventF32(payload) => {
                let ctx = EventContext::new(payload, false, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.change_event_f32(ctx);
                })
            }
            TreeEvent::ChangeEventString(payload) => {
                let ctx = EventContext::new(payload, false, target_node);
                self.dispatch_generic_event(ctx, |node, ctx| {
                    node.change_event_string(ctx);
                })
            }
        }
    }

    fn dispatch_generic_event<E: Clone>(
        &mut self,
        mut ctx: EventContext<E, T::Message>,
        mut handler: impl FnMut(
            &mut dyn Element<Message = T::Message>,
            &mut EventContext<E, T::Message>,
        ),
    ) -> EventResult<T::Message> {
        let mut current_node = ctx.target_node();
        let mut ancestors = vec![];

        while let Some(parent) = self.parent(current_node) {
            ancestors.push(parent);
            current_node = parent;
        }

        ctx.set_phase(EventPhase::Capturing);
        for node_id in ancestors.iter().rev() {
            ctx.set_current_node(*node_id);
            let node = self.get_node_mut(*node_id);
            handler(node, &mut ctx);
            if !ctx.is_propagating() {
                return self.handle_event_dispatch_cleanup(ctx);
            }
        }

        ctx.set_current_node(ctx.target_node());
        ctx.set_phase(EventPhase::AtTarget);
        let node = self.get_node_mut(ctx.target_node());
        handler(node, &mut ctx);
        if !ctx.is_propagating() {
            return self.handle_event_dispatch_cleanup(ctx);
        }

        if ctx.bubbles() {
            ctx.set_phase(EventPhase::Bubbling);
            for node_id in ancestors.iter() {
                ctx.set_current_node(*node_id);
                let node = self.get_node_mut(*node_id);
                handler(node, &mut ctx);
                if !ctx.is_propagating() {
                    return self.handle_event_dispatch_cleanup(ctx);
                }
            }
        }
        self.handle_event_dispatch_cleanup(ctx)
    }

    fn handle_event_dispatch_cleanup<E>(
        &mut self,
        mut ctx: EventContext<E, T::Message>,
    ) -> EventResult<T::Message> {
        self.handle_mouse_capture(&mut ctx);
        for command in ctx.tree_commands() {
            match command {
                TreeCommand::AddChild { parent, child } => {
                    self.add_child(*parent, *child);
                }
                TreeCommand::RemoveChild { parent, child } => {
                    self.remove_child(*parent, *child);
                }
                TreeCommand::SetStyle { node, style } => {
                    self.set_style(*node, style.clone());
                }
                TreeCommand::SetZIndex { node, zindex } => {
                    self.set_zindex_properties(*node, *zindex);
                }
            }
        }
        let mut result = ctx.into_result();
        for (node_id, event) in result.take_new_events() {
            result.combine(self.dispatch_tree_event(node_id, event));
        }
        result
    }

    pub fn keyboard_event(&mut self, event: KeyboardEvent) -> Option<EventResult<T::Message>> {
        // Only handle keyboard events if a node currently has keyboard focus.
        let node = self.capture.kb_focus?;

        let mut ctx = EventContext::direct(event, node);
        self.get_node_mut(node).keyboard_event(&mut ctx);
        Some(self.handle_event_dispatch_cleanup(ctx))
    }
}

impl<T: Element> Drawable<Vertex> for UITree<T> {
    fn render(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
        let start = std::time::Instant::now();
        if self.render_order_dirty {
            self.render_order = ZIndexOrdering::new(&self);
            self.render_cache = None;
            self.render_order_dirty = false;
        }
        tracing::trace!("Time taken to update render_order: {:?}", start.elapsed());

        let start = std::time::Instant::now();
        if self.layout_dirty {
            self.update_layout(systems);
        }
        tracing::trace!("Time taken to update layout: {:?}", start.elapsed());

        let start = std::time::Instant::now();
        if !self.is_dirty() {
            if let Some(ref cache) = self.render_cache {
                tracing::info!("hit cache");
                tracing::trace!("Time taken to tessellate: {:?}", start.elapsed());
                return cache;
            }
        }

        let mut mesh = Mesh::empty();
        for node in self.render_order.render_order().clone() {
            mesh.append(self.draw_node(systems, node))
        }

        self.render_cache = Some(mesh.clone());
        tracing::trace!("Time taken to tessellate: {:?}", start.elapsed());
        self.render_cache.as_ref().unwrap()
    }
    fn bounding_box(&self) -> Box2D<f32> {
        Box2D::new(
            Point2D::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
            Point2D::new(f32::INFINITY, f32::NEG_INFINITY),
        )
    }
    fn is_dirty(&self) -> bool {
        self.is_node_dirty(self.root_node) || self.layout_dirty || self.render_order_dirty
    }
}

/// Possible tree commands able to be emitted from within an event handler.
#[derive(Clone, Debug)]
pub enum TreeCommand {
    AddChild {
        parent: NodeId,
        child: NodeId,
    },
    RemoveChild {
        parent: NodeId,
        child: NodeId,
    },
    SetStyle {
        node: NodeId,
        style: Style,
    },
    SetZIndex {
        node: NodeId,
        zindex: ZIndexProperties,
    },
}
