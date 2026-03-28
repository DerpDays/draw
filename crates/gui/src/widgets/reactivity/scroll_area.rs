use color::AlphaColor;
use euclid::default::{Point2D, SideOffsets2D, Size2D, Vector2D};
use graphics::{Primitive, Rounding};
use input::{MouseButton, MouseEventKind};
use std::cell::Cell;
use sycamore_reactive::Signal;

use crate::prelude::{AvailableSpace, Layout, Size, Style};

use crate::{
    ElementId,
    MeasureCtx,
    TreeManager,
    events::EventPhase,
    tree::{
        Widget,
        builder::{ElementBuilder, HasChildren},
    },
};

#[derive(Debug)]
struct ScrollGeometry {
    scrollbar_w: f32,
    max_scroll: f32,
    bar_height: f32,
    track_height: f32,
    visible_ratio: f32,
}

impl ScrollGeometry {
    fn from_layout(layout: &Layout) -> Self {
        let scrollbar_w = layout.scrollbar_size.width;
        let viewport_h = layout.size.height - layout.border.top - layout.border.bottom;
        // With native scrolling (no inset/margin hack), taffy gives us the true content height.
        let content_h = layout.content_size.height;
        let max_scroll = (content_h - viewport_h).max(0.0);

        let visible_ratio = if content_h > 0.0 {
            (viewport_h / content_h).min(1.0)
        } else {
            1.0
        };
        let bar_height = (viewport_h * visible_ratio).max(MIN_SCROLLBAR_HEIGHT);
        let track_height = (viewport_h - bar_height).max(0.0);

        Self {
            scrollbar_w,
            max_scroll,
            bar_height,
            track_height,
            visible_ratio,
        }
    }

    fn scroll_to_px(&self, ratio: f32) -> f32 {
        (ratio * self.max_scroll).clamp(0.0, self.max_scroll)
    }

    fn px_to_ratio(&self, px: f32) -> f32 {
        if self.max_scroll > 0.0 {
            (px / self.max_scroll).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

pub struct ScrollArea {
    /// Scrolled amount in pixels
    scroll_amount: Signal<f32>,
}

const MIN_SCROLLBAR_HEIGHT: f32 = 12.;

impl HasChildren for ScrollArea {}
impl Widget for ScrollArea {
    fn render(&mut self, layout: &Layout, _: &Style) -> Option<Primitive> {
        let geo = ScrollGeometry::from_layout(layout);

        if geo.visible_ratio >= 1.0 || geo.scrollbar_w <= 0.0 {
            return None;
        }

        let bar_y = geo.track_height * geo.px_to_ratio(self.scroll_amount.get_untracked());

        Some(Primitive::Rectangle(graphics::primitives::Rectangle {
            origin: Point2D::new(
                layout.location.x + layout.size.width - geo.scrollbar_w - layout.border.right,
                layout.location.y + layout.border.top + bar_y,
            ),
            size: Size2D::new(geo.scrollbar_w, geo.bar_height),
            border: SideOffsets2D::zero(),
            rounding: Rounding::all(geo.scrollbar_w / 2.0),
            color: AlphaColor::new([1., 1., 1., 0.8]).into(),
            border_color: AlphaColor::TRANSPARENT.into(),
        }))
    }

    fn measure(
        &mut self,
        _: ElementId,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }

    fn debug_label(&self) -> &'static str {
        "Scroll Area"
    }

    fn focusable(&self) -> bool {
        true
    }
}

pub fn scroll_area(scroll_amount: Signal<f32>) -> ElementBuilder<ScrollArea> {
    let drag_offset = Cell::new(None::<f32>);

    ElementBuilder::new(ScrollArea { scroll_amount })
        .append_after_build(move |elem_id| {
            let current = scroll_amount.get_untracked();
            if current != 0.0 {
                TreeManager::global()
                    .queue_scroll_update(elem_id, Vector2D::new(0., current));
            }
        })
        .on_mouse(move |node, ctx| {
            let layout = node.get_abs_layout();
            let geo = ScrollGeometry::from_layout(layout);

            match ctx.payload().kind {
                MouseEventKind::Press {
                    button: MouseButton::Left,
                    ..
                } if !ctx.in_capture_phase() => {
                    ctx.stop_propagating();
                    ctx.request_mouse_capture(ctx.current_node());

                    let local_mouse_y =
                        ctx.payload().position.y - layout.location.y - layout.border.top;
                    let current_bar_y =
                        geo.track_height * geo.px_to_ratio(scroll_amount.get_untracked());

                    if local_mouse_y >= current_bar_y
                        && local_mouse_y <= (current_bar_y + geo.bar_height)
                    {
                        drag_offset.set(Some(local_mouse_y - current_bar_y));
                    } else {
                        // Jump scroll
                        let new_px = geo.scroll_to_px(
                            (local_mouse_y - (geo.bar_height / 2.0)) / geo.track_height,
                        );
                        scroll_amount.set(new_px);
                        drag_offset.set(Some(geo.bar_height / 2.0));
                    }
                }
                MouseEventKind::Motion { .. }
                    if ctx.current_phase() == EventPhase::Direct
                        && drag_offset.get().is_some() =>
                {
                    let local_mouse_y =
                        ctx.payload().position.y - layout.location.y - layout.border.top;
                    let new_bar_top = local_mouse_y - drag_offset.get().unwrap();
                    scroll_amount.set(geo.scroll_to_px(new_bar_top / geo.track_height));
                }
                MouseEventKind::Release {
                    button: MouseButton::Left,
                    ..
                } if ctx.current_phase() == EventPhase::Direct => {
                    ctx.request_mouse_release();
                    drag_offset.set(None);
                }
                MouseEventKind::Axis { vertical, .. } => {
                    scroll_amount.set(
                        (scroll_amount.get_untracked() - vertical.absolute as f32)
                            .clamp(0.0, geo.max_scroll),
                    );
                }
                _ => {}
            }

            TreeManager::global().queue_scroll_update(
                node.node_id(),
                Vector2D::new(0., scroll_amount.get_untracked()),
            );
            TreeManager::global().now();
        })
}

pub fn scroll_container(
    scroll_pos: Signal<f32>,
    children: impl Into<crate::prelude::BuilderList>,
    gap: f32,
) -> ElementBuilder<ScrollArea> {
    scroll_area(scroll_pos)
        .style(Style {
            display: taffy::Display::Flex,
            flex_direction: taffy::FlexDirection::Column,
            scrollbar_width: 6.,
            overflow: taffy::Point {
                y: taffy::Overflow::Scroll,
                ..Default::default()
            },
            gap: taffy::prelude::length(gap),
            size: taffy::prelude::percent(1.),
            ..Default::default()
        })
        .child(children)
}
