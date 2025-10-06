use std::time::Duration;

use color::AlphaColor;
use euclid::default::Point2D;
use graphics::{primitives::SvgOptions, Rounding};
use gui_reactive::{
    prelude::{Size, Style, *},
    reexports::reactive::{create_memo, create_signal, Signal},
    tree::builder::ErasedBuilder,
    widgets::{
        primitives::{div, svg, DivOptions},
        reactivity::{button_with, ButtonVisualState},
    },
};
use input::{MouseButton, MouseEventKind};

use crate::{
    tools::ToolKind,
    ui2::{drag_fn, floating_grab, styles::colors, DragState},
};

pub fn toolbar(selected_tool: Signal<ToolKind>) -> impl ErasedBuilder {
    let drag: Signal<Option<DragState>> = create_signal(None);
    let drag_style = create_signal(floating_grab(100., Point2D::new(100., 100.)));

    // root element used for visibility toggling
    div().child(
        // drag container
        div()
            .style(drag_style)
            .on_mouse(drag_fn(drag, drag_style))
            .child(
                // inner background
                div()
                    .options(DivOptions {
                        bg_color: Some(colors::BACKGROUND.into()),
                        stroke_color: Some(colors::BORDER.into()),
                        stroke_width: Some(3.),
                        rounding: Some(Rounding::all(5.)),
                    })
                    .style(Style {
                        display: Display::Flex,
                        padding: Rect::<LengthPercentage>::length(5.),
                        gap: Size::<LengthPercentage>::length(5.),
                        ..Default::default()
                    })
                    .child(
                        <ToolKind as strum::IntoEnumIterator>::iter()
                            .map(|tool| tool_button(tool, selected_tool))
                            .collect::<Vec<_>>(),
                    ),
            ),
    )
}

fn tool_button(tool: ToolKind, active_tool: Signal<ToolKind>) -> impl ErasedBuilder {
    let (btn, signals) = button_with(
        create_memo(|| true),
        create_memo(move || active_tool.get() == tool),
    );
    let visual_state = signals.to_visual();
    let btn_style = move || match visual_state.get() {
        ButtonVisualState::Pressed => DivOptions {
            bg_color: Some(AlphaColor::new([0.32, 0.32, 0.32, 1.]).into()),
            rounding: Some(Rounding::all(5.)),
            ..Default::default()
        },
        ButtonVisualState::Active => DivOptions {
            bg_color: Some(AlphaColor::new([0.25, 0.25, 0.25, 1.]).into()),
            rounding: Some(Rounding::all(5.)),
            ..Default::default()
        },
        ButtonVisualState::Hovered => DivOptions {
            bg_color: Some(AlphaColor::new([0.2, 0.2, 0.2, 1.]).into()),
            rounding: Some(Rounding::all(5.)),
            ..Default::default()
        },
        ButtonVisualState::Normal => DivOptions {
            bg_color: Some(colors::BACKGROUND.into()),
            rounding: Some(Rounding::all(5.)),
            ..Default::default()
        },
        ButtonVisualState::Disabled => DivOptions {
            bg_color: Some(AlphaColor::new([0.3, 0.14, 0.14, 0.5]).into()),
            rounding: Some(Rounding::all(5.)),
            ..Default::default()
        },
    };

    btn.style(Style {
        display: Display::Flex,
        justify_content: Some(AlignContent::Center),
        align_items: Some(AlignItems::Stretch),
        size: Size::<Dimension>::from_lengths(48., 48.),
        ..Default::default()
    })
    .on_mouse(move |_, ctx| {
        if !ctx.in_capture_phase() {
            match ctx.payload().kind {
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    active_tool.set(tool);
                }
                _ => {}
            }
            match ctx.current_phase() {
                EventPhase::Direct | EventPhase::AtTarget | EventPhase::Bubbling => {
                    ctx.stop_propagating();
                }
                _ => {}
            };
        }
    })
    .child(
        div()
            .style(Style {
                size: Size::<Dimension>::percent(1.),
                ..Default::default()
            })
            .options(btn_style)
            .transition_duration(Duration::from_secs(10))
            .child(
                svg(tool.svg_icon().to_vec())
                    .style(Style {
                        flex_grow: 1.,
                        margin: Rect::length(8.),
                        ..Default::default()
                    })
                    .options(SvgOptions {
                        fill_color: Some(AlphaColor::WHITE),
                        stroke_color: Some(AlphaColor::WHITE),
                        ..SvgOptions::default()
                    }),
            ),
    )
}
