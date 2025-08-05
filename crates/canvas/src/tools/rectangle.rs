use color::PremulColor;
use euclid::Size2D;
use graphics::{
    primitives::{Rectangle, RectangleOptions},
    Drawable, Systems,
};

use crate::tools::{Tool, ToolMessage};
use graphics::{CanvasCoordinates, Primitive};
use input::{Modifiers, MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Debug, Default)]
pub struct RectangleTool {
    drag: Option<Primitive<CanvasCoordinates>>,
}
impl RectangleTool {
    pub fn new() -> Self {
        Self { drag: None }
    }
}

impl Tool for RectangleTool {
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
                    Primitive::Rectangle(elem) => {
                        if modifiers.intersects(Modifiers::SHIFT) {
                            elem.resize_to_point_square(event.position);
                        } else {
                            elem.resize_to_point(event.position);
                        }
                    }
                    _ => unreachable!("rectangle tool's drag can only be a rectangle"),
                }
                vec![ToolMessage::Scratch(drag.render(systems).clone())]
            }
            MouseEventKind::Press { button, .. } => match button {
                MouseButton::Left => {
                    let drag = self.drag.insert(Primitive::Rectangle(Rectangle::new(
                        event.position,
                        Size2D::zero(),
                        RectangleOptions {
                            stroke_color: PremulColor::WHITE.into(),
                            stroke_width: 2.,
                            box_sizing: graphics::BoxSizing::BorderBox,
                            ..RectangleOptions::DEFAULT
                        },
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
                    let size;
                    match &mut drag {
                        Primitive::Rectangle(elem) => {
                            if modifiers.intersects(Modifiers::SHIFT) {
                                elem.resize_to_point_square(event.position);
                            } else {
                                elem.resize_to_point(event.position);
                            }
                            size = elem.size().abs();
                        }
                        _ => unreachable!("rectangle tool's drag can only be a rectangle"),
                    }
                    if size.is_empty() {
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
}
