use euclid::default::{Box2D, Point2D, Size2D};
use graphics::{
    Drawable,
    Mesh,
    Systems,
    Vertex,
    ViewportCoordinates,
    primitives::{self, TextOptions},
};
use input::{
    Key,
    KeyboardEvent,
    KeyboardEventKind,
    Modifiers,
    MouseButton,
    MouseEvent,
    MouseEventKind,
    SpecialKey,
};

use crate::{
    events::{BlurEvent, ChangeEvent, FocusEvent},
    macros::event_handlers::impl_event_handler,
    prelude::{Element, EventContext, EventHandler},
    widgets::{Widget, parse_layout_change},
};

use super::LayoutChange;

struct TextInputWidgetOptions {
    allow_newline: bool,
    allow_wrapping: bool,
    max_len: bool,
    prefix: Option<String>,
}

#[derive(Clone)]
pub struct TextInputWidget<M: Clone> {
    inner: primitives::Text<ViewportCoordinates>,

    placeholder: String,
    value: String,
    cursor: Option<u32>,

    layout: taffy::Layout,

    pub mouse_handler: EventHandler<MouseEvent, Self>,
    pub keyboard_handler: EventHandler<KeyboardEvent, Self>,
    pub change_handler: EventHandler<ChangeEvent<String>, Self>,
    pub focus_handler: EventHandler<FocusEvent, Self>,
    pub blur_handler: EventHandler<BlurEvent, Self>,
}

impl_event_handler! {
    TextInputWidget,
    MouseEvent => mouse_handler,
    KeyboardEvent => keyboard_handler,
    ChangeEvent<String> => change_handler,
    FocusEvent => focus_handler,
    BlurEvent => blur_handler,
}

impl<M: Clone> TextInputWidget<M> {
    pub fn new(placeholder: String, value: Option<String>, options: TextOptions) -> Self {
        let inner = graphics::primitives::Text::new(
            value.clone().unwrap_or(placeholder.clone()),
            options,
            Box2D::zero(),
        );
        Self {
            inner,
            value: value.unwrap_or_default(),
            placeholder,
            cursor: None,

            layout: taffy::Layout::new(),

            mouse_handler: EventHandler::none(),
            keyboard_handler: EventHandler::none(),
            change_handler: EventHandler::none(),
            focus_handler: EventHandler::none(),
            blur_handler: EventHandler::none(),
        }
    }
    pub fn update_content<E>(
        &mut self,
        content: impl Into<String>,
        ctx: &mut EventContext<E, M>,
        relayout: bool,
    ) {
        let new = content.into();
        ctx.push_event(crate::events::TreeEvent::ChangeEventString(ChangeEvent {
            new: new.clone(),
            old: self.content().clone(),
        }));
        self.inner.set_content(new);
        ctx.request_redraw(crate::events::Redraw::Now);
        if relayout {
            ctx.request_relayout(ctx.current_node());
        }
    }

    pub fn update_content_silent_no_redraw(&mut self, content: impl Into<String>) {
        self.inner.set_content(content.into());
    }
    pub fn content(&self) -> &String {
        self.inner.content()
    }
}

impl<M: Clone> Element for TextInputWidget<M> {
    type Message = M;

    fn as_widget(self) -> Widget<Self::Message> {
        Widget::TextInput(self)
    }

    fn render(&mut self, systems: &mut Systems, layout: taffy::Layout) -> &Mesh<Vertex> {
        if self.layout != layout {
            match parse_layout_change(layout, self.layout) {
                LayoutChange::Translate(dx) => {
                    self.inner.translate(dx);
                }
                LayoutChange::Rerender => {
                    self.inner.update_rect(Box2D::from_origin_and_size(
                        Point2D::new(layout.location.x, layout.location.y),
                        Size2D::new(layout.size.width, layout.size.height),
                    ));
                }
            }
            self.layout = layout;
        }
        self.inner.render(systems)
    }
    fn measure(
        &mut self,
        systems: &mut Systems,
        _: taffy::Size<taffy::AvailableSpace>,
        _: &taffy::Style,
    ) -> taffy::Size<f32> {
        let size = self.inner.measure(systems);
        taffy::Size {
            width: size.width,
            height: size.height,
        }
    }

    fn mouse_event(&mut self, ctx: &mut EventContext<MouseEvent, Self::Message>) {
        if !ctx.in_capture_phase()
            && matches!(ctx.payload().kind, MouseEventKind::Press { button, .. } if button == MouseButton::Left)
        {
            ctx.request_kb_focus_capture(ctx.current_node());
        }
        self.mouse_handler.clone().handle(self, ctx);
    }

    fn keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent, Self::Message>) {
        tracing::info!("input widget got keyboard event {:?}", ctx.payload());
        self.keyboard_handler.clone().handle(self, ctx);
        if !ctx.is_preventing_default() {
            match &ctx.payload().kind {
                KeyboardEventKind::Press(key) => match key {
                    Key::SpecialKey(special_key) => {
                        match special_key {
                            SpecialKey::Escape => {
                                ctx.request_kb_focus_release();
                            }
                            // Remove character infront cursor
                            SpecialKey::Delete => {
                                self.value.pop();
                            }
                            // Remove character behind cursor
                            SpecialKey::Backspace => {
                                self.value.pop();
                            }
                            SpecialKey::Enter => self.value.push('\n'),
                            SpecialKey::Home => self.cursor = Some(0),
                            // move cursor end of entire string.
                            SpecialKey::End => todo!(),
                            // move cursor left.
                            SpecialKey::Left => todo!(),
                            // move cursor right.
                            SpecialKey::Right => todo!(),
                            // move to line above.
                            SpecialKey::Up => todo!(),
                            // move to line below.
                            SpecialKey::Down => self.cursor = Some(0),
                            // insert 4 spaces.
                            SpecialKey::Tab => self.value.push_str("    "),
                            _ => {}
                        }
                    }
                    Key::Character(char) => match char.as_str() {
                        // ctrl-a
                        "\u{1}" if ctx.payload().modifiers.contains(Modifiers::CTRL) => {
                            self.value.clear()
                        }
                        _ => self.value.push_str(char),
                    },
                    Key::Unknown => {}
                },
                _ => {}
            }
            if self.value.is_empty() {
                self.update_content(self.placeholder.clone(), ctx, true);
            } else {
                self.update_content(self.value.clone(), ctx, true);
            }
        }
    }
    fn focus_event(&mut self, ctx: &mut EventContext<FocusEvent, Self::Message>) {
        self.focus_handler.clone().handle(self, ctx);
    }
    fn blur_event(&mut self, ctx: &mut EventContext<BlurEvent, Self::Message>) {
        self.blur_handler.clone().handle(self, ctx);
    }
    fn change_event_string(&mut self, ctx: &mut EventContext<ChangeEvent<String>, Self::Message>) {
        self.change_handler.clone().handle(self, ctx);
    }

    fn is_dirty(&self) -> bool {
        self.inner.is_dirty()
    }
    fn clear_cache(&mut self) {
        self.inner.clear_cache();
    }

    fn focusable(&self) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "textinput"
    }

    fn change_event_f32(&mut self, _: &mut EventContext<ChangeEvent<f32>, Self::Message>) {}
}

pub struct WidgetStyle {}
