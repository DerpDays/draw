use euclid::default::Size2D;
use graphics::{
    Systems,
    primitives::{Svg, SvgOptions},
};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::tools::{Tool, ToolMessage};

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
                    include_bytes!("../../../../resources/typst_test.svg").to_vec(),
                    SvgOptions {
                        quantize: true,
                        ..Default::default()
                    },
                )))]
            }
            _ => vec![],
        }
    }
}
