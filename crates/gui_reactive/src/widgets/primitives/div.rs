use std::{
    cell::Cell,
    time::{Duration, Instant},
};

use color::AlphaColor;
use euclid::default::{Point2D, Size2D};
use graphics::{
    primitives::{Rectangle, RectangleOptions},
    BasicColor,
    BoxSizing,
    Drawable,
    Mesh,
    Rounding,
    Systems,
    Vertex,
    ViewportCoordinates,
};
use sycamore_reactive::{create_effect, MaybeDyn};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{reexports::reactive::maybe_get_untracked, AnimationHandle};

use crate::{
    tree::{builder::ElementBuilder, Widget},
    TreeManager,
};

pub struct Div {
    background: Option<BackgroundDiv>,
}

struct BackgroundDiv {
    options: MaybeDyn<DivOptions>,

    transition_duration: Option<Duration>,
    transition_state: Option<TransitionState>,

    inner: Rectangle<ViewportCoordinates>,
    last_options: RectangleOptions,
    last_layout: taffy::Layout,
}

struct TransitionState {
    start: Instant,
    from: RectangleOptions,
    to: RectangleOptions,
    last_set: RectangleOptions,
    _animation_handle: AnimationHandle,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
        let current_options = maybe_get_untracked(&bg.options).into_rect_options(&style.box_sizing);
        tracing::warn!("has duration: {:?}", bg.transition_duration);

        // If options changed, maybe start a transition
        if current_options != bg.last_options {
            tracing::warn!("has changed");
            if bg.transition_duration.is_some() {
                if let Some(state) = &mut bg.transition_state {
                    bg.last_options = state.last_set;
                    *state = TransitionState {
                        start: state.start,
                        from: state.last_set, // needs RectangleOptions -> DivOptions
                        to: current_options,
                        last_set: state.last_set,
                        _animation_handle: TreeManager::global().new_animation_handle(),
                    };
                } else {
                    bg.transition_state = Some(TransitionState {
                        start: Instant::now(),
                        from: bg.last_options, // needs RectangleOptions -> DivOptions
                        to: current_options,
                        last_set: bg.last_options,
                        _animation_handle: TreeManager::global().new_animation_handle(),
                    });
                }
            } else {
                bg.last_options = current_options;
                bg.inner.update_options(current_options);
            }
        }

        // If we are in a transition, interpolate
        if let Some(mut state) = bg.transition_state.take() {
            let elapsed = state.start.elapsed();
            let duration = bg
                .transition_duration
                .expect("should only have a transition state when a transition duration is set");

            let new_opts = if elapsed < duration {
                let t = elapsed.div_duration_f32(duration);
                tracing::error!("t is : {t:?}");
                let lerped = state.from.lerp(&state.to, t);
                state.last_set = lerped;
                bg.transition_state = Some(state);
                lerped
            } else {
                bg.transition_state = None;
                bg.last_options = current_options;
                state.to
            };

            bg.inner.update_options(new_opts);
        }

        if &bg.last_layout != layout {
            bg.last_layout = *layout;
            bg.inner.update_area(
                Point2D::new(layout.location.x, layout.location.y),
                Size2D::new(layout.size.width, layout.size.height),
            )
        }
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
            let box_sizing = taffy::BoxSizing::default();
            let last_options = maybe_get_untracked(&options).into_rect_options(&box_sizing);
            let rect = Rectangle::new(Point2D::zero(), Size2D::zero(), last_options);
            BackgroundDiv {
                options: options.clone(),
                inner: rect,

                transition_duration: None,
                transition_state: None,

                last_options,
                last_layout: Layout::new(),
            }
        };

        let inner = Div {
            background: Some(bg_div),
        };
        ElementBuilder::set_inner(self, inner).append_after_build(move |_| {
            let mgr = TreeManager::global();
            let options = options.clone();

            let first_run = Cell::new(true);
            create_effect(move || {
                options.track();
                if !first_run.get() {
                    tracing::debug!("updating div options");
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

// impl<M: Clone> TransitionBackgroundWidget<M> {
//     pub fn new(options: RectangleOptions) -> Self {
//         let rect = Rectangle::new(Point2D::zero(), Size2D::zero(), options.clone());
//         Self {
//             rect,
//             layout: taffy::Layout::new(),
//             render_cache: None,
//
//             mouse_handler: EventHandler::none(),
//             keyboard_handler: EventHandler::none(),
//
//             options,
//             prev_options: options,
//             transition_start: None,
//             transition_duration: Duration::ZERO,
//         }
//     }
//
//     fn update_render_cache(&mut self, systems: &mut Systems) -> &Mesh<Vertex> {
//         self.render_cache = Some(self.rect.render(systems).clone());
//         self.render_cache.as_ref().unwrap()
//     }
//
//     pub fn change_options(
//         &mut self,
//         options: RectangleOptions,
//         transition_duration: Option<Duration>,
//     ) {
//         if let Some(duration) = transition_duration {
//             self.transition_start = Some(Instant::now());
//             self.transition_duration = duration;
//         } else {
//             self.rect.update_options(options);
//             self.transition_start = None;
//         }
//         self.prev_options = self.rect.options().clone();
//         self.options = options;
//         self.render_cache = None;
//     }
//
//     pub fn is_mid_transition(&self) -> bool {
//         let Some(start_time) = self.transition_start else {
//             return false;
//         };
//         let elapsed = Instant::now() - start_time;
//         elapsed < self.transition_duration
//     }
// }
//
// impl<M: Clone> Element for TransitionBackgroundWidget<M> {
//     type Message = M;
//
//     fn as_widget(self) -> Widget<Self::Message> {
//         Widget::TransitionBackground(self)
//     }
//
//     fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
//         if self.layout != layout {
//             match parse_layout_change(layout, self.layout) {
//                 LayoutChange::Translate(dx) => {
//                     self.rect.translate(dx);
//                     self.render_cache.as_mut().map(|x| x.translate(dx));
//                 }
//                 LayoutChange::Rerender => {
//                     self.rect.update_area(
//                         Point2D::new(layout.location.x, layout.location.y),
//                         Size2D::new(layout.size.width, layout.size.height),
//                     );
//                     self.clear_cache();
//                 }
//             }
//         }
//         if let Some(ref cache) = self.render_cache {
//             return cache;
//         }
//         let Some(start_time) = self.transition_start else {
//             return self.update_render_cache(systems);
//         };
//
//         let elapsed = Instant::now() - start_time;
//
//         if elapsed < self.transition_duration {
//             let new_options = RectangleOptions {
//                 color: self.prev_options.color.lerp(
//                     self.options.color,
//                     elapsed.div_duration_f32(self.transition_duration),
//                 ),
//                 // TODO: lerp rest of attributes
//                 ..self.options
//             };
//             self.rect.update_options(new_options);
//             self.rect.render(systems)
//         } else {
//             self.transition_start = None;
//             self.rect.update_options(self.options);
//             self.update_render_cache(systems)
//         }
//     }
//     fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, M>) {
//         self.mouse_handler.clone().handle(self, ctx);
//     }
//
//     fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, M>) {
//         self.keyboard_handler.clone().handle(self, ctx);
//     }
//
//     fn is_dirty(&self) -> bool {
//         self.render_cache.is_none() || self.rect.is_dirty()
//     }
//
//     fn clear_cache(&mut self) {
//         self.render_cache = None;
//         self.rect.clear_cache();
//     }
//
//     fn focusable(&self) -> bool {
//         false
//     }
// }
