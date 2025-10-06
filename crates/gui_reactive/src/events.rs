use std::sync::Arc;

use input::{KeyboardEvent, MouseEvent};

use crate::{
    tree::{builder::ElementBuilder, Node, Widget},
    ElementId,
};

/// Represents the phase of event propagation.
///
/// Event propagation starts at the [`EventPhase::Capturing`] phase, when it reaches the target node it is changed
/// to [`EventPhase::AtTarget`], after which it is changed to [`EventPhase::Bubbling] after
/// starting to propagate back up the node tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPhase {
    /// The event is in the capturing phase, traveling down from the root node to the target node.
    Capturing,
    /// The event is currently at the target node.
    AtTarget,
    /// The event is in the bubbling phase, traveling back up from the target node to the root node.
    Bubbling,

    /// The event is not following normal event propagation, and has been directly sent to the
    /// corresponding target node.
    ///
    /// This can occur in several situations:
    /// - Under keyboard focus: Keyboard events are directly sent to the target node.
    /// - Under mouse capture: Mouse events are directly sent to the target node.
    Direct,
}

/// Context for application-level events, such as mouse, keyboard, and tablet events.
///
/// [`EventContext`] tracks the lifecycle of an event as it propagates through the node tree,
/// allowing event listeners to inspect and modify event-related state, such as stopping propagation,
/// requesting redraws, or pushing messages.
#[derive(Clone, Debug)]
pub struct EventContext<E> {
    /// The user-defined event payload.
    payload: E,

    /// The current phase of this event, it can be one of the three standard event phases
    /// used in the web: capture, at target, bubbling; or alternatively direct.
    ///
    /// For more information about each phase, see [`EventPhase`].
    event_phase: EventPhase,

    /// Whether this event goes through the bubbling phase.
    bubbles: bool,

    /// The original node to which the event was dispatched.
    target_node: ElementId,
    /// The node currently handling the event.
    current_node: ElementId,

    /// Prevent the default behaviour of this widget.
    prevent_default: bool,
    /// If set, indicates that this node is requesting keyboard focus.
    requesting_kb_focus_capture: Option<ElementId>,
    requesting_kb_focus_release: bool,
    /// If set, indicates that this node is requesting mouse capture.
    requesting_mouse_capture: Option<ElementId>,
    /// If set, indicates that a node in the propagation path is requesting mouse release.
    requesting_mouse_release: bool,

    /// Whether the event is still propagating.
    propagating: bool,
}

impl<E> EventContext<E> {
    pub(crate) const fn non_bubbling(payload: E, target_node: ElementId) -> Self {
        Self {
            payload,

            event_phase: EventPhase::Capturing,
            bubbles: false,

            target_node,
            current_node: target_node,

            prevent_default: false,

            requesting_kb_focus_capture: None,
            requesting_kb_focus_release: false,
            requesting_mouse_capture: None,
            requesting_mouse_release: false,

            propagating: true,
        }
    }
    pub(crate) const fn bubbling(payload: E, target_node: ElementId) -> Self {
        Self {
            payload,

            event_phase: EventPhase::Capturing,
            bubbles: true,

            target_node,
            current_node: target_node,

            prevent_default: false,

            requesting_kb_focus_capture: None,
            requesting_kb_focus_release: false,
            requesting_mouse_capture: None,
            requesting_mouse_release: false,

            propagating: true,
        }
    }

    /// Create an event contexts for a given payload and node,
    /// direct events have a [`EventPhase::Direct`], and do not bubble.
    pub(crate) const fn direct(inner: E, node: ElementId) -> Self {
        Self {
            payload: inner,

            event_phase: EventPhase::Direct,
            bubbles: false,

            target_node: node,
            current_node: node,

            prevent_default: false,

            requesting_kb_focus_capture: None,
            requesting_kb_focus_release: false,
            requesting_mouse_capture: None,
            requesting_mouse_release: false,

            propagating: false,
        }
    }
    pub(crate) const fn set_phase(&mut self, phase: EventPhase) {
        self.event_phase = phase;
    }
    pub(crate) const fn set_current_node(&mut self, node: ElementId) {
        self.current_node = node;
    }
    pub(crate) const fn is_propagating(&self) -> bool {
        self.propagating
    }
}

impl<E> EventContext<E> {
    /// Returns a reference to the inner event payload.
    ///
    /// This could be a standard event such as a mouse event, or a custom defined event.
    pub const fn payload(&self) -> &E {
        &self.payload
    }
    /// Stops further propagation of the event.
    ///
    /// This prevents the event from reaching subsequent listeners, this will stop the event at the
    /// node that is current handling it, regardless of the current phase of event dispatch.
    pub const fn stop_propagating(&mut self) {
        self.propagating = false;
    }

    /// Prevents the default action from being taken for the given event.
    ///
    /// NOTE: This does not stop all default actions from being taken, only those that are in
    /// direct associated with the given widgets event handler.
    pub const fn prevent_default(&mut self) {
        self.prevent_default = true;
    }

