use crate::tools::{Tool, ToolMessage};
use graphics::Systems;
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct EraserTool;

impl EraserTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for EraserTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        event: MouseEvent,
        _modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                vec![ToolMessage::Erase(event.position)]
            }
            _ => vec![],
        }
    }
}
