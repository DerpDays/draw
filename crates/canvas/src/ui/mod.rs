use euclid::default::Point2D;

use gui::{prelude::*, tree::ZIndexProperties, widgets::Widget, UITree};
use input::{CursorIcon, Modifiers, MouseButton, MouseEventKind};
use renderer::GrowableMeshBuffer;

use crate::{
    tools::ToolKind,
    ui::{
        options::{OptionsMessage, OptionsTree},
        toolbar::Toolbar,
    },
    RedrawRequest,
};

pub mod options;
pub mod styles;
pub mod toolbar;

#[derive(Copy, Clone, Debug)]
pub enum Message {
    CursorIcon(input::CursorIcon),
    MoveTop(NodeId),

    StartGrab(Point2D<f32>),
    HandleGrabMove(Point2D<f32>),
    EndGrab,

    SwapTool(ToolKind),
    ToolOptions(OptionsMessage),
}

pub struct Application {
    pub gui: UITree<Widget<Message>>,
    pub gui_buffer: GrowableMeshBuffer,

    pub drag_start: Option<DragState>,

    pub selected_tool: ToolKind,

    pub toolbar: Toolbar,
    pub options: OptionsTree,

    pub modifiers: Modifiers,
}

#[derive(Copy, Clone, Debug)]
pub struct DragState {
    /// Where to base the movement from.
    origin: Point2D<f32>,
    /// Where to calculate the change in position from.
    start: Point2D<f32>,
}

pub fn handle_message<T: RedrawRequest + Clone + 'static>(
    app: &mut Application,
    node: NodeId,
    message: &Message,
    redraw_manager: &T,
) -> Option<CursorIcon> {
    let mut cursor_icon = None;
    match message {
        Message::CursorIcon(icon) => {
            cursor_icon = Some(*icon);
        }
        Message::SwapTool(tool_kind) => {
            tracing::info!(
                "swapping tool from {:?} to {:?}",
                app.selected_tool,
                tool_kind
            );
            app.selected_tool = *tool_kind;
            app.toolbar.swap_tool(&mut app.gui, *tool_kind);
        }
        Message::StartGrab(point) => {
            let style = app.gui.relative_layout(node);
            app.drag_start = Some(DragState {
                origin: Point2D::new(style.location.x, style.location.y),
                start: *point,
            });
        }
        Message::HandleGrabMove(point) => {
            if let Some(state) = app.drag_start {
                cursor_icon = Some(CursorIcon::Grabbing);
                let new_origin = (state.origin + (*point - state.start)).round();
                // TODO: constant
                let grab_style = styles::floating_grab(100., new_origin);
                tracing::info!("Moving grab area to: {new_origin:?}");
                app.gui.set_style(node, grab_style);
                // TODO: redo this
                redraw_manager.request_redraw();
            } else {
                cursor_icon = Some(CursorIcon::Grab);
            }
        }
        Message::EndGrab => {
            app.drag_start = None;
        }
        Message::MoveTop(node) => {
            if let Some(parent) = app.gui.parent(*node) {
                let target_z = app.gui.get_zindex_properties(*node);

                let children = app.gui.children(parent);
                for child in children.iter() {
                    if *child == *node {
                        app.gui.set_zindex_properties(
                            *node,
                            ZIndexProperties {
                                z_index: children.len() - 1,
                                ..target_z
                            },
                        );
                        continue;
                    }

                    let child_z = app.gui.get_zindex_properties(*child);
                    if target_z.z_index < child_z.z_index {
                        app.gui.set_zindex_properties(
                            *child,
                            ZIndexProperties {
                                z_index: child_z.z_index.saturating_sub(1),
                                ..child_z
                            },
                        );
                    }
                }
            }
            redraw_manager.request_redraw();
        }
        Message::ToolOptions(message) => {
            app.options.update(&mut app.gui, message);
        }
    };
    cursor_icon
}

pub fn grab_fn<T>(_: &mut T, ctx: &mut EventContext<MouseEvent, Message>) {
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
                    ctx.push_messages(vec![Message::HandleGrabMove(ctx.payload().position)])
                }
                MouseEventKind::Press {
                    button: MouseButton::Left,
                    ..
                } => {
                    ctx.push_messages(vec![
                        Message::MoveTop(ctx.current_node()),
                        Message::CursorIcon(CursorIcon::Grabbing),
                        Message::StartGrab(ctx.payload().position),
                    ]);
                    ctx.request_mouse_capture(ctx.current_node());
                }
                MouseEventKind::Release {
                    button: MouseButton::Left,
                    ..
                } => {
                    ctx.push_messages(vec![
                        Message::CursorIcon(CursorIcon::Grab),
                        Message::EndGrab,
                    ]);
                    ctx.request_mouse_release();
                }
                _ => {}
            }
        }
        EventPhase::Capturing => {
            if let MouseEventKind::Press {
                button: MouseButton::Left,
                ..
            } = ctx.payload().kind
            {
                ctx.push_messages(vec![Message::MoveTop(ctx.current_node())])
            }
        }
    };
}
