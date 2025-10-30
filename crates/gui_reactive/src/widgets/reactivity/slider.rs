use graphics::{Mesh, Systems, Vertex};
use input::{MouseButton, MouseEvent, MouseEventKind};
use sycamore_reactive::{batch, create_memo, create_signal, ReadSignal, Signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    prelude::EventContext,
    tree::{builder::ElementBuilder, Widget},
    widgets::reactivity::slider::sealed::BoundedRange,
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SliderVisualState {
    /// Pressed is the state with the most visual priority, it represents that the widget
    /// is currently being pressed by a left mouse click.
    Pressed,
    /// Represents that the mouse is currently inside this widget (or mouse events are being shared
    /// to it).
    Hovered,
    /// The normal state of the widget when it has neither mouse or keyboard focus.
    Normal,
    /// A special state for when the widget is marked as disabled.
    Disabled,
}

pub struct Slider {
    state: SliderSignals,
    steps: u32,
    inclusive_range: (f32, f32),
}

#[derive(Copy, Clone)]
pub struct SliderSignals {
    enabled: ReadSignal<bool>,

    pressed: Signal<bool>,
    hovered: Signal<bool>,

    value: Signal<f32>,
}

impl SliderSignals {
    /// Create a read signal that reacts to changes in the visual state
    pub fn to_visual(&self) -> ReadSignal<SliderVisualState> {
        let Self {
            enabled,
            pressed,
            hovered,
            ..
        } = *self;
        create_memo(move || {
            if !enabled.get() {
                return SliderVisualState::Disabled;
            } else if pressed.get() {
                return SliderVisualState::Pressed;
            } else if hovered.get() {
                return SliderVisualState::Hovered;
            } else {
                SliderVisualState::Normal
            }
        })
    }
}

impl Widget for Slider {
    fn render(&mut self, _: &mut Mesh<Vertex>, _: &mut Systems, _: &Layout, _: &Style) {}
    fn measure(
        &mut self,
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

    fn default_mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>, layout: &Layout) {
        if !ctx.in_capture_phase() {
            tracing::info!("default mouse event!!");
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
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    self.state.pressed.set(true);
                    ctx.request_mouse_capture(ctx.current_node());
                }
                MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                    self.state.pressed.set(false);
                    ctx.request_mouse_release();
                }
                MouseEventKind::Motion { .. } => {
                    if self.state.pressed.get_untracked() {
                        let (x, max_x) = (layout.location.x, layout.location.x + layout.size.width);

                        let pos = ctx.payload().position;

                        let percentage = ((pos.x - x) / max_x).clamp(0., 1.);

                        let range = self.inclusive_range.1 - self.inclusive_range.0;
                        let step_size = range / (self.steps) as f32;
                        let step_index = (percentage * (self.steps) as f32).round();

                        let value = self.inclusive_range.0 + (step_size * step_index);

                        self.state.value.set(value);
                    }
                }
                _ => {}
            }
        }
    }
}

mod sealed {
    use std::ops::{Range, RangeInclusive};

    pub trait BoundedRange<T> {
        fn start_inclusive(&self) -> T;
        fn end_inclusive(&self) -> T;
    }
    impl BoundedRange<f32> for Range<f32> {
        fn start_inclusive(&self) -> f32 {
            self.start
        }
        fn end_inclusive(&self) -> f32 {
            self.end - 1.
        }
    }
    impl<T: Copy> BoundedRange<T> for RangeInclusive<T> {
        fn start_inclusive(&self) -> T {
            *self.start()
        }
        fn end_inclusive(&self) -> T {
            *self.end()
        }
    }
}

pub fn slider(
    initial_val: f32,
    steps: u32,
    range: impl BoundedRange<f32>,
) -> (ElementBuilder<Slider>, SliderSignals) {
    let state = SliderSignals {
        enabled: create_memo(|| true),
        pressed: create_signal(false),
        hovered: create_signal(false),
        value: create_signal(initial_val),
    };
    (
        ElementBuilder::new(Slider {
            state,
            steps,
            inclusive_range: (range.start_inclusive(), range.end_inclusive()),
        }),
        state,
    )
}

pub fn slider_with(
    steps: u32,
    range: impl BoundedRange<f32>,
    enabled: ReadSignal<bool>,
    value: Signal<f32>,
) -> (ElementBuilder<Slider>, SliderSignals) {
    let state = SliderSignals {
        enabled,

        pressed: create_signal(false),
        hovered: create_signal(false),

        value,
    };
    (
        ElementBuilder::new(Slider {
            state,
            steps,
            inclusive_range: (range.start_inclusive(), range.end_inclusive()),
        }),
        state,
    )
}
