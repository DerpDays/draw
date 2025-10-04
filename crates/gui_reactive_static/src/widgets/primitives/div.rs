use color::AlphaColor;
use euclid::default::{Point2D, Size2D};
use graphics::{
    primitives::{Rectangle, RectangleOptions},
    BasicColor, BoxSizing, Drawable, Mesh, Rounding, Systems, Vertex, ViewportCoordinates,
};
use reactive_graph::{effect::Effect, prelude::Get, traits::GetUntracked, wrappers::read::Signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    tree::{DynNode, Element, ElementWithChildren, NodeForEach, Widget},
    TreeManager,
};

pub struct Div {
    background: Option<BackgroundDiv>,
}

struct BackgroundDiv {
    options: Signal<DivOptions>,
    inner: Rectangle<ViewportCoordinates>,
    last_options: RectangleOptions,
    last_layout: taffy::Layout,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DivOptions {
    pub bg_color: Option<BasicColor>,
    pub stroke_color: Option<BasicColor>,
    pub stroke_width: Option<f32>,
    pub rounding: Option<Rounding>,
}

impl DivOptions {
    fn into_rect_options(&self, box_sizing: &taffy::BoxSizing) -> RectangleOptions {
        RectangleOptions {
            color: self
                .bg_color
                .unwrap_or(BasicColor::Solid(AlphaColor::TRANSPARENT)),
            stroke_color: self
                .bg_color
                .unwrap_or(BasicColor::Solid(AlphaColor::TRANSPARENT)),
            stroke_width: self.stroke_width.unwrap_or(0.),
            rounding: self.rounding.unwrap_or(Rounding::DEFAULT),
            box_sizing: match box_sizing {
                taffy::BoxSizing::BorderBox => BoxSizing::BorderBox,
                taffy::BoxSizing::ContentBox => BoxSizing::ContentBox,
            },
        }
    }
}

impl ElementWithChildren for Div {}
impl Widget for Div {
    fn render(
        &mut self,
        mesh: &mut Mesh<Vertex>,
        systems: &mut Systems,
        layout: &Layout,
        style: &Style,
    ) {
        let Some(bg) = &mut self.background else {
            return;
        };
        let current_options = bg
            .options
            .get_untracked()
            .into_rect_options(&style.box_sizing);

        if current_options != bg.last_options {
            bg.last_options = current_options;
            bg.inner.update_options(current_options);
        };

        if &bg.last_layout != layout {
            bg.last_layout = *layout;
            bg.inner.update_area(
                Point2D::new(layout.location.x, layout.location.y),
                Size2D::new(layout.size.width, layout.size.height),
            )
        }
        tracing::info!("rendering div!!!! {:?}", bg.inner.size());
        mesh.append(bg.inner.render(systems));
    }
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Div"
    }

    fn focusable() -> bool {
        false
    }
}

pub fn div() -> Element<Div, ()> {
    Element::new_empty(Div { background: None })
}

impl<C: NodeForEach + 'static> Element<Div, C> {
    pub fn options(self, options: impl Into<Signal<DivOptions>>) -> Self {
        let options = options.into();
        let bg_div = {
            let box_sizing = taffy::BoxSizing::default();
            let last_options = options.get_untracked().into_rect_options(&box_sizing);
            let rect = Rectangle::new(Point2D::zero(), Size2D::zero(), last_options);
            BackgroundDiv {
                options,
                inner: rect,
                last_options,
                last_layout: Layout::new(),
            }
        };

        let inner = Div {
            background: Some(bg_div),
        };
        let elem = Element::update_inner(self, inner);
        let node_id = elem.node_id();

        let mgr = TreeManager::global();
        Effect::watch_sync(
            move || options.get(),
            move |new, old, _| {
                if Some(new) != old {
                    tracing::debug!("new text!!");
                    mgr.relayout(node_id);
                    mgr.now();
                }
            },
            false,
        );
        elem
    }
}
