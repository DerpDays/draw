use crate::tools::{Tool, ToolMessage};
use color::{PremulColor, Srgb};
use input::{Key, Modifiers, MouseButton, MouseEvent, MouseEventKind};

use graphics::primitives::{Line, LineOptions};
use graphics::{CanvasCoordinates, Drawable, Primitive, Systems};

const SNAP_RADIANS: f32 = std::f32::consts::FRAC_PI_8 / 2.;

#[derive(Clone, Debug)]
pub struct LineTool {
    drag: Option<Primitive<CanvasCoordinates>>,
    pub color: PremulColor<Srgb>,
}
impl LineTool {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Default for LineTool {
    fn default() -> Self {
        Self {
            drag: None,
            color: PremulColor::new([1., 1., 1., 1.]),
        }
    }
}

impl Tool for LineTool {
    fn mouse_event(
        &mut self,
        systems: &mut Systems,
        event: MouseEvent,
        modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Leave => {
                if self.drag.is_some() {
                    self.drag = None;
                    vec![ToolMessage::ReleaseFocus, ToolMessage::ClearScratch]
                } else {
                    vec![]
                }
            }
            MouseEventKind::Motion { .. } => {
                let Some(drag) = self.drag.as_mut() else {
                    return vec![];
                };
                match drag {
                    Primitive::Line(elem) => {
                        if modifiers.intersects(Modifiers::SHIFT) {
                            elem.set_destination_snap(event.position, SNAP_RADIANS);
                        } else {
                            elem.set_destination(event.position);
                        }
                    }
                    _ => unreachable!("line tool's drag can only be a line"),
                }
                vec![ToolMessage::Scratch(drag.render(systems).clone())]
            }
            MouseEventKind::Press { button, .. } => match button {
                MouseButton::Left => {
                    let drag = self.drag.insert(Primitive::Line(Line::new(
                        event.position,
                        event.position,
                        LineOptions {
                            color: self.color,
                            ..Default::default()
                        },
                    )));
                    let mesh = drag.render(systems).clone();
                    vec![ToolMessage::SetFocus, ToolMessage::Scratch(mesh)]
                }
                MouseButton::Right => {
                    self.drag = None;
                    vec![ToolMessage::ClearScratch]
                }
                _ => vec![],
            },
            MouseEventKind::Release { button, .. } => match button {
                MouseButton::Left => {
                    let Some(mut drag) = self.drag.take() else {
                        return vec![];
                    };
                    let is_empty;
                    match &mut drag {
                        Primitive::Line(elem) => {
                            if modifiers.intersects(Modifiers::SHIFT) {
                                elem.set_destination_snap(event.position, SNAP_RADIANS);
                            } else {
                                elem.set_destination(event.position);
                            }
                            is_empty = elem.is_empty();
                        }
                        _ => unreachable!("line tool's drag can only be a line"),
                    }
                    if is_empty {
                        vec![ToolMessage::ReleaseFocus]
                    } else {
                        vec![ToolMessage::ReleaseFocus, ToolMessage::Commit(drag)]
                    }
                }
                _ => vec![],
            },
            _ => vec![],
        }
    }

    fn keyboard_event(
        &mut self,
        _systems: &mut Systems,
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
                }
                vec![]
            }
            _ => vec![],
        }
    }
}
