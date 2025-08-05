use crate::tools::{Tool, ToolMessage};
use color::{PremulColor, Srgb};
use graphics::primitives::{Pen, PenOptions};
use graphics::{CanvasCoordinates, Drawable, Primitive, Systems};
use input::{Key, Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug)]
pub struct PenTool {
    drag: Option<Primitive<CanvasCoordinates>>,
    pub color: PremulColor<Srgb>,
}
impl PenTool {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for PenTool {
    fn default() -> Self {
        Self {
            drag: Default::default(),
            color: PremulColor::new([1., 1., 1., 1.]),
        }
    }
}

impl Tool for PenTool {
    fn mouse_event(
        &mut self,
        systems: &mut Systems,
        event: MouseEvent,
        _modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Enter => {
                self.drag = None;
                vec![]
            }
            MouseEventKind::Leave => {
                if self.drag.is_some() {
                    self.drag = None;
                    vec![ToolMessage::ReleaseFocus, ToolMessage::ClearScratch]
                } else {
                    vec![]
                }
            }
            MouseEventKind::Motion { .. } => {
                if let Some(mut drag) = self.drag.take() {
                    match &mut drag {
                        Primitive::Pen(elem) => elem.handle_drag(event.position),
                        _ => unreachable!(),
                    }
                    self.drag = Some(drag);
                    if let Some(drag) = &mut self.drag {
                        vec![ToolMessage::Scratch(drag.render(systems).clone())]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            MouseEventKind::Press { button, .. } => {
                if button == MouseButton::Left {
                    self.drag = Some(Primitive::Pen(Pen::new(
                        event.position,
                        PenOptions {
                            color: self.color,
                            ..Default::default()
                        },
                    )));
                    if let Some(drag) = &mut self.drag {
                        vec![
                            ToolMessage::SetFocus,
                            ToolMessage::Scratch(drag.render(systems).clone()),
                        ]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            MouseEventKind::Release { button, .. } => {
                if button == MouseButton::Left {
                    if let Some(mut drag) = self.drag.take() {
                        match &mut drag {
                            Primitive::Pen(elem) => elem.handle_drag(event.position),
                            _ => unreachable!(),
                        }
                        vec![ToolMessage::ReleaseFocus, ToolMessage::Commit(drag)]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            MouseEventKind::Axis { .. } => vec![],
        }
    }
    fn keyboard_event(
        &mut self,
        systems: &mut Systems,
        event: input::KeyboardEvent,
    ) -> Vec<ToolMessage> {
        match event.kind {
            input::KeyboardEventKind::Press(key) => {
                if key == Key::SpecialKey(input::SpecialKey::Tab) {
                    if self.color == PremulColor::WHITE {
                        self.color = PremulColor::BLACK
                    } else {
                        self.color = PremulColor::WHITE
                    };
                    if let Some(drag) = &mut self.drag {
                        match drag {
                            Primitive::Pen(pen) => pen.update_options(PenOptions {
                                color: self.color,
                                ..*pen.options()
                            }),
                            _ => unreachable!(),
                        };
                        vec![ToolMessage::Scratch(drag.render(systems).clone())]
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
