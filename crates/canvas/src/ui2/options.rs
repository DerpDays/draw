use color::AlphaColor;
use euclid::default::Point2D;
use gui_reactive::{
    prelude::{Size, Style},
    reexports::reactive::{create_signal, Signal},
    tree::builder::ErasedBuilder,
    widgets::primitives::{div, DivOptions},
};

use crate::ui2::{drag_fn, floating_grab, DragState};

pub fn options() -> impl ErasedBuilder {
    let drag: Signal<Option<DragState>> = create_signal(None);
    let drag_style = create_signal(floating_grab(100., Point2D::new(500., 500.)));

    // root elemgeneric over
    div()
        .style(drag_style)
        .on_mouse(drag_fn(drag, drag_style))
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
