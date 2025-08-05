use euclid::default::Point2D;
use graphics::{primitives, Drawable, Mesh, Systems, Vertex, ViewportCoordinates};
use input::{KeyboardEvent, MouseEvent, MouseEventKind};

use crate::{
    events::{EventContext, EventHandler},
    macros::event_handlers::impl_event_handler,
    widgets::Widget,
    Element,
};

#[derive(Clone)]
pub struct SvgWidget<M: Clone> {
    inner: primitives::Svg<ViewportCoordinates>,
    options: SvgOptions,

    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,

    pub hovered: bool,
}

impl_event_handler! {
    SvgWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
}

#[derive(Clone, Copy, Debug)]
pub struct SvgOptions {
    pub normal: primitives::SvgOptions,
    pub hover: Option<primitives::SvgOptions>,
}

impl<M: Clone> SvgWidget<M> {
    pub fn new(data: Vec<u8>, options: SvgOptions) -> Self {
        let inner = graphics::primitives::Svg::new(
            Point2D::zero(),
            lyon::math::Size::zero(),
            data,
            options.normal,
        );
        Self {
            inner,
            options,

            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),

            hovered: false,
        }
    }
}

impl<M: Clone> Element for SvgWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::Svg(self)
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
                self.inner.update_rect(
                    Point2D::new(layout.location.x, layout.location.y),
                    lyon::math::Size::new(layout.size.width, layout.size.height),
                );
            }

            self.layout = layout;
        }
        self.inner.render(systems)
    }
    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        match ctx.payload().kind {
            MouseEventKind::Enter => {
                self.hovered = true;
                self.inner
                    .update_options(self.options.hover.unwrap_or(self.options.normal));
            }
            MouseEventKind::Leave => {
                self.hovered = false;
                self.inner.update_options(self.options.normal);
            }
            _ => {}
        };

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
