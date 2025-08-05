use std::time::{Duration, Instant};

use euclid::default::Point2D;
use graphics::{
    primitives::{Rectangle, RectangleOptions},
    Drawable, Mesh, Systems, Vertex, ViewportCoordinates,
};
use input::{KeyboardEvent, MouseEvent};

use crate::{
    events::{EventContext, EventHandler, HandlesEvent},
    widgets::Widget,
    Element,
};

#[derive(Clone)]
pub struct BackgroundWidget<M: Clone> {
    rect: Rectangle<ViewportCoordinates>,
    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
}

crate::macros::event_handlers::impl_event_handler! {
    BackgroundWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
}

impl<M: Clone> BackgroundWidget<M> {
    pub fn new(options: RectangleOptions) -> Self {
        let rect = Rectangle::new(Point2D::zero(), lyon::math::Size::zero(), options);
        Self {
            rect,
            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
        }
    }
    pub fn change_options(&mut self, options: RectangleOptions) {
        self.rect.update_options(options);
    }
}

impl<M: Clone> Element for BackgroundWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Background(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            self.layout = layout;
            self.rect.update_area(
                Point2D::new(layout.location.x, layout.location.y),
                lyon::math::Size::new(layout.size.width, layout.size.height),
            );
        }
        self.rect.render(systems)
    }
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, M>) {
        self.mouse_handler.clone().handle(self, ctx);
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, M>) {
        self.keyboard_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        self.rect.is_dirty()
    }

    fn clear_cache(&mut self) {
        self.rect.clear_cache();
    }

    fn focusable(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct TransitionBackgroundWidget<M: Clone> {
    rect: Rectangle<ViewportCoordinates>,
    tessellation: Option<Mesh<Vertex>>,
    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,

    options: RectangleOptions,
    prev_options: RectangleOptions,

    transition_start: Option<Instant>,
    transition_duration: Duration,
}

crate::macros::event_handlers::impl_event_handler! {
    TransitionBackgroundWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
}

impl<M: Clone> TransitionBackgroundWidget<M> {
    pub fn new(options: RectangleOptions) -> Self {
        let rect = Rectangle::new(Point2D::zero(), lyon::math::Size::zero(), options.clone());
        Self {
            rect,
            layout: taffy::Layout::new(),
            tessellation: None,

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),

            options,
            prev_options: options,
            transition_start: None,
            transition_duration: Duration::ZERO,
        }
    }

    fn update_tessellation_cache(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
        self.tessellation = Some(self.rect.render(systems).clone());
        self.tessellation.as_ref().unwrap()
    }

    pub fn change_options(
        &mut self,
        options: RectangleOptions,
        transition_duration: Option<Duration>,
    ) {
        if let Some(duration) = transition_duration {
            self.transition_start = Some(Instant::now());
            self.transition_duration = duration;
        } else {
            self.rect.update_options(options);
            self.transition_start = None;
        }
        self.prev_options = self.rect.options().clone();
        self.options = options;
        self.tessellation = None;
    }

    pub fn is_mid_transition(&self) -> bool {
        let Some(start_time) = self.transition_start else {
            return false;
        };
        let elapsed = Instant::now() - start_time;
        elapsed < self.transition_duration
    }
}

impl<M: Clone> Element for TransitionBackgroundWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::TransitionBackground(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            self.layout = layout;
            self.tessellation = None;
            self.rect.update_area(
                Point2D::new(layout.location.x, layout.location.y),
                lyon::math::Size::new(layout.size.width, layout.size.height),
            );
        }
        if let Some(ref cache) = self.tessellation {
            return cache;
        }
        let Some(start_time) = self.transition_start else {
            return self.update_tessellation_cache(systems);
        };

        let elapsed = Instant::now() - start_time;

        if elapsed < self.transition_duration {
            let new_options = RectangleOptions {
                color: self.prev_options.color.lerp(
                    self.options.color,
                    elapsed.div_duration_f32(self.transition_duration),
                ),
                // TODO: lerp rest of attributes
                ..self.options
            };
            self.rect.update_options(new_options);
            self.rect.render(systems)
        } else {
            self.transition_start = None;
            self.rect.update_options(self.options);
            self.update_tessellation_cache(systems)
        }
    }
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, M>) {
        self.mouse_handler.clone().handle(self, ctx);
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, M>) {
        self.keyboard_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        self.tessellation.is_none() || self.rect.is_dirty()
    }

    fn clear_cache(&mut self) {
        self.tessellation = None;
    }

    fn focusable(&self) -> bool {
        false
    }
}
