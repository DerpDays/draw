use euclid::default::Point2D;
use graphics::Systems;

use crate::tools::{Tool, ToolMessage};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct SelectTool {
    pub initial_point: Option<Point2D<f32>>,
}
impl SelectTool {
    pub fn new() -> Self {
        Self {
            initial_point: None,
        }
    }
}

impl Tool for SelectTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        event: MouseEvent,
        _active_modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                self.initial_point = Some(event.position);
                vec![ToolMessage::SetFocus]
            }
            MouseEventKind::Release { button, .. } if button == MouseButton::Left => {
                if let Some(initial_point) = self.initial_point.take() {
                    vec![
                        ToolMessage::ReleaseFocus,
                        ToolMessage::Select(initial_point),
                    ]
                } else {
                    vec![ToolMessage::ReleaseFocus]
                }
            }
            MouseEventKind::Enter | MouseEventKind::Leave => {
                self.initial_point = None;
                vec![]
            }
            _ => vec![],
        }
    }
}
