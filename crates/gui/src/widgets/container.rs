use graphics::{get_empty_mesh, Mesh, Systems, Vertex};
use input::{KeyboardEvent, MouseEvent};

use crate::{
    events::{BlurEvent, EventContext, EventHandler, FocusEvent, HandlesEvent},
    widgets::Widget,
    Element,
};

#[derive(Clone)]
pub struct ContainerWidget<M: Clone> {
    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
    pub focus_handler: EventHandler<FocusEvent, Self>,
    pub blur_handler: EventHandler<BlurEvent, Self>,

    focusable: bool,
}

crate::macros::event_handlers::impl_event_handler! {
    ContainerWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
    FocusEvent => focus_handler,
    BlurEvent => blur_handler,
}

impl<M: Clone> ContainerWidget<M> {
    pub fn new(focusable: bool) -> Self {
        Self {
            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
            focus_handler: EventHandler::none(),
            blur_handler: EventHandler::none(),

            focusable,
        }
    }
}

impl<M: Clone> Element for ContainerWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Container(self)
    }

    fn render(&mut self, _systems: &mut Systems, _layout: taffy::Layout) -> &Mesh<Vertex> {
        get_empty_mesh()
    }

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        self.mouse_handler.clone().handle(self, ctx);
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
        false
    }

    fn clear_cache(&mut self) {
        tracing::warn!("called clear_cache on a container widget, which don't have meshes");
    }

    fn focusable(&self) -> bool {
        self.focusable
    }
}
