use std::{cell::Cell, sync::Arc};

use euclid::default::{Point2D, Size2D};
use graphics::{
    BasicColor,
    Primitive,
    primitives::{self},
};
use sycamore_reactive::{MaybeDyn, ReadSignal, create_effect};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    MeasureCtx,
    TreeManager,
    reexports::reactive::maybe_get_clone_untracked,
    tree::{Widget, builder::ElementBuilder},
};

pub struct Svg {
    pub data: ReadSignal<Arc<[u8]>>,
    options: MaybeDyn<SvgOptions>,
}

#[derive(Clone, Debug, Default)]
pub struct SvgOptions {
    pub fill_color: Option<BasicColor>,
    pub stroke_color: Option<BasicColor>,
}
impl SvgOptions {}
impl From<SvgOptions> for MaybeDyn<SvgOptions> {
    fn from(value: SvgOptions) -> Self {
        MaybeDyn::Static(value)
    }
}

impl Widget for Svg {
    fn render(&mut self, layout: &Layout, _: &Style) -> Option<Primitive> {
        let options = maybe_get_clone_untracked(&self.options);

        Some(Primitive::Svg(primitives::Svg {
            origin: Point2D::new(layout.location.x, layout.location.y),
            size: Size2D::new(layout.size.width, layout.size.height),

            data: self.data.get_clone_untracked(),
            fill_color: options.fill_color,
            stroke_color: options.stroke_color,
        }))
    }
    fn measure(
        &mut self,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }
    fn debug_label(&self) -> &'static str {
        "Svg"
    }

    fn focusable(&self) -> bool {
        false
    }
}

pub fn svg(data: ReadSignal<Arc<[u8]>>) -> ElementBuilder<Svg> {
    ElementBuilder::new_with_after_build(
        Svg {
            data,
            options: MaybeDyn::Static(SvgOptions::default()),
        },
        move |_| {
            let mgr = TreeManager::global();

            create_effect(move || {
                data.track();
                mgr.now();
            });
        },
    )
}

impl ElementBuilder<Svg> {
    pub fn options(self, options: impl Into<MaybeDyn<SvgOptions>>) -> Self {
        let options = options.into();
        let inner = Svg {
            data: self.inner().data,
            options: options.clone(),
        };
        self.set_inner(inner).append_after_build(move |_| {
            let mgr = TreeManager::global();
            let options = options.clone();

            let first_run = Cell::new(true);
            create_effect(move || {
                options.track();
                if !first_run.get() {
                    log::debug!("updating svg options");
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            });
        })
    }
}
