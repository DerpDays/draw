use euclid::default::Point2D;

use gui::{prelude::*, tree::ZIndexProperties, widgets::Widget, UITree};
use input::{CursorIcon, Modifiers};
use renderer::GrowableMeshBuffer;

use crate::{
    tools::{ToolKind, ToolNodeMap},
    ui::options::{OptionsMessage, OptionsTree},
    RedrawRequest,
};

pub mod options;
pub mod styles;
pub mod toolbar;

#[derive(Clone, Copy, Debug)]
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

    pub tool_nodes: ToolNodeMap,
    pub options: OptionsTree,

    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug)]
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

            app.gui
                .get_node_mut(app.selected_tool.get_node_id(&app.tool_nodes))
                .as_button_mut()
                .map(|x| x.set_active(false));
            app.selected_tool = *tool_kind;
            app.gui
                .get_node_mut(app.selected_tool.get_node_id(&app.tool_nodes))
                .as_button_mut()
                .map(|x| x.set_active(true));

            tracing::info!("called swap tool to : {tool_kind:?}");
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
