use color::AlphaColor;
use euclid::default::Point2D;
use gui_reactive::prelude::{Size, Style};
use gui_reactive::widgets::primitives::div;
use gui_reactive::{
    reexports::reactive_graph::{
        signal::{arc_signal, ArcReadSignal, ArcWriteSignal},
        traits::{GetUntracked, Set},
    },
    tree::ErasedBuilder,
    widgets::primitives::DivOptions,
};
use input::{MouseButton, MouseEventKind};

use crate::ui2::{floating_grab, DragState};

pub fn options() -> impl ErasedBuilder {
    let (toolbar_style, set_toolbar_style) =
        arc_signal(floating_grab(100., Point2D::new(500., 500.)));
    let (drag, set_drag): (
        ArcReadSignal<Option<DragState>>,
        ArcWriteSignal<Option<DragState>>,
    ) = arc_signal(None);

    // root elemgeneric over
    div()
        .style(toolbar_style)
        .on_mouse(move |elem, ctx| match ctx.payload().kind {
            MouseEventKind::Enter | MouseEventKind::Leave => {
                set_drag.set(None);
            }
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                let layout = elem.get_final_layout();
                set_drag.set(Some(DragState {
                    origin: Point2D::new(layout.location.x, layout.location.y),
                    start: ctx.payload().position,
                }));
                ctx.request_mouse_capture(ctx.current_node());
            }
            MouseEventKind::Release { .. } => {
                set_drag.set(None);
                ctx.request_mouse_release();
            }
            MouseEventKind::Motion { .. } => {
                if let Some(drag) = drag.get_untracked() {
                    let new_origin = (drag.origin + (ctx.payload().position - drag.start)).round();
                    tracing::info!("updating position!!! {new_origin:#?}");
                    set_toolbar_style.set(floating_grab(100., new_origin));
                } else {
                }
            }
            _ => {}
        })
        .child(
            div()
                .options(DivOptions {
                    bg_color: Some(AlphaColor::BLACK.into()),
                    ..Default::default()
                })
                .style(Style {
                    size: Size::length(100.),
                    ..Style::DEFAULT
                }),
        )
}
