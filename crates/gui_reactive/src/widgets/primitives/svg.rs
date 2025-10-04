use euclid::default::{Point2D, Size2D};
use graphics::{primitives::SvgOptions, Drawable, Mesh, Systems, Vertex, ViewportCoordinates};
use reactive_graph::{
    effect::Effect,
    traits::{Get, GetUntracked},
    wrappers::read::Signal,
};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    tree::{ElementBuilder, Widget},
    TreeManager,
};

pub struct Svg {
    options: Signal<SvgOptions>,
    inner: graphics::primitives::Svg<ViewportCoordinates>,
    last_options: SvgOptions,
    last_layout: taffy::Layout,
}

impl Widget for Svg {
    fn render(
        &mut self,
        mesh: &mut Mesh<Vertex>,
        systems: &mut Systems,
        layout: &Layout,
        _: &Style,
    ) {
        let current_options = self.options.get_untracked();

        if current_options != self.last_options {
            self.last_options = current_options;
            self.inner.update_options(current_options);
        };

        if &self.last_layout != layout {
            self.last_layout = *layout;
            self.inner.update_rect(
                Point2D::new(layout.location.x, layout.location.y),
                Size2D::new(layout.size.width, layout.size.height),
            )
        }
        tracing::info!("rendering svg!!!!");
        mesh.append(self.inner.render(systems));
    }
    fn measure(
        &mut self,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        let res = known_dimensions.unwrap_or(Size::zero());
        tracing::info!("measuring to {res:?}");
        res
    }
    fn debug_label(&self) -> &'static str {
        "Svg"
    }

    fn focusable(&self) -> bool {
        false
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self as &dyn std::any::Any
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self as &mut dyn std::any::Any
    }
}

pub fn svg(bytes: Vec<u8>) -> ElementBuilder<Svg> {
    ElementBuilder::new(Svg {
        inner: graphics::primitives::Svg::new(
            Point2D::zero(),
            Size2D::zero(),
            bytes.to_vec(),
            SvgOptions::default(),
        ),
        options: SvgOptions::default().into(),
        last_options: SvgOptions::default(),
        last_layout: Layout::new(),
    })
}

impl ElementBuilder<Svg> {
    pub fn options(mut self, options: impl Into<Signal<SvgOptions>>) -> Self {
        let options = options.into();
        self.inner_mut()
            .inner
            .update_options(options.get_untracked());
        self.append_after_build(move |_| {
            let mgr = TreeManager::global();
            Effect::watch_sync(
                move || options.get(),
                move |new, old, _| {
                    if Some(new) != old {
                        tracing::debug!("updating svg options");
                        mgr.now();
                    }
                },
                false,
            );
        })
    }
}
