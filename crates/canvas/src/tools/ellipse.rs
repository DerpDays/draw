use euclid::default::Vector2D;
use graphics::{
    primitives::{Ellipse, EllipseOptions},
    Drawable, Systems,
};

use crate::tools::{Tool, ToolMessage};
use graphics::{CanvasCoordinates, Primitive};
use input::{KeyboardEvent, KeyboardEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Default)]
pub struct EllipseTool {
    drag: Option<Primitive<CanvasCoordinates>>,
}
impl EllipseTool {
    pub fn new() -> Self {
        Self { drag: None }
    }
}

impl Tool for EllipseTool {
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
                    Primitive::Ellipse(elem) => {
                        if modifiers.intersects(Modifiers::SHIFT) {
                            elem.resize_to_point_square(event.position);
                        } else {
                            elem.resize_to_point(event.position);
                        }
                    }
                    _ => unreachable!("ellipse tool's drag can only be a ellipse"),
                }
                vec![ToolMessage::Scratch(drag.render(systems).clone())]
            }
            MouseEventKind::Press { button, .. } => match button {
                MouseButton::Left => {
                    let drag = self.drag.insert(Primitive::Ellipse(Ellipse::new(
                        event.position,
                        Vector2D::zero(),
                        EllipseOptions::default(),
                    )));
                    vec![
                        ToolMessage::SetFocus,
                        ToolMessage::Scratch(drag.render(systems).clone()),
                    ]
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
                    let radius;
                    match &mut drag {
                        Primitive::Ellipse(elem) => {
                            if modifiers.intersects(Modifiers::SHIFT) {
                                elem.resize_to_point_square(event.position);
                            } else {
                                elem.resize_to_point(event.position);
                            }
                            radius = elem.radius().abs();
                        }
                        _ => unreachable!("ellipse tool's drag can only be a ellipse"),
                    }
                    if radius.x == 0. || radius.y == 0. {
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
    fn keyboard_event(&mut self, systems: &mut Systems, event: KeyboardEvent) -> Vec<ToolMessage> {
        match event.kind {
            KeyboardEventKind::ModifiersChanged => {
                let Some(drag) = self.drag.as_mut() else {
                    return vec![];
                };
                match drag {
                    Primitive::Ellipse(elem) => {
                        if event.modifiers.intersects(Modifiers::SHIFT) {
                            elem.resize_square();
                            vec![ToolMessage::Scratch(drag.render(systems).clone())]
                        } else {
                            vec![]
                        }
                    }
                    _ => unreachable!("ellipse tool's drag can only be a ellipse"),
                }
            }
            _ => vec![],
        }
    }
}
