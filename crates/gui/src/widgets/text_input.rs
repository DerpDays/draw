use euclid::default::{Box2D, Point2D, Size2D};
use graphics::{
    primitives::{self, TextOptions},
    Drawable, Mesh, Systems, Vertex, ViewportCoordinates,
};
use input::{KeyboardEvent, MouseEvent};

use crate::macros::event_handlers::impl_event_handler;
use crate::{
    prelude::{Element, EventContext, EventHandler},
    widgets::Widget,
};

#[derive(Clone)]
pub struct TextInputWidget<M: Clone> {
    inner: primitives::Text<ViewportCoordinates>,

    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
}

impl_event_handler! {
    TextInputWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
}

impl<M: Clone> TextInputWidget<M> {
    pub fn new(data: String, options: TextOptions) -> Self {
        let inner = graphics::primitives::Text::new(data, options, Box2D::zero());
        Self {
            inner,

            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
        }
    }
    pub fn update_content(&mut self, content: String) {
        self.inner.set_content(content);
    }
}

impl<M: Clone> Element for TextInputWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::TextInput(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            let mut merged_layout = self.layout;
            merged_layout.location = layout.location;

            if merged_layout == layout {
                // if only the position changed, move the origin of the svg.
                self.inner.translate(
                    Point2D::new(layout.location.x, layout.location.y)
                        - Point2D::new(self.layout.location.x, self.layout.location.y),
                );
            } else {
                // otherwise we need to completely update the svg, which means re-tessellating.
                self.inner.update_rect(Box2D::from_origin_and_size(
                    Point2D::new(layout.location.x, layout.location.y),
                    Size2D::new(layout.size.width, layout.size.height),
                ));
            }

            self.layout = layout;
        }
        self.inner.render(systems)
    }
    fn measure(
        &mut self,
        systems: &mut Systems,
        _: taffy::Size<taffy::AvailableSpace>,
        _: &taffy::Style,
    ) -> taffy::Size<f32> {
        let size = self.inner.measure(systems);
        taffy::Size {
            width: size.width,
            height: size.height,
        }
    }

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        self.mouse_handler.clone().handle(self, ctx);
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>) {
        self.keyboard_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    fn focusable(&self) -> bool {
        false
    }
}
