use crate::tools::{Tool, ToolMessage};
use euclid::default::Size2D;
use graphics::{
    primitives::{Svg, SvgOptions},
    Systems,
};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Copy, Default, PartialEq)]
pub struct HighlighterTool {}
impl HighlighterTool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tool for HighlighterTool {
    fn mouse_event(
        &mut self,
        _systems: &mut Systems,
        event: MouseEvent,
        _active_modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                vec![ToolMessage::Commit(graphics::Primitive::Svg(Svg::new(
                    event.position.round(),
                    Size2D::new(105.2898 * 2., 74.635 * 2.),
                    include_bytes!("../../../../test.svg").to_vec(),
                    SvgOptions::default(),
                )))]
            }
            _ => vec![],
        }
    }
}
