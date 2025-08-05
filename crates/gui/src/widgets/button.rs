use std::time::Duration;

use graphics::{primitives::RectangleOptions, Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseButton, MouseEvent, MouseEventKind};

use crate::{
    events::{BlurEvent, EventContext, EventHandler, EventPhase, FocusEvent, HandlesEvent, Redraw},
    widgets::{
        background::TransitionBackgroundWidget, Widget, WidgetInteractionState, WidgetVisualState,
    },
    Element,
};

#[derive(Clone)]
pub struct ButtonWidget<M: Clone> {
    tessellation: Option<Mesh<Vertex>>,
    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
    pub focus_handler: EventHandler<FocusEvent, Self>,
    pub blur_handler: EventHandler<BlurEvent, Self>,

    options: ButtonOptions,
    background: TransitionBackgroundWidget<M>,
    transition_duration: Duration,

    state: WidgetInteractionState,
    last_visual_state: WidgetVisualState,

    is_event_target: bool,
}

pub const FADE_DURATION: Duration = Duration::from_millis(150);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonOptions {
    pub pressed: RectangleOptions,
    pub active: RectangleOptions,
    pub hovered: RectangleOptions,
    pub normal: RectangleOptions,
    pub disabled: RectangleOptions,
}

impl ButtonOptions {
    pub const DEFAULT: Self = Self {
        pressed: RectangleOptions::DEFAULT,
        active: RectangleOptions::DEFAULT,
        hovered: RectangleOptions::DEFAULT,
        normal: RectangleOptions::DEFAULT,
        disabled: RectangleOptions::DEFAULT,
    };
    pub const fn from_visual(&self, visual: WidgetVisualState) -> &RectangleOptions {
        match visual {
            WidgetVisualState::Pressed => &self.pressed,
            WidgetVisualState::Active => &self.active,
            WidgetVisualState::Hovered => &self.hovered,
            WidgetVisualState::Normal => &self.normal,
            WidgetVisualState::Disabled => &self.disabled,
        }
    }
}

crate::macros::event_handlers::impl_event_handler! {
    ButtonWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
    FocusEvent => focus_handler,
    BlurEvent => blur_handler,
}

impl<M: Clone> ButtonWidget<M> {
    pub fn new(
        options: ButtonOptions,
        focusable: bool,
        active: bool,
        transition_duration: Duration,
    ) -> Self {
        let state = WidgetInteractionState::new(active, false, false, true);
        Self {
            tessellation: None,
            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
            focus_handler: EventHandler::none(),
            blur_handler: EventHandler::none(),

            options,
            background: TransitionBackgroundWidget::new(
                options.from_visual(state.to_visual()).clone(),
            ),
            transition_duration,

            state,
            last_visual_state: state.to_visual(),

            is_event_target: focusable,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.state.active = active;
        self.update_visual_state();
    }
    fn update_visual_state(&mut self) -> bool {
        if self.last_visual_state != self.state.to_visual() {
            self.clear_cache();
            self.background.change_options(
                *self.options.from_visual(self.state.to_visual()),
                Some(self.transition_duration),
            );
            self.last_visual_state = self.state.to_visual();
            true
        } else {
            false
        }
    }
}

impl<M: Clone> Element for ButtonWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Button(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            self.clear_cache();
            self.layout = layout;
        }
        if let Some(ref cache) = self.tessellation {
            return cache;
        }

        if self.background.is_mid_transition() {
            self.background.render(systems, layout)
        } else {
            self.tessellation = Some(self.background.render(systems, layout).clone());
            self.tessellation.as_ref().unwrap()
        }
    }

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        if ctx.current_phase() != EventPhase::Capturing {
            match ctx.payload().kind {
                MouseEventKind::Enter => self.state.hovered = true,
                MouseEventKind::Leave => {
                    self.state.pressed = false;
                    self.state.hovered = false;
                }
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    ctx.request_mouse_capture(ctx.current_node());
                    self.state.pressed = true
                }
                MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                    ctx.request_mouse_release();
                    self.state.pressed = false
                }
                _ => {}
            };
        }

        self.mouse_handler.clone().handle(self, ctx);

        if self.update_visual_state() {
            ctx.request_redraw(Redraw::Duration(self.transition_duration));
        }
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>) {
        self.keyboard_handler.clone().handle(self, ctx);
    }
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent, Self::Message>) {
        self.focus_handler.clone().handle(self, ctx);
    }
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent, Self::Message>) {
        self.blur_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        self.tessellation.is_none() || self.background.is_dirty()
    }

    fn clear_cache(&mut self) {
        self.tessellation = None;
        self.background.clear_cache();
    }

    fn focusable(&self) -> bool {
        self.is_event_target
    }
}
