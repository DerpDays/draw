use euclid::default::{Box2D, Point2D, Size2D};
use graphics::{
    Drawable, Mesh, Systems, Vertex, ViewportCoordinates,
    primitives::{Text, TextOptions},
};
use input::{KeyboardEvent, MouseEvent};

use crate::{
    Element,
    events::{EventContext, EventHandler},
    widgets::{LayoutChange, Widget, parse_layout_change},
};

#[derive(Clone)]
pub struct TextWidget<M: Clone> {
    inner: Text<ViewportCoordinates>,

    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
}

crate::macros::event_handlers::impl_event_handler! {
    TextWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
}

impl<M: Clone> TextWidget<M> {
    pub fn new(data: String, options: TextOptions) -> Self {
        let inner = Text::new(data, options, Box2D::zero());
        Self {
            inner,

            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
        }
    }

    /// # CORRECTNESS
    /// You must remeasure the layout after calling this function.
    pub fn update_content(&mut self, content: String) {
        self.inner.set_content(content);
    }
}

impl<M: Clone> Element for TextWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Text(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            match parse_layout_change(layout, self.layout) {
                LayoutChange::Translate(dx) => {
                    self.inner.translate(dx);
                }
                LayoutChange::Rerender => {
                    tracing::info!("rerendering text");
                    self.inner.update_rect(Box2D::from_origin_and_size(
                        Point2D::new(layout.location.x, layout.location.y),
                        Size2D::new(layout.size.width, layout.size.height),
                    ));
                }
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
