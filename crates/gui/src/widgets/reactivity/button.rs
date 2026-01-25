use graphics::Primitive;
use input::{MouseButton, MouseEvent, MouseEventKind};
use sycamore_reactive::{ReadSignal, Signal, batch, create_memo, create_signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    MeasureCtx,
    prelude::EventContext,
    tree::{Widget, builder::ElementBuilder},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ButtonVisualState {
    /// Pressed is the state with the most visual priority, it represents that the widget
    /// is currently being pressed by a left mouse click.
    Pressed,
    /// Active is a special state for widgets that can be toggled on/off at the user's discretion.
    /// This is meant to represent when a widget is in a special state such as being the current
    /// tool, menu currently selected, or otherwise.
    Active,
    /// Represents that the mouse is currently inside this widget (or mouse events are being shared
    /// to it).
    Hovered,
    /// The normal state of the widget when it has neither mouse or keyboard focus.
    Normal,
    /// A special state for when the widget is marked as disabled.
    Disabled,
}

pub struct Button {
    state: ButtonSignals,
}

#[derive(Copy, Clone)]
pub struct ButtonSignals {
    enabled: ReadSignal<bool>,
    active: ReadSignal<bool>,

    pressed: Signal<bool>,
    hovered: Signal<bool>,
}

impl ButtonSignals {
    /// Create a read signal that reacts to changes in the visual state
    pub fn to_visual(self) -> ReadSignal<ButtonVisualState> {
        let Self {
            enabled,
            active,
            pressed,
            hovered,
        } = self;
        create_memo(move || {
            if !enabled.get() {
                ButtonVisualState::Disabled
            } else if pressed.get() {
                ButtonVisualState::Pressed
            } else if active.get() {
                ButtonVisualState::Active
            } else if hovered.get() {
                ButtonVisualState::Hovered
            } else {
                ButtonVisualState::Normal
            }
        })
    }
}

impl Widget for Button {
    fn render(&mut self, _: &Layout, _: &Style) -> Option<Primitive> {
        None
    }
    fn measure(
        &mut self,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Button"
    }

    fn focusable(&self) -> bool {
        true
    }

    fn default_mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>, _: &Layout) {
        if !ctx.in_capture_phase() {
            log::info!("default mouse event!!");
            if !self.state.enabled.get_untracked() {
                self.state.pressed.set(false);
                self.state.hovered.set(false);
                return;
            }
            match ctx.payload().kind {
                MouseEventKind::Enter => batch(|| {
                    self.state.pressed.set(false);
                    self.state.hovered.set(true);
                }),
                MouseEventKind::Leave => batch(|| {
                    self.state.pressed.set(false);
                    self.state.hovered.set(false);
                }),
                MouseEventKind::Press {
                    button: MouseButton::Left,
                    ..
                } => {
                    self.state.pressed.set(true);
                }
                MouseEventKind::Release {
                    button: MouseButton::Left,
                    ..
                } => {
                    self.state.pressed.set(false);
                }
                _ => {}
            }
        }
    }
}

pub fn button() -> (ElementBuilder<Button>, ButtonSignals) {
    let state = ButtonSignals {
        enabled: create_memo(|| true),
        active: create_memo(|| false),
        pressed: create_signal(false),
        hovered: create_signal(false),
    };
    (ElementBuilder::new(Button { state }), state)
}

pub fn button_with(
    enabled: ReadSignal<bool>,
    active: ReadSignal<bool>,
) -> (ElementBuilder<Button>, ButtonSignals) {
    let state = ButtonSignals {
        enabled,
        active,
        pressed: create_signal(false),
        hovered: create_signal(false),
    };
    (ElementBuilder::new(Button { state }), state)
}
