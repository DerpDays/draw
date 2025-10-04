use color::AlphaColor;
use euclid::default::Point2D;
use graphics::primitives::SvgOptions;
use graphics::Rounding;
use gui_reactive::prelude::{Size, Style, *};
use gui_reactive::reexports::reactive_graph::signal::{signal, ReadSignal, WriteSignal};
use gui_reactive::reexports::reactive_graph::traits::Get;
use gui_reactive::reexports::reactive_graph::wrappers::read::Signal;
use gui_reactive::widgets::primitives::{div, svg};
use gui_reactive::{
    reexports::reactive_graph::{
        signal::{arc_signal, ArcReadSignal, ArcWriteSignal},
        traits::{GetUntracked, Set},
    },
    tree::ErasedBuilder,
    widgets::primitives::DivOptions,
};
use input::{MouseButton, MouseEventKind};

use crate::tools::ToolKind;
use crate::ui2::styles::colors;
use crate::ui2::{floating_grab, DragState};

pub fn toolbar() -> impl ErasedBuilder {
    let (toolbar_style, set_toolbar_style) =
        arc_signal(floating_grab(100., Point2D::new(100., 100.)));
    let (drag, set_drag): (
        ArcReadSignal<Option<DragState>>,
        ArcWriteSignal<Option<DragState>>,
    ) = arc_signal(None);

    let (active_tool, set_active_tool) = signal(ToolKind::default());

    // root element used for visibility toggling
    div().child(
        // drag container
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
                        let new_origin =
                            (drag.origin + (ctx.payload().position - drag.start)).round();
                        tracing::info!("updating position!!! {new_origin:#?}");
                        set_toolbar_style.set(floating_grab(100., new_origin));
                    } else {
                    }
                }
                _ => {}
            })
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
                            .map(|tool| tool_button(tool, active_tool, set_active_tool))
                            .collect::<Vec<_>>(),
                    ),
            ),
    )
}

pub enum ButtonStatus {
    Pressed,
    Active,
    Hovered,
    Normal,
    Disabled,
}

fn options_from_status() {}

fn tool_button(
    tool: ToolKind,
    active_tool: ReadSignal<ToolKind>,
    set_active_tool: WriteSignal<ToolKind>,
) -> impl ErasedBuilder {
    // let current_status = move || {
    //     if active_tool.get() == tool {
    //         ButtonStatus::Active
    //     } else {
    //         ButtonStatus::Normal
    //     }
    // };
    div()
        .style(Style {
            display: Display::Flex,
            justify_content: Some(AlignContent::Center),
            align_items: Some(AlignItems::Stretch),
            size: Size::<Dimension>::from_lengths(48., 48.),
            ..Default::default()
        })
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
        )
}
