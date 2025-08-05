use euclid::default::Point2D;
use graphics::Systems;

use crate::tools::{Tool, ToolKind, ToolMessage};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct GrabTool {
    pub grab: Option<Point2D<f32>>,
}
impl GrabTool {
    pub fn new() -> Self {
        Self { grab: None }
    }
}

impl Tool for GrabTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        event: MouseEvent,
        _modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                self.grab = Some(event.position);
                vec![
                    ToolMessage::SetFocus,
                    ToolMessage::CursorIcon(input::CursorIcon::Grabbing),
                ]
            }
            MouseEventKind::Release { .. } => {
                self.grab = None;
                vec![
                    ToolMessage::ReleaseFocus,
                    ToolMessage::CursorIcon(ToolKind::Grab.default_cursor()),
                ]
            }
            MouseEventKind::Motion { .. } => {
                if let Some(origin) = self.grab {
                    vec![ToolMessage::GrabMove(origin, event.position)]
                } else {
                    vec![ToolMessage::ReleaseFocus]
                }
            }
            MouseEventKind::Enter | MouseEventKind::Leave => {
                self.grab = None;
                vec![]
            }
            _ => vec![],
        }
    }
}
