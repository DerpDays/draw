use crate::tools::{Tool, ToolMessage};
use graphics::Systems;
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct ZoomTool {}
impl ZoomTool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tool for ZoomTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        event: MouseEvent,
        _modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Press { button, .. } => match button {
                MouseButton::Left => vec![ToolMessage::ZoomIn(event.position)],
                MouseButton::Right => vec![ToolMessage::ZoomOut(event.position)],
                MouseButton::Middle => vec![ToolMessage::ResetZoom],
                _ => vec![],
            },
            _ => vec![],
        }
    }
}
