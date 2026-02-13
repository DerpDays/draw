use graphics::Primitive;
use input::{
    Key,
    KeyboardEvent,
    KeyboardEventKind,
    MouseButton,
    MouseEvent,
    MouseEventKind,
    SpecialKey,
};
use sycamore_reactive::{ReadSignal, Signal};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    ElementId,
    MeasureCtx,
    prelude::EventContext,
    tree::{Widget, builder::ElementBuilder},
};

pub struct InputField {
    enabled: ReadSignal<bool>,
}

impl Widget for InputField {
    fn render(&mut self, _: &Layout, _: &Style) -> Option<Primitive> {
        None
    }
    fn measure(
        &mut self,
        _: ElementId,
        _: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        _: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        known_dimensions.unwrap_or(Size::zero())
    }

    fn debug_label(&self) -> &'static str {
        "InputField"
    }

    fn focusable(&self) -> bool {
        true
    }

    fn default_mouse_event(&mut self, ctx: &mut EventContext<MouseEvent>, _: &Layout) {
        if !ctx.in_capture_phase() {
            if self.enabled.is_alive() && !self.enabled.get_untracked() {
                return;
            }
            if let MouseEventKind::Press {
                button: MouseButton::Left,
                ..
            } = ctx.payload().kind
            {
                ctx.request_kb_focus_capture(ctx.current_node());
            }
        }
    }
    fn default_keyboard_event(&mut self, ctx: &mut EventContext<KeyboardEvent>, _: &Layout) {
        if !ctx.in_capture_phase()
            && let KeyboardEventKind::Press(Key::SpecialKey(SpecialKey::Escape)) =
                ctx.payload().kind
        {
            ctx.request_kb_focus_release();
        }
    }
}

#[inline(always)]
pub fn input_field(enabled: ReadSignal<bool>) -> ElementBuilder<InputField> {
    ElementBuilder::new(InputField { enabled })
}

pub fn color_hex_input_field_blur(value_signal: Signal<String>) {
    let val = value_signal.get_clone_untracked();
    if val.len() != 6 {
        if val.len() == 3 {
            value_signal.set(
                val.chars()
                    .map(|c| format!("{0}{0}", c))
                    .collect::<String>(),
            );
        } else {
            value_signal.set(format!("{:0<6}", val).to_string())
        }
    };
}

pub fn color_hex_input_field(
    value_signal: Signal<String>,
    enabled: ReadSignal<bool>,
) -> ElementBuilder<InputField> {
    input_field(enabled)
        .on_keyboard(move |_, ctx| {
            if !ctx.in_capture_phase() {
                let mut value = value_signal.get_clone_untracked();
                let mut updated = false;
                if let KeyboardEventKind::Press(key) = &ctx.payload().kind {
                    match key {
                        Key::SpecialKey(SpecialKey::Enter) => {
                            ctx.request_kb_focus_release();
                        }
                        Key::SpecialKey(SpecialKey::Backspace) => {
                            value.pop();
                            updated = true;
                        }
                        Key::Character(string) => {
                            for i in string.chars().filter(|x| x.is_ascii_hexdigit()) {
                                if value.len() < 6 {
                                    value.push(i.to_ascii_uppercase());
                                }
                            }
                            updated = true;
                        }
                        _ => {}
                    }
                    if updated {
                        value_signal.set(value)
                    }
                }
            }
        })
        .on_blur(move |_, ctx| {
            if !ctx.in_capture_phase() {
                color_hex_input_field_blur(value_signal)
            }
        })
}

pub fn uint_input_field(
    value_signal: Signal<String>,
    enabled: ReadSignal<bool>,
    min: usize,
    max: usize,
) -> ElementBuilder<InputField> {
    input_field(enabled)
        .on_keyboard(move |_, ctx| {
            if !ctx.in_capture_phase() {
                let mut value = value_signal.get_clone_untracked();
                let mut updated = false;
                if let KeyboardEventKind::Press(key) = &ctx.payload().kind {
                    match key {
                        Key::SpecialKey(SpecialKey::Enter) => {
                            ctx.request_kb_focus_release();
                        }
                        Key::SpecialKey(SpecialKey::Backspace) => {
                            value.pop();
                            updated = true;
                        }
                        Key::Character(string) => {
                            for i in string.chars().filter(|x| x.is_numeric()) {
                                value.push(i);
                            }
                            let parsed = value.parse::<usize>().unwrap_or(min);
                            if parsed > max {
                                value = max.to_string();
                            } else if parsed < min {
                                value = min.to_string();
                            }
                            updated = true;
                        }
                        _ => {}
                    }
                    if updated {
                        value_signal.set(value)
                    }
                }
            }
        })
        .on_blur(move |_, ctx| {
            if !ctx.in_capture_phase() {
                let value = value_signal.get_clone_untracked();
                let parsed = value.parse::<usize>().unwrap_or(min).clamp(min, max);
                let parsed_str = parsed.to_string();
                if parsed_str != value {
                    value_signal.set(parsed_str)
                }
            }
        })
}

pub fn float_input_field(
    value_signal: Signal<String>,
    enabled: ReadSignal<bool>,
    max_precision: usize,
) -> ElementBuilder<InputField> {
    input_field(enabled)
        .on_keyboard(move |_, ctx| {
            if !ctx.in_capture_phase() {
                let mut value = value_signal.get_clone_untracked();
                let mut updated = false;
                if let KeyboardEventKind::Press(key) = &ctx.payload().kind {
                    match key {
                        Key::SpecialKey(SpecialKey::Enter) => {
                            ctx.request_kb_focus_release();
                        }
                        Key::SpecialKey(SpecialKey::Backspace) => {
                            value.pop();
                            updated = true;
                        }
                        Key::Character(string) => {
                            for i in string
                                .chars()
                                .filter(|x| x.is_numeric() || *x == '.' || *x == '-')
                            {
                                if i == '-' {
                                    if value.is_empty() {
                                        value.push(i)
                                    }
                                    continue;
                                }
                                if let Some(idx) = value.find('.') {
                                    if i != '.' && value.len() - idx < max_precision {
                                        value.push(i);
                                    }
                                } else {
                                    value.push(i);
                                }
                            }
                            updated = true;
                        }
                        _ => {}
                    }
                    if updated {
                        value_signal.set(value)
                    }
                }
            }
        })
        .on_blur(move |_, ctx| {
            if !ctx.in_capture_phase() {
                let value = value_signal.get_clone_untracked();
                if value.is_empty() {
                    value_signal.set(0.0.to_string())
                }
            }
        })
}

pub fn single_line_input_field(
    value_signal: Signal<String>,
    enabled: ReadSignal<bool>,
) -> ElementBuilder<InputField> {
    input_field(enabled).on_keyboard(move |_, ctx| {
        if !ctx.in_capture_phase() {
            let mut value = value_signal.get_clone_untracked();
            let mut updated = false;
            if let KeyboardEventKind::Press(key) = &ctx.payload().kind {
                match key {
                    Key::SpecialKey(SpecialKey::Enter) => {
                        ctx.request_kb_focus_release();
                    }
                    Key::SpecialKey(SpecialKey::Backspace) => {
                        value.pop();
                        updated = true;
                    }
                    Key::Character(string) => {
                        for i in string.chars() {
                            value.push(i);
                        }
                        updated = true;
                    }
                    _ => {}
                }
                if updated {
                    value_signal.set(value)
                }
            }
        }
    })
}
