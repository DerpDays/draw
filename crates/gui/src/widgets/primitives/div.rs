use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use color::AlphaColor;
use euclid::default::{Point2D, Size2D};
use graphics::{BasicColor, Primitive, Rounding, primitives::Rectangle};
use sycamore_reactive::{MaybeDyn, create_effect};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{AnimationHandle, MeasureCtx, reexports::reactive::maybe_get_untracked};

use crate::{
    TreeManager,
    tree::{Widget, builder::ElementBuilder},
};

pub struct Div {
    background: Option<BackgroundDiv>,
}

struct BackgroundDiv {
    options: MaybeDyn<DivOptions>,

    transition_duration: Option<Duration>,
    transition_state: Option<TransitionState>,

    last_options: DivOptions,
}

struct TransitionState {
    start: Instant,

    from: DivOptions,
    to: DivOptions,

    last_set: DivOptions,

    _animation_handle: AnimationHandle,
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct DivOptions {
    pub bg_color: Option<BasicColor>,
    pub stroke_color: Option<BasicColor>,
    pub stroke_width: Option<f32>,
    pub rounding: Option<Rounding>,
}
impl From<DivOptions> for MaybeDyn<DivOptions> {
    fn from(value: DivOptions) -> Self {
        MaybeDyn::Static(value)
    }
}
impl DivOptions {
    pub const fn bg_color(&self) -> BasicColor {
        if let Some(color) = self.bg_color {
            color
        } else {
            BasicColor::Solid(AlphaColor::TRANSPARENT)
        }
    }
    pub const fn stroke_color(&self) -> BasicColor {
        if let Some(color) = self.stroke_color {
            color
        } else {
            BasicColor::Solid(AlphaColor::TRANSPARENT)
        }
    }
    pub const fn stroke_width(&self) -> f32 {
        if let Some(width) = self.stroke_width {
            width
        } else {
            0.
        }
    }
    pub const fn rounding(&self) -> Rounding {
        if let Some(rounding) = self.rounding {
            rounding
        } else {
            Rounding::ZERO
        }
    }
    pub fn to_rect(&self, origin: Point2D<f32>, size: Size2D<f32>) -> Rectangle {
        Rectangle {
            origin,
            size,
            rounding: self.rounding(),
            color: self.bg_color(),
            stroke_color: self.stroke_color(),
            stroke_width: self.stroke_width(),
        }
    }
    pub fn lerp(&self, other: &Self, t: f32) -> DivOptions {
        Self {
            bg_color: Some(self.bg_color().lerp(other.bg_color(), t)),
            stroke_color: Some(self.stroke_color().lerp(other.stroke_color(), t)),
            stroke_width: Some(
                self.stroke_width() + ((other.stroke_width() - self.stroke_width()) * t),
            ),
            rounding: Some(self.rounding().lerp(&other.rounding(), t)),
        }
    }
}

impl Widget for Div {
    fn render(&mut self, layout: &Layout, _style: &Style) -> Option<Primitive> {
        let bg = self.background.as_mut()?;

        let mut options = maybe_get_untracked(&bg.options);

        if options != bg.last_options {
            if let Some(state) = &mut bg.transition_state {
                // only start a new transition if the target changed
                // otherwise do nothing — let the existing transition continue
                if options != state.to {
                    state.start = Instant::now();
                    state.from = state.last_set;
                    state.to = options;
                }
            } else if bg.transition_duration.is_some() {
                let mgr = TreeManager::global();
                // start a new transition from last_options
                bg.transition_state = Some(TransitionState {
                    start: Instant::now(),
                    from: bg.last_options,
                    to: options,
                    last_set: bg.last_options,
                    _animation_handle: mgr.new_animation_handle(),
                });
            } else {
                // no transition: snap immediately
                bg.last_options = options;
            }
        }
        // If we are in a transition, interpolate
        if let Some(mut state) = bg.transition_state.take() {
            let elapsed = state.start.elapsed();
            let duration = bg
                .transition_duration
                .expect("should only have a transition state when a transition duration is set");

            options = if elapsed < duration {
                let t = elapsed.div_duration_f32(duration);
                let lerped = state.from.lerp(&state.to, t);
                state.last_set = lerped;
                bg.transition_state = Some(state);
                lerped
            } else {
                bg.last_options = state.to;
                state.to
            };
        };

        Some(Primitive::Rectangle(options.to_rect(
            Point2D::new(layout.location.x, layout.location.y),
            Size2D::new(layout.size.width, layout.size.height),
        )))
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
        "Div"
    }

    fn focusable(&self) -> bool {
        false
    }
}

pub fn div() -> ElementBuilder<Div> {
    ElementBuilder::new(Div { background: None })
}

impl ElementBuilder<Div> {
    pub fn options(self, options: impl Into<MaybeDyn<DivOptions>>) -> Self {
        let options = options.into();
        let bg_div = {
            let _box_sizing = taffy::BoxSizing::default();
            let last_options = maybe_get_untracked(&options);
            // let rect = Rectangle::new(Point2D::zero(), Size2D::zero(), last_options);
            BackgroundDiv {
                options: options.clone(),
                // inner: rect,
                transition_duration: None,
                transition_state: None,

                last_options,
            }
        };

        let inner = Div {
            background: Some(bg_div),
        };
        self.set_inner(inner).append_after_build(move |_| {
            let mgr = TreeManager::global();
            let options = options.clone();

            let first_run = Cell::new(true);
            create_effect(move || {
                options.track();
                if !first_run.get() {
                    log::debug!("updating div options");
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            });
        })
    }

    pub fn transition_duration(mut self, duration: impl Into<Duration>) -> Self {
        let duration = duration.into();
        if let Some(bg) = &mut self.inner_mut().background {
            bg.transition_duration = Some(duration);
        };
        self
    }
}
