use color::{PremulColor, Srgb};
use euclid::default::{Box2D, Size2D};
use graphics::primitives::{Text, TextOptions};
use graphics::{CanvasCoordinates, Drawable, Primitive, Systems};

use crate::tools::{Tool, ToolMessage};
use input::{
    Key, KeyboardEvent, KeyboardEventKind, Modifiers, MouseButton, MouseEvent, MouseEventKind,
    SpecialKey,
};

#[derive(Clone, Debug)]
pub struct TextTool {
    current: Option<Primitive<CanvasCoordinates>>,
    pub color: PremulColor<Srgb>,
}

impl TextTool {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for TextTool {
    fn default() -> Self {
        Self {
            current: None,
            color: PremulColor::new([1., 1., 1., 1.]),
        }
    }
}

impl Tool for TextTool {
    fn mouse_event(
        &mut self,
        systems: &mut Systems,
        event: MouseEvent,
        _modifiers: Modifiers,
    ) -> Vec<ToolMessage> {
        match event.kind {
            MouseEventKind::Enter | MouseEventKind::Leave => {
                // self.editor = None;
                self.current = None;
                vec![ToolMessage::ReleaseFocus, ToolMessage::ClearScratch]
            }
            MouseEventKind::Press { button, .. } if button == MouseButton::Left => {
                tracing::info!("adding a text element!!! with color: {:?}", self.color);
                let current = self.current.get_or_insert(Primitive::Text(Text::new(
                    String::new(),
                    TextOptions {
                        color: self.color,
                        ..Default::default()
                    },
                    Box2D::from_origin_and_size(event.position, Size2D::new(f32::MAX, f32::MAX)),
                )));

                vec![
                    ToolMessage::SetFocus,
                    ToolMessage::Scratch(current.render(systems).clone()),
                ]
            }
            _ => vec![],
        }
    }
    fn keyboard_event(&mut self, systems: &mut Systems, event: KeyboardEvent) -> Vec<ToolMessage> {
        tracing::info!("received kb event: {event:?}");
        match event.kind {
            KeyboardEventKind::Press(key) => {
                if key == Key::SpecialKey(SpecialKey::Enter) {
                    tracing::info!("finalising text tool");
                    if let Some(current) = self.current.take() {
                        return vec![ToolMessage::ReleaseFocus, ToolMessage::Commit(current)];
                    } else {
                        return vec![ToolMessage::ReleaseFocus];
                    }
                };

                if key == Key::SpecialKey(input::SpecialKey::Tab) {
                    if self.color == PremulColor::WHITE {
                        self.color = PremulColor::BLACK
                    } else {
                        self.color = PremulColor::WHITE
                    };
                }

                let Some(current) = self.current.as_mut() else {
                    return vec![];
                };
                match current {
                    Primitive::Text(text) => {
                        let mut content = text.content();
                        match key {
                            Key::SpecialKey(special_key) => match special_key {
                                SpecialKey::Delete | SpecialKey::Backspace => {
                                    content.pop();
                                }
                                SpecialKey::Left => {
                                    content =
                                        "test\nsomething multiline\n with more lines".to_string();
                                }
                                SpecialKey::Right => todo!(),
                                SpecialKey::Up => todo!(),
                                SpecialKey::Down => todo!(),
                                _ => {}
                            },
                            Key::Character(str) => content.push_str(str.as_str()),
                            Key::Unknown => {}
                        }
                        tracing::info!("content is now: {content:?}");
                        text.set_content(content);
                    }
                    _ => unreachable!("text tool should only contain a text primitive"),
                }
                vec![ToolMessage::Scratch(current.render(systems).clone())]
            }
            KeyboardEventKind::Release(_key) => vec![],
            KeyboardEventKind::ModifiersChanged => vec![],
        }
    }
}
