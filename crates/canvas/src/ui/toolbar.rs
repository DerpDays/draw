use color::PremulColor;
use euclid::default::Point2D;
use graphics::{primitives::RectangleOptions, Rounding};

use gui::prelude::*;

use gui::tree::{UITree, ZIndexProperties};
use gui::widgets::button::{ButtonOptions, FADE_DURATION};
use gui::widgets::{BackgroundWidget, ButtonWidget, ContainerWidget, SvgWidget};
use gui::Element;
use input::{CursorIcon, MouseButton, MouseEventKind};
use strum::IntoEnumIterator;

use crate::tools::{ToolKind, ToolNodeMap};
use crate::ui::styles::{colors, floating_grab};
use crate::ui::Message;

pub fn create_toolbar(
    tree: &mut UITree<gui::widgets::Widget<Message>>,
    selected_tool: ToolKind,
) -> (NodeId, ToolNodeMap) {
    let grab_container = ContainerWidget::new(true).mouse_handler(|_, ctx| {
        match ctx.current_phase() {
            EventPhase::Bubbling | EventPhase::AtTarget | EventPhase::Direct => {
                match ctx.payload().kind {
                    MouseEventKind::Enter => {
                        ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Grab)])
                    }
                    MouseEventKind::Leave => ctx.push_messages(vec![
                        Message::CursorIcon(CursorIcon::default()),
                        Message::EndGrab,
                    ]),
                    MouseEventKind::Motion { .. } => {
                        // TODO: shouldnt need a redraw for all mouse motion in the wrapper, only when
                        // held down
                        ctx.push_messages(vec![Message::HandleGrabMove(ctx.payload().position)])
                    }
                    MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                        ctx.push_messages(vec![
                            Message::MoveTop(ctx.current_node()),
                            Message::CursorIcon(CursorIcon::Grabbing),
                            Message::StartGrab(ctx.payload().position),
                        ]);
                        ctx.request_mouse_capture(ctx.current_node());
                    }
                    MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                        ctx.push_messages(vec![
                            Message::CursorIcon(CursorIcon::Grab),
                            Message::EndGrab,
                        ]);
                        ctx.request_mouse_release();
                    }
                    _ => {}
                }
            }
            EventPhase::Capturing => match ctx.payload().kind {
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    ctx.push_messages(vec![Message::MoveTop(ctx.current_node())])
                }
                _ => {}
            },
        };
    });

    let grab_container_node = tree.new_leaf_with_z(
        grab_container.as_widget(),
        floating_grab(100., Point2D::new(100., 100.)),
        ZIndexProperties {
            z_index: 0,
            isolate_z: true,
        },
    );

    let toolbar_style = Style {
        display: Display::Flex,
        padding: Rect::<LengthPercentage>::length(5.),
        gap: Size::<LengthPercentage>::length(5.),
        ..Default::default()
    };

    let toolbar = tree.new_leaf(
        BackgroundWidget::new(RectangleOptions {
            color: colors::BACKGROUND.into(),
            rounding: Rounding::all(5.),
            stroke_width: 3.,
            stroke_color: colors::BORDER.into(),
            box_sizing: graphics::BoxSizing::ContentBox,
        })
        .as_widget(),
        toolbar_style,
    );

    let tools = ToolKind::iter();
    let mut tool_nodes = ToolNodeMap::zero();

    for tool in tools {
        let tool_style = Style {
            display: Display::Flex,
            justify_content: Some(AlignContent::Center),
            align_items: Some(AlignItems::Stretch),
            size: Size::<Dimension>::from_lengths(48., 48.),
            ..Default::default()
        };
        let button_container = ButtonWidget::new(
            ButtonOptions {
                pressed: RectangleOptions {
                    color: PremulColor::new([0.32, 0.32, 0.32, 1.]).into(),
                    rounding: Rounding::all(5.),
                    ..Default::default()
                },
                active: RectangleOptions {
                    color: PremulColor::new([0.25, 0.25, 0.25, 1.]).into(),
                    rounding: Rounding::all(5.),
                    ..Default::default()
                },
                hovered: RectangleOptions {
                    color: PremulColor::new([0.2, 0.2, 0.2, 1.]).into(),
                    rounding: Rounding::all(5.),
                    ..Default::default()
                },
                normal: RectangleOptions {
                    color: colors::BACKGROUND.into(),
                    rounding: Rounding::all(5.),
                    ..Default::default()
                },
                disabled: RectangleOptions {
                    color: PremulColor::new([0.3, 0.14, 0.14, 0.5]).into(),
                    rounding: Rounding::all(5.),
                    ..Default::default()
                },
            },
            true,
            tool == selected_tool,
            FADE_DURATION,
        )
        .mouse_handler(move |_, ctx| {
            match ctx.payload().kind {
                MouseEventKind::Enter => {
                    ctx.push_messages(vec![Message::CursorIcon(CursorIcon::Pointer)])
                }
                MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                    ctx.push_messages(vec![Message::SwapTool(tool)])
                }
                _ => {}
            };
            match ctx.current_phase() {
                EventPhase::Direct | EventPhase::AtTarget | EventPhase::Bubbling => {
                    ctx.stop_propagation();
                }
                _ => {}
            }
        });

        let button_node = tree.new_leaf(button_container.as_widget(), tool_style);

        tool_nodes.set(tool, button_node);

        let icon_style = Style {
            flex_grow: 1.,
            margin: Rect::length(8.),
            ..Default::default()
        };
        let svg_node = tree.new_leaf(
            SvgWidget::new(
                tool.svg_icon().into(),
                gui::widgets::svg::SvgOptions {
                    normal: graphics::primitives::SvgOptions {
                        fill_color: Some(colors::WHITE),
                        stroke_color: Some(colors::WHITE),
                        ..Default::default()
                    },
                    hover: None,
                },
            )
            .as_widget(),
            icon_style,
        );

        tree.add_child(button_node, svg_node);
        tree.add_child(toolbar, button_node);
    }

    tree.add_child(grab_container_node, toolbar);
    (grab_container_node, tool_nodes)
}
