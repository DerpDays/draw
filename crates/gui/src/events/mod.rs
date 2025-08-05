use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use taffy::NodeId;

use crate::tree::TreeCommand;
use crate::Element;
pub use input::KeyboardEvent;
pub use input::MouseEvent;

#[derive(Clone, Debug, PartialEq)]
pub enum TreeEvent {
    MouseEvent(MouseEvent),
    KeyboardEvent(KeyboardEvent),
    FocusEvent(FocusEvent),
    BlurEvent(BlurEvent),
    ChangeEventF32(ChangeEvent<f32>),
    ChangeEventString(ChangeEvent<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangeEvent<T> {
    pub new: T,
    pub old: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusEvent;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlurEvent;

#[derive(Clone, Default)]
pub struct EventHandler<E, S: Element> {
    pub handler: Option<Arc<dyn Fn(&mut S, &mut EventContext<E, S::Message>)>>,
    _event_marker: PhantomData<E>,
    _widget_marker: PhantomData<S>,
}

pub trait HandlesEvent<E>
where
    Self: Sized + Element,
{
    fn handler_mut(&mut self) -> &mut EventHandler<E, Self>;
}

pub trait MouseEventHandler<S: Element> {
    fn mouse_handler<F>(self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<MouseEvent, S::Message>) + 'static;
}

impl<S: HandlesEvent<MouseEvent> + Element> MouseEventHandler<S> for S {
    fn mouse_handler<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<MouseEvent, <S as Element>::Message>) + 'static,
    {
        *self.handler_mut() = EventHandler::new(func);
        self
    }
}

pub trait KeyboardEventHandler<S: Element> {
    fn keyboard_handler<F>(self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<KeyboardEvent, S::Message>) + 'static;
}

impl<S: HandlesEvent<KeyboardEvent> + Element> KeyboardEventHandler<S> for S {
    fn keyboard_handler<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<KeyboardEvent, <S as Element>::Message>) + 'static,
    {
        *self.handler_mut() = EventHandler::new(func);
        self
    }
}

pub trait FocusEventHandler<S: Element> {
    fn focus_handler<F>(self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<FocusEvent, S::Message>) + 'static;
}

impl<S: HandlesEvent<FocusEvent> + Element> FocusEventHandler<S> for S {
    fn focus_handler<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<FocusEvent, <S as Element>::Message>) + 'static,
    {
        *self.handler_mut() = EventHandler::new(func);
        self
    }
}

pub trait BlurEventHandler<S: Element> {
    fn blur_handler<F>(self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<BlurEvent, S::Message>) + 'static;
}

impl<S: HandlesEvent<BlurEvent> + Element> BlurEventHandler<S> for S {
    fn blur_handler<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<BlurEvent, <S as Element>::Message>) + 'static,
    {
        *self.handler_mut() = EventHandler::new(func);
        self
    }
}

pub trait ChangeEventHandler<S: Element, T> {
    fn change_handler<F>(self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<ChangeEvent<T>, S::Message>) + 'static;
}

impl<S: HandlesEvent<ChangeEvent<T>> + Element, T> ChangeEventHandler<S, T> for S {
    fn change_handler<F>(mut self, func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<ChangeEvent<T>, <S as Element>::Message>) + 'static,
    {
        *self.handler_mut() = EventHandler::new(func);
        self
    }
}