    /// Requests keyboard focus for the given node.
    ///
    /// If a node prior in the event propagation chain has requested focus, then focus will be only
    /// requested for that prior node instead of the new node provided.
    pub const fn request_kb_focus_capture(&mut self, node: ElementId) {
        if self.requesting_kb_focus_capture.is_none() {
            self.requesting_kb_focus_capture = Some(node);
        }
    }
    pub const fn request_kb_focus_release(&mut self) {
        self.requesting_kb_focus_release = true;
    }
    /// Requests mouse capture for the given node.
    ///
    /// If a node prior in the event propagation chain has requested mouse capture, then the
    /// capture will only be requested for that prior node instead of the new node provided.
    pub fn request_mouse_capture(&mut self, node: ElementId) {
        if self.requesting_mouse_capture.is_none() {
            self.requesting_mouse_capture = Some(node);
        }
    }
    pub const fn request_mouse_release(&mut self) {
        self.requesting_mouse_release = true;
    }

    /// The current phase of the event dispatch lifecycle.
    ///
    /// See [`EventPhase`] for more detail on each lifecycle.
    pub const fn current_phase(&self) -> EventPhase {
        self.event_phase
    }
    /// Helper method to match when the current phase is not capturing.
    /// See [`EventPhase`] for more detail on each lifecycle.
    pub const fn in_capture_phase(&self) -> bool {
        matches!(self.event_phase, EventPhase::Capturing)
    }

    /// Returns `true` if the event supports bubbling.
    pub const fn bubbles(&self) -> bool {
        self.bubbles
    }

    /// The target node of this event
    pub const fn target_node(&self) -> ElementId {
        self.target_node
    }

    /// The current node that is handling the event
    pub const fn current_node(&self) -> ElementId {
        self.current_node
    }

    /// Whether the default behaviour should be prevented.
    pub const fn is_preventing_default(&self) -> bool {
        self.prevent_default
    }
    /// Whether a node is requesting keyboard focus.
    pub const fn is_requesting_kb_focus_capture(&self) -> Option<ElementId> {
        self.requesting_kb_focus_capture
    }
    /// Whether the event is requesting keyboard focus release.
    ///
    /// As captured events are directly sent to the node (no event propagation), this indicates
    /// that the target node requested the keyboard focus release.
    pub const fn is_requesting_kb_focus_release(&self) -> bool {
        self.requesting_kb_focus_release
    }
    /// Whether a node is requesting mouse capture.
    pub const fn is_requesting_mouse_capture(&self) -> Option<ElementId> {
        self.requesting_mouse_capture
    }
    /// Whether the event is requesting mouse release.
    ///
    /// As captured events are directly sent to the node (no event propagation), this indicates
    /// that the target node requested the mouse capture release.
    pub const fn is_requesting_mouse_release(&self) -> bool {
        self.requesting_mouse_release
    }
}

pub struct EventHandler<E> {
    inner: Option<EventHandlerInner<E>>,
}

pub struct EventHandlerInner<E> {
    // owner: Owner,
    handler: Arc<dyn Fn(&dyn Node, &mut EventContext<E>)>,
}
impl<E> EventHandler<E> {
    pub fn empty() -> Self {
        Self { inner: None }
    }
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&dyn Node, &mut EventContext<E>) + 'static,
    {
        Self {
            inner: Some(EventHandlerInner {
                handler: Arc::new(f),
            }),
        }
    }
    pub(crate) fn handle(&self, elem: &dyn Node, ctx: &mut EventContext<E>) {
        if let Some(inner) = &self.inner {
            (inner.handler)(elem, ctx);
        }
    }
}

// Custom events related to the application

pub struct FocusEvent;
pub struct BlurEvent;

impl<W: Widget> ElementBuilder<W> {
    /// Assign a mouse event handler to this element.
    ///
    /// ```rust
    /// div().on_mouse(|event: MouseEvent| ..);
    /// ```
    pub fn on_mouse<F>(mut self, func: F) -> Self
    where
        F: Fn(&dyn Node, &mut EventContext<MouseEvent>) + 'static,
    {
        self.mouse_handler = EventHandler::new(func);
        self
    }

    /// Assign a keyboard event handler to this element.
    ///
    /// ```rust
    /// div().on_keyboard(|event: KeyboardEvent| ..);
    /// ```
    pub fn on_keyboard<F>(mut self, func: F) -> Self
    where
        F: Fn(&dyn Node, &mut EventContext<KeyboardEvent>) + 'static,
    {
        self.keyboard_handler = EventHandler::new(func);
        self
    }

    /// Assign a focus event handler to this element.
    ///
    /// ```rust
    /// div().on_focus(|event: FocusEvent| ..);
    /// ```
    pub fn on_focus<F>(mut self, func: F) -> Self
    where
        F: Fn(&dyn Node, &mut EventContext<FocusEvent>) + 'static,
    {
        self.focus_handler = EventHandler::new(func);
        self
    }

    /// Assign a blur event handler to this element.
    ///
    /// ```rust
    /// div().on_blur(|event: blurEvent| ..);
    /// ```
    pub fn on_blur<F>(mut self, func: F) -> Self
    where
        F: Fn(&dyn Node, &mut EventContext<BlurEvent>) + 'static,
    {
        self.blur_handler = EventHandler::new(func);
        self
    }
}
