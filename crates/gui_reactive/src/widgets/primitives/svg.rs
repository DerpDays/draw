use std::cell::Cell;

use euclid::default::{Point2D, Size2D};
use graphics::{primitives::SvgOptions, Drawable, Mesh, Systems, Vertex, ViewportCoordinates};
use sycamore_reactive::{create_effect, MaybeDyn, ReadSignal, Signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::reexports::reactive::maybe_get_untracked;

use crate::{
    tree::{builder::ElementBuilder, Widget},
    widgets::LayoutEqNoLocation,
    TreeManager,
};

pub struct Svg {
    options: MaybeDyn<OptionsWrapper>,
    inner: graphics::primitives::Svg<ViewportCoordinates>,
    last_options: OptionsWrapper,
    last_layout: taffy::Layout,
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct OptionsWrapper(SvgOptions);

impl From<OptionsWrapper> for SvgOptions {
    fn from(value: OptionsWrapper) -> Self {
        value.0
    }
}

pub struct MaybeDynSvgOptions(MaybeDyn<OptionsWrapper>);
impl MaybeDynSvgOptions {
    #[inline(always)]
    pub fn get(self) -> MaybeDyn<OptionsWrapper> {
        self.0
    }
}

impl From<OptionsWrapper> for MaybeDyn<OptionsWrapper> {
    fn from(value: OptionsWrapper) -> Self {
        Self::Static(value)
    }
}

impl From<SvgOptions> for MaybeDynSvgOptions {
    fn from(value: SvgOptions) -> Self {
        Self(MaybeDyn::Static(OptionsWrapper(value)))
    }
}

impl From<Signal<OptionsWrapper>> for MaybeDynSvgOptions {
    fn from(value: Signal<OptionsWrapper>) -> Self {
        let (read_signal, _) = value.split();
        Self(MaybeDyn::Signal(read_signal))
    }
}
impl From<ReadSignal<OptionsWrapper>> for MaybeDynSvgOptions {
    fn from(value: ReadSignal<OptionsWrapper>) -> Self {
        Self(MaybeDyn::Signal(value))
    }
}

impl Widget for Svg {
    fn render(
        &mut self,
        mesh: &mut Mesh<Vertex>,
        systems: &mut Systems,
        layout: &Layout,
        _: &Style,
    ) {
        let current_options = maybe_get_untracked(&self.options);

        if current_options != self.last_options {
            self.last_options = current_options;
            self.inner.update_options(current_options.into());
        };

        if &self.last_layout != layout {
            if layout.eq_no_location(&self.last_layout) {
                self.inner
                    .translate(layout.location_delta(&self.last_layout));
            } else {
                self.inner.update_rect(
                    Point2D::new(layout.location.x, layout.location.y),
                    Size2D::new(layout.size.width, layout.size.height),
                );
            };
            self.last_layout = *layout;
        }
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
}

pub fn svg(bytes: Vec<u8>) -> ElementBuilder<Svg> {
    ElementBuilder::new(Svg {
        inner: graphics::primitives::Svg::new(
            Point2D::zero(),
            Size2D::zero(),
            bytes.to_vec(),
            SvgOptions::default(),
        ),
        options: MaybeDyn::Static(OptionsWrapper::default()),
        last_options: OptionsWrapper::default(),
        last_layout: Layout::new(),
    })
}

impl ElementBuilder<Svg> {
    pub fn options(mut self, options: impl Into<MaybeDynSvgOptions>) -> Self {
        let options = options.into().get();
        self.inner_mut().inner.update_options(options.get().into());
        self.append_after_build(move |_| {
            let mgr = TreeManager::global();
            let options = options.clone();
            let first_run = Cell::new(true);
            create_effect(move || {
                options.track();
                if !first_run.get() {
                    tracing::debug!("updating svg options");
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            });
        })
    }
}