impl<E, S: Element> EventHandler<E, S> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut S, &mut EventContext<E, S::Message>) + 'static,
    {
        Self {
            handler: Some(Arc::new(func)),
            _event_marker: PhantomData,
            _widget_marker: PhantomData,
        }
    }
    pub const fn none() -> Self {
        Self {
            handler: None,
            _event_marker: PhantomData,
            _widget_marker: PhantomData,
        }
    }
    pub(crate) fn handle(&self, state: &mut S, ctx: &mut EventContext<E, S::Message>) {
        if let Some(handler) = &self.handler {
            handler(state, ctx)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redraw {
    Now,
    Duration(Duration),
}

impl Redraw {
    pub fn retain_longest(self, other: Redraw) -> Redraw {
        match other {
            Redraw::Now => return self,
            Redraw::Duration(new_duration) => match self {
                Redraw::Now => return other,
                Redraw::Duration(current_duration) => {
                    if current_duration < new_duration {
                        return other;
                    } else {
                        return self;
                    }
                }
            },
        }
    }
}

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
pub struct EventContext<E, M: Clone> {
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
    target_node: NodeId,
    /// The node currently handling the event.
    current_node: NodeId,

    /// If set, indicates that a redraw is requested after event processing.
    redraw: Option<Redraw>,
    /// If set, indicates that this node is requesting keyboard focus.
    requesting_kb_focus: Option<NodeId>,
    /// If set, indicates that this node is requesting mouse capture.
    requesting_mouse_capture: Option<NodeId>,
    /// If set, indicates that a node in the propagation path is requesting mouse release.
    requesting_mouse_release: bool,
    /// Accumulated messages emitted during the event lifecycle, along with their associated node.
    pub messages: Vec<(NodeId, Vec<M>)>,

    /// Accumulated tree commands emitted during the event lifecycle.
    tree_commands: Vec<TreeCommand>,
    /// New events needing to be dispatched.
    events: Vec<(NodeId, TreeEvent)>,
    /// Whether the event is still propagating.
    propagating: bool,
}

impl<E, M: Clone> EventContext<E, M> {
    /// Stops further propagation of the event.
    ///
    /// This prevents the event from reaching subsequent listeners, this will stop the event at the
    /// node that is current handling it, regardless of the current phase of event dispatch.
    pub const fn stop_propagation(&mut self) {
        self.propagating = false;
    }

    /// Requests a redraw after the event is processed.
    ///
    /// If a redraw has already been set, it will be updated only if the new redraw
    /// is more urgent than the current one.
    pub fn request_redraw(&mut self, new_redraw: Redraw) {
        self.redraw = Some(match self.redraw {
            Some(current_redraw) => current_redraw.retain_longest(new_redraw),
            None => new_redraw,
        });
    }

    /// Requests keyboard focus for the given node.
    ///
    /// If a node prior in the event propagation chain has requested focus, then focus will be only
    /// requested for that prior node instead of the new node provided.
    pub fn request_keyboard_focus(&mut self, node: NodeId) {
        if self.requesting_kb_focus.is_none() {
            self.requesting_kb_focus = Some(node);
        }
    }
    /// Requests mouse capture for the given node.
    ///
    /// If a node prior in the event propagation chain has requested mouse capture, then the
    /// capture will only be requested for that prior node instead of the new node provided.
    pub fn request_mouse_capture(&mut self, node: NodeId) {
        if self.requesting_mouse_capture.is_none() {
            self.requesting_mouse_capture = Some(node);
        }
    }
    pub const fn request_mouse_release(&mut self) {
        self.requesting_mouse_release = true;
    }

    /// Pushes messages to be handled later, associated with the current node.
    pub fn push_messages(&mut self, messages: Vec<M>) {
        self.messages.push((self.current_node, messages))
    }
    /// Pushes messages to be handled later, associated with the current node.
    pub fn push_tree_command(&mut self, command: TreeCommand) {
        self.tree_commands.push(command)
    }
    /// Pushes messages to be handled later, associated with the current node.
    pub fn extend_tree_commands(&mut self, commands: &[TreeCommand]) {
        self.tree_commands.extend_from_slice(commands)
    }

    /// The current phase of the event dispatch lifecycle.
    ///
    /// See [`EventPhase`] for more detail on each lifecycle.
    pub const fn current_phase(&self) -> EventPhase {
        self.event_phase
    }

    /// Returns `true` if the event supports bubbling.
    pub const fn bubbles(&self) -> bool {
        self.bubbles
    }

    /// The target node of this event
    pub const fn target_node(&self) -> NodeId {
        self.target_node
    }

    /// The current node that is handling the event
    pub const fn current_node(&self) -> NodeId {
        self.current_node
    }

    /// Whether a redraw is required after processing this event
    pub const fn is_requesting_redraw(&self) -> Option<Redraw> {
        self.redraw
    }
    /// Whether a node is requesting keyboard focus.
    pub const fn is_requesting_kb_focus(&self) -> Option<NodeId> {
        self.requesting_kb_focus
    }
    /// Whether a node is requesting mouse capture.
    pub const fn is_requesting_mouse_capture(&self) -> Option<NodeId> {
        self.requesting_mouse_capture
    }
    /// Whether the event is requesting mouse release.
    ///
    /// As captured events are directly sent to the node (no event propagation), this indicates
    /// that the target node requested the mouse capture release.
    pub const fn is_requesting_mouse_release(&self) -> bool {
        self.requesting_mouse_release
    }

    /// Returns a reference to the inner event payload.
    ///
    /// This could be a standard event such as a mouse event, or a custom defined event.
    pub const fn payload(&self) -> &E {
        &self.payload
    }
}

impl<E, M: Clone> EventContext<E, M> {
    pub(crate) const fn new(payload: E, bubbles: bool, target_node: NodeId) -> Self {
        Self {
            payload,

            event_phase: EventPhase::Capturing,
            bubbles,

            target_node,
            current_node: target_node,

            redraw: None,
            requesting_kb_focus: None,
            requesting_mouse_capture: None,
            requesting_mouse_release: false,

            messages: vec![],
            tree_commands: vec![],
            events: vec![],
            propagating: true,
        }
    }

    /// Create an event contexts for a given payload and node,
    /// direct events have a [`EventPhase::Direct`], and do not bubble.
    pub(crate) const fn direct(inner: E, node: NodeId) -> Self {
        Self {
            payload: inner,

            event_phase: EventPhase::Direct,
            bubbles: false,

            target_node: node,
            current_node: node,

            redraw: None,
            requesting_kb_focus: None,
            requesting_mouse_capture: None,
            requesting_mouse_release: false,

            messages: vec![],
            tree_commands: vec![],
            events: vec![],
            propagating: false,
        }
    }
    pub(crate) const fn set_phase(&mut self, phase: EventPhase) {
        self.event_phase = phase;
    }
    pub(crate) const fn set_current_node(&mut self, node: NodeId) {
        self.current_node = node;
    }
    pub(crate) const fn is_propagating(&self) -> bool {
        self.propagating
    }
    /// Pushes messages to be handled later, associated with the current node.
    pub(crate) fn push_event(&mut self, event: TreeEvent) {
        self.events.push((self.current_node, event))
    }
    pub(crate) fn tree_commands(&self) -> &Vec<TreeCommand> {
        &self.tree_commands
    }

    pub(crate) fn into_result(self) -> EventResult<M> {
        EventResult {
            redraw: self.redraw,
            messages: self.messages,
            events: self.events,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EventResult<M: Clone> {
    /// If set, indicates that a redraw is requested after event processing.
    redraw: Option<Redraw>,
    /// Accumulated messages emitted from all events dispatched.
    messages: Vec<(NodeId, Vec<M>)>,

    /// New events needing to be dispatched.
    events: Vec<(NodeId, TreeEvent)>,
}

impl<M: Clone> EventResult<M> {
    pub(crate) const fn empty() -> Self {
        Self {
            redraw: None,
            messages: vec![],
            events: vec![],
        }
    }

    pub(crate) fn take_new_events(&mut self) -> Vec<(NodeId, TreeEvent)> {
        std::mem::take(&mut self.events)
    }

    pub(crate) fn combine(&mut self, other: Self) {
        if let Some(redraw_other) = other.redraw {
            if let Some(ref mut redraw_self) = self.redraw {
                redraw_self.retain_longest(redraw_other);
            } else {
                self.redraw = Some(redraw_other);
            }
        }
        self.messages.extend(other.messages);
        self.events.extend(other.events);
    }

    pub const fn messages(&self) -> &Vec<(NodeId, Vec<M>)> {
        &self.messages
    }
    /// Whether a redraw is required after processing this event
    pub const fn is_requesting_redraw(&self) -> Option<Redraw> {
        self.redraw
    }
}
