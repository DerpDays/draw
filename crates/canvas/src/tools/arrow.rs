use crate::tools::{Tool, ToolMessage};
use graphics::Systems;
use input::{Modifiers, MouseEvent};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct ArrowTool {}
impl ArrowTool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tool for ArrowTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        _event: MouseEvent,
        _active_modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        vec![]
    }
}
